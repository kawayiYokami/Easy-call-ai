#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use futures_util::{future::AbortHandle, future::join_all, future::BoxFuture, StreamExt};
use image::ImageFormat;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rmcp::{schemars, ServiceExt};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, Position, State,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use uuid::Uuid;

macro_rules! eprintln {
    ($($arg:tt)*) => {{
        runtime_log_info(format!($($arg)*));
    }};
}

// ==================== 核心领域模型 ====================
include!("features/core/domain.rs");
include!("features/core/time_semantics.rs");

// ==================== 配置与存储 ====================
include!("features/config/storage_and_stt.rs");
include!("features/config/app_data_layout.rs");
include!("features/chat/message_store/mod.rs");

// ==================== 对话核心 ====================
include!("features/chat/message_semantics.rs");
include!("features/chat/conversation.rs");
include!("features/chat/prompt_manager.rs");
include!("features/chat/conversation_prompt_service.rs");
include!("features/chat/conversation_service/mod.rs");
include!("features/chat/model_runtime.rs");
include!("features/chat/scheduler.rs");
include!("features/remote_im/channel_store.rs");
include!("features/remote_im/onebot_v11_ws.rs");
include!("features/remote_im/dingtalk_stream.rs");
include!("features/remote_im/weixin_oc.rs");
include!("features/remote_im.rs");
include!("features/remote_im_adapters.rs");

// ==================== 系统窗口与命令 ====================
include!("features/system/windowing.rs");
include!("features/system/record_hotkey_probe.rs");
include!("features/system/windows_job.rs");
include!("features/system/sandbox.rs");
include!("features/system/tools.rs");
include!("features/system/updater.rs");

// ==================== 记忆匹配 ====================
include!("features/memory/store.rs");
include!("features/memory/matcher.rs");
include!("features/memory/providers.rs");

// ==================== MCP ====================
include!("features/mcp.rs");
include!("features/skill.rs");
include!("features/task.rs");
include!("features/delegate.rs");
mod relationship_state {
    use crate::{AppState, State};
    #[cfg(test)]
    use crate::Uuid;
    include!("features/relationship_state/types.rs");
    include!("features/relationship_state/state.rs");
    include!("features/relationship_state/analyzer.rs");
    include!("features/relationship_state/reducer.rs");
    include!("features/relationship_state/block.rs");
    include!("features/relationship_state/mod.rs");
}
mod relationship_state_llm_analyzer_runtime {
    use crate::*;
    include!("features/relationship_state/llm_analyzer_runtime.rs");
}

include!("features/system/commands.rs");

fn should_enable_devtools() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    matches!(
        std::env::var("EASYCALL_DEVTOOLS")
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

fn install_tauri_async_runtime() -> Result<tokio::runtime::Runtime, String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("创建 Tauri 异步运行时失败: {err}"))?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // 必须在 Tauri Builder 初始化前设置，否则会命中 tauri::async_runtime::set 的已初始化 panic。
        tauri::async_runtime::set(runtime.handle().clone());
    }))
    .map_err(|panic_payload| {
        let panic_text = panic_payload
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| panic_payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "未知 panic".to_string());
        format!("设置 Tauri 异步运行时失败: {panic_text}")
    })?;
    Ok(runtime)
}

// Remote IM 命令包装
#[tauri::command]
async fn remote_im_get_channel_status(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<ChannelConnectionStatus, String> {
    let config = state_read_config_cached(&state).map_err(|e| format!("{e:?}"))?;
    if let Some(channel) = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
    {
        return match channel.platform {
            RemoteImPlatform::OnebotV11 => get_channel_connection_status(channel_id).await,
            RemoteImPlatform::Dingtalk => Ok(dingtalk_stream_manager()
                .get_channel_status(&channel.id)
                .await),
            RemoteImPlatform::Feishu => Ok(ChannelConnectionStatus {
                channel_id: channel.id.clone(),
                connected: false,
                peer_addr: None,
                connected_at: None,
                listen_addr: String::new(),
                status_text: None,
                last_error: None,
                account_id: None,
                base_url: None,
                login_session_key: None,
                qrcode_url: None,
            }),
            RemoteImPlatform::WeixinOc => Ok(weixin_oc_manager().build_status(&channel.id).await),
        };
    }
    get_channel_connection_status(channel_id).await
}

#[tauri::command]
async fn remote_im_get_channel_logs(channel_id: String) -> Result<Vec<ChannelLogEntry>, String> {
    get_channel_logs(channel_id).await
}

#[tauri::command]
async fn remote_im_get_contact_logs(
    input: RemoteImContactLogsInput,
    state: State<'_, AppState>,
) -> Result<Vec<ChannelLogEntry>, String> {
    let (channel_id, contact_marker) =
        remote_im_resolve_contact_log_query(state.inner(), &input.contact_id)?;
    let logs = get_channel_logs(channel_id).await?;
    Ok(remote_im_filter_channel_logs_for_contact(logs, &contact_marker))
}

#[tauri::command]
async fn remote_im_restart_channel(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<ChannelConnectionStatus, String> {
    eprintln!("[远程IM] 重启渠道: {}", channel_id);
    onebot_v11_ws_manager()
        .add_log(&channel_id, "info", "[远程IM] 收到渠道重启请求")
        .await;
    let config = state_read_config_cached(&state).map_err(|e| format!("{e:?}"))?;
    let channel = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
        .ok_or_else(|| format!("渠道 {} 未找到", channel_id))?
        .clone();
    onebot_v11_ws_manager()
        .add_log(
            &channel_id,
            "info",
            &format!(
                "[远程IM] 当前渠道配置: enabled={}, platform={:?}",
                channel.enabled, channel.platform
            ),
        )
        .await;

    let effective_channel = remote_im_channel_with_effective_credentials(state.inner(), &channel)?;
    let manager = onebot_v11_ws_manager();
    manager
        .reconcile_channel_runtime(&effective_channel)
        .await
        .map_err(|err| format!("重启渠道失败: {}", err))?;
    eprintln!(
        "[远程IM] 渠道 {} 已按配置收敛: enabled={}, platform={:?}",
        channel_id, channel.enabled, channel.platform
    );

    if channel.enabled && channel.platform == RemoteImPlatform::OnebotV11 {
        manager
            .start_event_consumer(channel_id.clone(), state.inner().clone())
            .await
            .map_err(|err| format!("重启事件消费器失败: {}", err))?;
    } else if channel.enabled && channel.platform == RemoteImPlatform::Dingtalk {
        let state_clone = state.inner().clone();
        let manager = dingtalk_stream_manager();
        let channel_clone = remote_im_channel_with_effective_credentials(&state_clone, &channel)?;
        tauri::async_runtime::spawn(async move {
            if let Err(err) = manager
                .reconcile_channel_runtime(&channel_clone, state_clone)
                .await
            {
                eprintln!(
                    "[远程IM] 钉钉渠道收敛失败: channel_id={}, platform={:?}, error={}",
                    channel_clone.id, channel_clone.platform, err
                );
            }
        });
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        let state_clone = state.inner().clone();
        weixin_oc_manager()
            .reconcile_channel_runtime(&effective_channel, state_clone)
            .await?;
    }

    if channel.platform == RemoteImPlatform::Dingtalk {
        Ok(dingtalk_stream_manager()
            .get_channel_status(&channel_id)
            .await)
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        Ok(weixin_oc_manager().build_status(&channel_id).await)
    } else {
        Ok(manager.get_connection_status(&channel_id).await)
    }
}

#[tauri::command]
fn frontend_ready_start_remote_im_services(app: AppHandle) -> Result<bool, String> {
    static BACKGROUND_SERVICES_START_REQUESTED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    if BACKGROUND_SERVICES_START_REQUESTED
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_err()
    {
        eprintln!("[启动] 前端就绪后启动后台服务：已请求过，跳过重复启动");
        return Ok(false);
    }

    eprintln!("[启动] 前端已就绪，开始异步启动后台服务");
    let startup_state = app.state::<AppState>().inner().clone();
    tauri::async_runtime::spawn(async move {
        start_background_services_after_frontend_ready(app, startup_state).await;
    });
    Ok(true)
}

async fn start_background_services_after_frontend_ready(
    app_handle: AppHandle,
    startup_state: AppState,
) {
    let ide_context_runtime = app_handle.state::<IdeContextRuntime>().inner().clone();
    start_task_scheduler(startup_state.clone());
    tauri::async_runtime::spawn({
        let probe_state = startup_state.clone();
        async move {
            probe_release_source_once(&probe_state).await;
        }
    });
    match load_workspace(&startup_state).await {
        Ok(result) => log_workspace_load_result("[工作区加载]", &result),
        Err(err) => eprintln!("[工作区加载] 状态=失败，error={err}"),
    }
    start_remote_im_services_after_frontend_ready(app_handle).await;
    start_ide_context_bridge_server(startup_state, ide_context_runtime);
}

async fn start_remote_im_services_after_frontend_ready(app_handle: AppHandle) {
    let app_state = app_handle.state::<AppState>();
    let config = match state_read_config_cached(&app_state) {
        Ok(mut config) => {
            if let Err(err) =
                remote_im_migrate_channel_private_states(app_state.inner(), &mut config)
                    .and_then(|changed| {
                        if changed {
                            state_write_config_cached(app_state.inner(), &config)?;
                        }
                        Ok(())
                    })
            {
                eprintln!("[启动] 远程 IM 私有状态迁移失败，继续按现有配置启动: {}", err);
            }
            config
        }
        Err(err) => {
            eprintln!("[启动] 前端就绪后读取远程 IM 配置失败，跳过启动: {:?}", err);
            return;
        }
    };
    let event_state = app_handle.state::<AppState>().inner().clone();

    let napcat_channels: Vec<_> = config
        .remote_im_channels
        .iter()
        .filter(|ch| ch.enabled && ch.platform == RemoteImPlatform::OnebotV11)
        .cloned()
        .collect();
    for channel in &napcat_channels {
        let effective_channel =
            match remote_im_channel_with_effective_credentials(&event_state, channel) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!(
                        "[启动] OneBot v11 渠道私有状态读取失败: channel_id={}, error={}",
                        channel.id, err
                    );
                    continue;
                }
            };
        if let Err(err) = onebot_v11_ws_server_start(effective_channel, app_handle.clone()) {
            eprintln!(
                "[启动] 前端就绪后启动 OneBot v11 WS 服务失败: channel_id={}, error={}",
                channel.id, err
            );
        }
    }
    for channel in napcat_channels {
        let channel_id = channel.id.clone();
        let state_clone = event_state.clone();
        if let Err(err) = onebot_v11_ws_manager()
            .start_event_consumer(channel_id.clone(), state_clone)
            .await
        {
            eprintln!(
                "[启动] 前端就绪后启动 OneBot v11 事件消费器失败: channel_id={}, error={}",
                channel_id, err
            );
        }
    }

    let dingtalk_channels: Vec<_> = config
        .remote_im_channels
        .iter()
        .filter(|ch| ch.enabled && ch.platform == RemoteImPlatform::Dingtalk)
        .cloned()
        .collect();
    for channel in dingtalk_channels {
        let state_clone = event_state.clone();
        let manager = dingtalk_stream_manager();
        tauri::async_runtime::spawn(async move {
            let channel_id = channel.id.clone();
            let effective_channel =
                match remote_im_channel_with_effective_credentials(&state_clone, &channel) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!(
                            "[启动] 钉钉渠道私有状态读取失败: channel_id={}, error={}",
                            channel_id, err
                        );
                        return;
                    }
                };
            if let Err(err) = manager.start_channel(effective_channel, state_clone).await {
                eprintln!(
                    "[启动] 前端就绪后启动钉钉 Stream 渠道失败: channel_id={}, error={}",
                    channel_id, err
                );
            }
        });
    }

    let weixin_channels: Vec<_> = config
        .remote_im_channels
        .iter()
        .filter(|ch| ch.enabled && ch.platform == RemoteImPlatform::WeixinOc)
        .cloned()
        .collect();
    for channel in weixin_channels {
        let state_clone = event_state.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) = weixin_oc_manager()
                .reconcile_channel_runtime(&channel, state_clone)
                .await
            {
                eprintln!(
                    "[启动] 前端就绪后启动个人微信渠道失败: channel_id={}, error={}",
                    channel.id, err
                );
            }
        });
    }
}

const APP_SHUTDOWN_STATE_IDLE: u8 = 0;
const APP_SHUTDOWN_STATE_RUNNING: u8 = 1;
const APP_SHUTDOWN_STATE_DONE: u8 = 2;

static APP_SHUTDOWN_STATE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(APP_SHUTDOWN_STATE_IDLE);

async fn graceful_shutdown_background_services(app: &AppHandle) {
    let shutdown_started = APP_SHUTDOWN_STATE.compare_exchange(
        APP_SHUTDOWN_STATE_IDLE,
        APP_SHUTDOWN_STATE_RUNNING,
        std::sync::atomic::Ordering::SeqCst,
        std::sync::atomic::Ordering::SeqCst,
    );
    if shutdown_started.is_err() {
        while APP_SHUTDOWN_STATE.load(std::sync::atomic::Ordering::SeqCst)
            == APP_SHUTDOWN_STATE_RUNNING
        {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        eprintln!("[退出] 已等待进行中的后台服务优雅停机完成");
        return;
    }

    let state = app.state::<AppState>().inner().clone();
    let started_at = std::time::Instant::now();
    eprintln!("[退出] 开始优雅停机后台服务");

    let config = match state_read_config_cached(&state) {
        Ok(config) => Some(config),
        Err(err) => {
            eprintln!("[退出] 读取配置失败，继续按兜底方式停机: {err}");
            None
        }
    };

    if let Some(config) = config.as_ref() {
        let mut shutdown_futures = Vec::<BoxFuture<'static, ()>>::new();
        for channel in &config.remote_im_channels {
            let channel_id = channel.id.trim().to_string();
            if channel_id.is_empty() {
                continue;
            }
            match channel.platform {
                RemoteImPlatform::OnebotV11 => shutdown_futures.push(Box::pin(async move {
                    let stop_started = std::time::Instant::now();
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        onebot_v11_ws_manager().stop_channel(&channel_id),
                    )
                    .await
                    {
                        Ok(Ok(())) => eprintln!(
                            "[退出] OneBot 渠道已停止: channel_id={}，duration_ms={}",
                            channel_id,
                            stop_started.elapsed().as_millis()
                        ),
                        Ok(Err(err)) => eprintln!(
                            "[退出] OneBot 渠道停止失败: channel_id={}，duration_ms={}，error={}",
                            channel_id,
                            stop_started.elapsed().as_millis(),
                            err
                        ),
                        Err(_) => eprintln!(
                            "[退出] OneBot 渠道停止超时: channel_id={}，timeout_ms=10000",
                            channel_id
                        ),
                    }
                })),
                RemoteImPlatform::Dingtalk => shutdown_futures.push(Box::pin(async move {
                    let stop_started = std::time::Instant::now();
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        dingtalk_stream_manager().stop_channel(&channel_id),
                    )
                    .await
                    {
                        Ok(()) => eprintln!(
                            "[退出] 钉钉渠道已停止: channel_id={}，duration_ms={}",
                            channel_id,
                            stop_started.elapsed().as_millis()
                        ),
                        Err(_) => eprintln!(
                            "[退出] 钉钉渠道停止超时: channel_id={}，timeout_ms=10000",
                            channel_id
                        ),
                    }
                })),
                RemoteImPlatform::WeixinOc => shutdown_futures.push(Box::pin(async move {
                    let stop_started = std::time::Instant::now();
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        weixin_oc_manager().stop_channel(&channel_id),
                    )
                    .await
                    {
                        Ok(()) => eprintln!(
                            "[退出] 个人微信渠道已停止: channel_id={}，duration_ms={}",
                            channel_id,
                            stop_started.elapsed().as_millis()
                        ),
                        Err(_) => eprintln!(
                            "[退出] 个人微信渠道停止超时: channel_id={}，timeout_ms=10000",
                            channel_id
                        ),
                    }
                })),
                RemoteImPlatform::Feishu => {}
            }
        }
        join_all(shutdown_futures).await;
    }

    // 兜底广播关闭，确保仍持有 shutdown receiver 的旧渠道尽快退出 accept 循环。
    onebot_v11_ws_manager().shutdown().await;

    match load_workspace_mcp_servers(&state) {
        Ok(servers) => {
            let shutdown_futures = servers
                .into_iter()
                .filter_map(|server| {
                    let server_id = server.id.trim().to_string();
                    if server_id.is_empty() {
                        return None;
                    }
                    Some(Box::pin(async move {
                        let disconnect_started = std::time::Instant::now();
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            mcp_disconnect_cached_client(&server_id),
                        )
                        .await
                        {
                            Ok(()) => eprintln!(
                                "[退出] MCP 已断开: server_id={}，duration_ms={}",
                                server_id,
                                disconnect_started.elapsed().as_millis()
                            ),
                            Err(_) => eprintln!(
                                "[退出] MCP 断开超时: server_id={}，timeout_ms=10000",
                                server_id
                            ),
                        }
                    }) as BoxFuture<'static, ()>)
                })
                .collect::<Vec<BoxFuture<'static, ()>>>();
            join_all(shutdown_futures).await;
        }
        Err(err) => {
            eprintln!("[退出] 读取 MCP 工作区失败，跳过逐个断开: {err}");
        }
    }

    eprintln!(
        "[退出] 后台服务优雅停机完成: duration_ms={}",
        started_at.elapsed().as_millis()
    );
    APP_SHUTDOWN_STATE.store(APP_SHUTDOWN_STATE_DONE, std::sync::atomic::Ordering::SeqCst);
}

fn graceful_shutdown_background_services_blocking(app: &AppHandle) {
    tauri::async_runtime::block_on(graceful_shutdown_background_services(app));
}

fn graceful_exit_app(app: &AppHandle, code: i32) {
    graceful_shutdown_background_services_blocking(app);
    app.exit(code);
}

fn graceful_restart_app(app: &AppHandle) {
    graceful_shutdown_background_services_blocking(app);
    app.restart();
}

fn main() {
    if std::env::args().any(|arg| arg == MCP_SCREENSHOT_SERVER_FLAG) {
        if let Err(err) = run_desktop_screenshot_mcp_server() {
            eprintln!("{err}");
        }
        return;
    }
    if std::env::args().any(|arg| arg == MCP_OPERATE_SERVER_FLAG) {
        if let Err(err) = run_operate_mcp_server() {
            eprintln!("{err}");
        }
        return;
    }
    match maybe_run_portable_update_helper_from_args() {
        Ok(true) => return,
        Ok(false) => {}
        Err(err) => {
            eprintln!("[自动更新] 便携版 helper 启动失败: {err}");
            return;
        }
    }

    let _tauri_runtime = match install_tauri_async_runtime() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("[启动] 初始化 Tokio 运行时失败: {err}");
            return;
        }
    };

    let state = match AppState::new() {
        Ok(state) => state,
        Err(err) => {
            eprintln!("[启动] 初始化应用状态失败: {err}");
            return;
        }
    };
    init_last_panic_snapshot_slot(state.last_panic_snapshot.clone());
    {
        let panic_slot = state.last_panic_snapshot.clone();
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic payload".to_string()
            };
            let location = info
                .location()
                .map(|loc| format!("{}:{}", loc.file(), loc.line()))
                .unwrap_or_else(|| "unknown location".to_string());
            let thread_name = std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_string();
            let snapshot = format!(
                "{} thread={} payload={}",
                location.trim(),
                thread_name.trim(),
                payload.trim()
            );
            if let Ok(mut slot) = panic_slot.lock() {
                *slot = Some(snapshot);
            }
            previous_hook(info);
        }));
    }

    let builder = tauri::Builder::default();
    let builder = if cfg!(debug_assertions) {
        eprintln!("[单实例] 当前为调试构建，跳过单实例拦截");
        builder
    } else {
        builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let chat_is_visible = app
                .get_webview_window("chat")
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false);
            let target = if chat_is_visible {
                "chat"
            } else {
                match state_read_config_cached(app.state::<AppState>().inner()) {
                    Ok(mut config) => {
                        normalize_app_config(&mut config);
                        startup_window_label_for_config(&config)
                    }
                    Err(err) => {
                        eprintln!("[单实例] 读取启动窗口配置失败: {err}");
                        "quick-setup"
                    }
                }
            };
            if let Err(err) = show_window(app, target) {
                eprintln!(
                    "[单实例] 激活已有实例失败: target={}, error={}",
                    target, err
                );
            } else {
                eprintln!("[单实例] 已拦截重复启动并激活现有实例: target={}", target);
            }
        }))
    };

    let ide_context_runtime = IdeContextRuntime::new();

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    handle_global_shortcut_probe(app, shortcut, event.state());
                })
                .build(),
        )
        .manage(state)
        .manage(ide_context_runtime)
        .setup(|app| {
            let app_handle = app.handle().clone();
            match app_handle.state::<AppState>().app_handle.lock() {
                Ok(mut handle_slot) => {
                    *handle_slot = Some(app_handle.clone());
                }
                Err(e) => {
                    eprintln!("[启动] 写入应用句柄槽位失败: {e}");
                }
            }
            if let Err(err) = register_default_hotkey(&app_handle) {
                eprintln!("[启动] 注册默认快捷键失败: {err}");
            }
            if let Err(err) = start_app_data_persist_worker(app_handle.state::<AppState>().inner()) {
                eprintln!("[启动] 启动后台持久化服务失败: {err}");
            }
            if let Err(err) = start_conversation_persist_worker(app_handle.state::<AppState>().inner()) {
                eprintln!("[启动] 启动会话后台持久化服务失败: {err}");
            }
            if let Err(err) = start_record_hotkey_probe(
                app_handle.clone(),
                app_handle.state::<AppState>().config_path.clone(),
            ) {
                eprintln!("[启动] 启动录音热键探针失败: {err}");
            }
            if let Err(err) = build_tray(&app_handle) {
                eprintln!("[启动] 构建托盘失败: {err}");
            }
            let app_state = app_handle.state::<AppState>();
            match state_read_config_cached(app_state.inner()) {
                Ok(mut config) => {
                    if run_startup_self_checks(&mut config) {
                        if let Err(err) = state_write_config_cached(app_state.inner(), &config) {
                            eprintln!("[启动自检] 写入修复后的配置失败: {err}");
                        } else {
                            eprintln!("[启动自检] 完成，已将副手部门模型从默认人格修正为副手");
                        }
                    }
                }
                Err(err) => {
                    eprintln!("[启动自检] 读取配置失败: {err}");
                }
            }
            if let Err(err) = memory_store_open(&app_state.data_path) {
                eprintln!("[启动] 初始化记忆存储失败: {err}");
            }
            if let Err(err) = task_store_open(&app_state.data_path) {
                eprintln!("[启动] 初始化任务存储失败: {err}");
            }
            if let Err(err) = delegate_store_open(&app_state.data_path) {
                eprintln!("[启动] 初始化委托存储失败：{err}");
            }
            let _ = sync_default_tray_icon(&app_handle);
            attach_window_layout_persistence(&app_handle);
            hide_on_close(&app_handle);
            let startup_window_label = match state_read_config_cached(app_state.inner()) {
                Ok(mut config) => {
                    normalize_app_config(&mut config);
                    startup_window_label_for_config(&config)
                }
                Err(err) => {
                    eprintln!("[启动] 读取启动窗口配置失败: {err}");
                    "quick-setup"
                }
            };
            if let Err(err) = show_window(&app_handle, startup_window_label) {
                eprintln!("[启动] 显示启动窗口失败: target={startup_window_label}, error={err}");
                if let Some(window) = app_handle.get_webview_window(startup_window_label) {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            if should_enable_devtools() {
                eprintln!("[启动] 检测到 devtools 开关已开启，但当前构建未启用 open_devtools API，跳过打开 devtools");
            }
            eprintln!("[启动] 任务调度、工作区加载与远程 IM 服务延后到前端 mounted ready 后启动");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            show_chat_window,
            show_archives_window,
            show_quick_setup_window,
            complete_quick_setup_and_open_chat,
            detach_current_conversation_to_window,
            get_detached_chat_window_info,
            focus_detached_chat_window_by_conversation,
            set_chat_window_active,
            check_github_update,
            start_github_update,
            apply_prepared_github_update,
            fetch_project_changelog_markdown,
            get_app_version,
            get_project_repository_url,
            set_github_update_method,
            set_ui_language,
            load_config,
            check_message_store_migration,
            run_message_store_migration,
            load_app_bootstrap_snapshot,
            list_system_fonts,
            set_webview_zoom_percent,
            set_chat_side_panels_window_expanded,
            save_config,
            load_agents,
            save_agents,
            get_agent_private_memory_count,
            set_agent_private_memory_enabled,
            export_agent_private_memories,
            disable_agent_private_memory,
            import_agent_memories,
            load_chat_settings,
            save_chat_settings,
            patch_chat_settings,
            save_agent_avatar,
            clear_agent_avatar,
            read_avatar_data_url,
            read_chat_image_data_url,
            sync_tray_icon,
            save_conversation_api_settings,
            patch_conversation_api_settings,
            set_department_primary_api_config,
            get_chat_snapshot,
            list_unarchived_conversations,
            set_active_unarchived_conversation,
            switch_active_conversation_snapshot,
            get_foreground_conversation_light_snapshot,
            mark_conversation_read,
            set_conversation_plan_mode,
            create_unarchived_conversation,
            branch_unarchived_conversation_from_selection,
            forward_unarchived_conversation_selection,
            rename_unarchived_conversation,
            toggle_unarchived_conversation_pin,
            get_unarchived_conversation_messages,
            get_unarchived_conversation_block_page,
            get_unarchived_conversation_recent_block_messages,
            get_unarchived_conversation_recent_messages,
            get_unarchived_conversation_message_by_id,
            list_delegate_conversations,
            list_conversation_delegate_statuses,
            abort_delegate_conversation,
            get_delegate_conversation_messages,
            delete_delegate_conversation,
            delete_unarchived_conversation,
            get_active_conversation_messages,
            get_active_conversation_messages_before,
            get_active_conversation_messages_after,
            list_tool_review_batches,
            list_tool_review_reports,
            delete_tool_review_report,
            list_tool_review_commit_options,
            get_tool_review_item_detail,
            run_tool_review_for_call,
            run_tool_review_for_batch,
            submit_tool_review_code,
            request_conversation_messages_after_async,
            rewind_conversation_from_message,
            get_prompt_preview,
            get_system_prompt_preview,
            list_archives,
            list_memories,
            delete_memory,
            export_memories,
            preview_export_memories,
            export_memories_to_file,
            export_memories_to_path,
            import_memories,
            preview_import_angel_memories,
            import_angel_memories,
            write_utf8_text_file_to_path,
            write_base64_file_to_path,
            search_memories_mixed,
            sync_memory_embedding_provider,
            test_memory_embedding_provider,
            test_memory_rerank_provider,
            get_memory_provider_bindings,
            get_memory_embedding_sync_progress,
            save_memory_embedding_binding,
            save_memory_rerank_binding,
            memory_rebuild_indexes,
            memory_health_check,
            memory_backup_db,
            memory_restore_db,
            get_archive_messages,
            get_archive_block_page,
            get_archive_summary,
            delete_archive,
            export_archive_to_file,
            import_archives_from_json,
            open_external_url,
            open_workspace_file,
            read_plan_file_content,
            confirm_plan_and_continue,
            send_chat_message,
            send_user_mention_message,
            submit_user_async_delegate,
            bind_active_chat_view_stream,
             stop_chat_message,
             get_chat_queue_snapshot,
             recall_chat_queue_event,
             mark_chat_queue_event_guided,
             interrupt_conversation_runtime,
             get_main_session_state_snapshot,
             get_conversation_runtime_snapshot,
            read_local_binary_file,
            queue_local_file_attachment,
            queue_inline_file_attachment,
            stt_transcribe,
            force_archive_current,
            force_compact_current,
            preview_force_archive_current,
            preview_force_compact_current,
            refresh_models,
            quick_genai_chat,
            resolve_model_adapter_kind,
            fetch_model_metadata,
            export_config_migration_package,
            preview_import_config_migration_package,
            apply_import_config_migration_package,
            codex_get_auth_status,
            codex_get_rate_limits,
            codex_start_oauth_login,
            codex_logout,
            check_tools_status,
            list_tool_catalog,
            list_department_permission_catalog,
            get_image_text_cache_stats,
            clear_image_text_cache,
            list_recent_llm_round_logs,
            clear_recent_llm_round_logs,
            list_recent_runtime_logs,
            clear_recent_runtime_logs,
            append_runtime_log_probe,
            remote_im_list_channels,
            remote_im_list_contacts,
            remote_im_update_contact_allow_send,
            remote_im_update_contact_allow_send_files,
            remote_im_update_contact_allow_receive,
            remote_im_update_contact_activation,
            remote_im_update_contact_remark,
            remote_im_update_contact_route_mode,
            remote_im_update_contact_department_binding,
            remote_im_update_contact_processing_mode,
            remote_im_update_contact_workspace,
            remote_im_delete_contact,
            remote_im_clear_contact_conversation,
            remote_im_enqueue_message,
            remote_im_list_contact_conversations,
            remote_im_get_contact_conversation_messages,
            remote_im_get_contact_conversation_block_page,
            remote_im_get_channel_status,
            remote_im_get_channel_logs,
            remote_im_get_contact_logs,
            remote_im_restart_channel,
            frontend_ready_start_remote_im_services,
            remote_im_weixin_oc_start_login,
            remote_im_weixin_oc_get_login_status,
            remote_im_weixin_oc_sync_contacts,
            remote_im_weixin_oc_logout,
            desktop_screenshot,
            xcap,
            demo_send_native_notification,
            get_host_runtime_prerequisites,
            install_host_runtime_prerequisite,
            terminal_self_check,
            list_terminal_shell_candidates,
            open_chat_shell_workspace_dir,
            reset_chat_shell_workspace,
            migrate_shell_workspace_directory,
            get_default_chat_shell_workspace_path,
            get_chat_shell_workspace,
            update_chat_shell_workspace_layout,
            upsert_ide_context_snapshot,
            query_ide_context_references,
            task_list_tasks,
            task_get_task,
            task_create_task,
            task_dispatch_task_now,
            task_update_task,
            task_complete_task,
            task_delete_task,
            task_list_run_logs,
            resolve_terminal_approval,
            open_file_reader_window_command,
            read_file_reader_file,
            list_file_reader_directory,
            open_file_reader_directory_shell,
            open_local_file_directory,
            open_file_with_default_program,
            mcp_list_servers,
            mcp_validate_definition,
            mcp_save_server,
            mcp_remove_server,
            mcp_deploy_server,
            mcp_undeploy_server,
            mcp_list_server_tools,
            mcp_list_server_tools_cached,
            mcp_set_tool_enabled,
            commands::mcp_refresh_mcp_and_skills,
            commands::mcp_list_skills,
            mcp_open_workspace_dir,
            commands::skill_open_workspace_dir,
            relationship_state::refresh_relationship_rules,
            relationship_state::get_relationship_panel_snapshot,
            relationship_state::preview_relationship_block,
            relationship_state::reset_relationship_state,
            relationship_state::simulate_relationship_event
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("[启动] 运行 Tauri 应用失败: {err}");
        });
}

#[cfg(test)]
mod tests {
    include!("features/tests.rs");
}
