fn build_remote_im_enqueue_input(
    channel_id: &str,
    sender_name: String,
    sender_id: String,
    im_name: String,
    activate_assistant: Option<bool>,
    remote_contact_type: String,
    remote_contact_id: String,
    remote_contact_name: Option<String>,
    platform_message_id: Option<String>,
    final_text: String,
    images: Vec<BinaryPart>,
    attachments: Vec<AttachmentMetaInput>,
) -> RemoteImEnqueueInput {
    RemoteImEnqueueInput {
        channel_id: channel_id.to_string(),
        platform: RemoteImPlatform::OnebotV11,
        im_name,
        remote_contact_type,
        remote_contact_id,
        remote_contact_name,
        sender_id,
        sender_name,
        sender_avatar_url: None,
        platform_message_id,
        dingtalk_session_webhook: None,
        dingtalk_session_webhook_expired_time: None,
        activate_assistant,
        session: SessionSelector {
            api_config_id: None,
            department_id: None,
            agent_id: String::new(),
            conversation_id: None,
        },
        payload: ChatInputPayload {
            text: Some(final_text),
            display_text: None,
            images: if images.is_empty() { None } else { Some(images) },
            audios: None,
            attachments: if attachments.is_empty() { None } else { Some(attachments) },
            model: None,
            extra_text_blocks: None,
            mentions: None,
            provider_meta: None,
        },
    }
}

/// 解析 OneBot v11 message 事件并入队
async fn parse_and_enqueue_onebot_event(
    channel_id: &str,
    event: &Value,
    state: &AppState,
    manager: &OnebotV11WsManager,
) -> Result<RemoteImEnqueueResult, String> {
    eprintln!(
        "[远程IM][OneBot v11 事件][trace] channel_id={}, message_type={}, user_id={}, message_id={}",
        channel_id,
        event
            .get("message_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
        event
            .get("user_id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        event
            .get("message_id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    let user_id_for_media = onebot_read_u64_like(event, "user_id").unwrap_or(0);
    let group_id = onebot_read_u64_like(event, "group_id");
    let sender_id = onebot_read_id_as_string(event, "user_id").unwrap_or_else(|| user_id_for_media.to_string());
    let sender_name = resolve_sender_name(event);
    let message_field = event.get("message");
    let (mut text, mut media_refs, embedded_refs) = extract_message_content(event);
    if !embedded_refs.is_empty() {
        let (embedded_text, nested_media_refs) =
            onebot_expand_embedded_content(manager, channel_id, &embedded_refs).await;
        if !embedded_text.is_empty() {
            text = if text.trim().is_empty() {
                embedded_text
            } else {
                format!("{}\n{}", text.trim(), embedded_text)
            };
        }
        media_refs.extend(nested_media_refs);
    }
    let (images, attachments, notices) =
        onebot_resolve_inbound_media(manager, channel_id, group_id, Some(user_id_for_media), state, &media_refs)
            .await;
    if !notices.is_empty() {
        let notice_text = notices.join("\n");
        text = if text.trim().is_empty() {
            notice_text
        } else {
            format!("{}\n{}", text.trim(), notice_text)
        };
    }
    if text.trim().is_empty() && images.is_empty() && attachments.is_empty() {
        return Err(format!(
            "消息内容为空，跳过 (message_type={}, user_id={}, message_field_type={})",
            event
                .get("message_type")
                .and_then(|v| v.as_str())
                .unwrap_or("private"),
            sender_id,
            message_field_kind(message_field)
        ));
    }

    let (remote_contact_type, remote_contact_id, mut remote_contact_name) =
        resolve_contact_info(event, manager, channel_id).await?;
    if remote_contact_type != "group" {
        remote_contact_name = Some(sender_name.clone());
    }
    let platform_message_id = onebot_read_id_as_string(event, "message_id");
    let channel_config = read_channel_config(state, channel_id)?;
    let im_name = channel_config
        .as_ref()
        .map(|ch| ch.name.clone())
        .unwrap_or_else(|| "OneBot v11".to_string());
    let activate_assistant = channel_config.as_ref().map(|ch| ch.activate_assistant);
    let input = build_remote_im_enqueue_input(
        channel_id,
        sender_name,
        sender_id,
        im_name,
        activate_assistant,
        remote_contact_type,
        remote_contact_id,
        remote_contact_name,
        platform_message_id,
        text,
        images,
        attachments,
    );
    remote_im_enqueue_message_internal(input, state)
}

async fn napcat_run_event_consumer_loop(
    manager: OnebotV11WsManager,
    channel_id: String,
    state: AppState,
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        // 等待连接建立后才能订阅事件
        let (mut event_rx, cancel_token) = loop {
            if *stop_rx.borrow() {
                manager.add_log(&channel_id, "info", "事件消费器收到停止信号").await;
                return;
            }
            if let Some(rx) = manager.subscribe_events(&channel_id).await {
                if let Some(token) = manager.get_channel_cancel_token(&channel_id).await {
                    break (rx, token);
                }
            }
            // 连接尚未建立或渠道已停止，按节流间隔重试
            tokio::select! {
                changed = stop_rx.changed() => {
                    match changed {
                        Ok(()) => {
                            if *stop_rx.borrow() {
                                manager.add_log(&channel_id, "info", "事件消费器收到停止信号").await;
                                return;
                            }
                        }
                        Err(_) => return,
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(NAPCAT_RECONNECT_INTERVAL_SECS)) => {}
            }
        };

        eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 开始消费事件", channel_id);
        manager.add_log(&channel_id, "info", "事件消费器已启动").await;

        loop {
            tokio::select! {
                event_result = event_rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            // 只处理 message 事件
                            if event.get("post_type").and_then(|v| v.as_str()) != Some("message") {
                                continue;
                            }

                            match parse_and_enqueue_onebot_event(&channel_id, &event, &state, &manager).await {
                                Ok(result) => {
                                    eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 入队成功: 事件ID={}", channel_id, result.event_id);
                                }
                                Err(err) if err.contains("跳过") => {
                                    // 正常跳过（联系人未开启、内容为空等），仅输出调试日志，不写渠道日志
                                    eprintln!("[远程IM][OneBot v11 事件] 渠道 {} {}", channel_id, err);
                                }
                                Err(err) => {
                                    eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 入队失败: {}", channel_id, err);
                                    manager.add_log(&channel_id, "warn", &format!("消息入队失败: {}", err)).await;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 落后 {} 条事件", channel_id, n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 事件通道关闭", channel_id);
                            break;
                        }
                    }
                }
                changed = stop_rx.changed() => {
                    match changed {
                        Ok(()) => {
                            if *stop_rx.borrow() {
                                eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 收到消费器停止信号", channel_id);
                                manager.add_log(&channel_id, "info", "事件消费器已停止").await;
                                return;
                            }
                        }
                        Err(_) => return,
                    }
                }
                _ = cancel_token.cancelled() => {
                    eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 收到取消信号，停止事件消费", channel_id);
                    manager.add_log(&channel_id, "info", "事件消费器已停止").await;
                    return; // 渠道已停止，完全退出消费循环
                }
            }
        }

        // 事件通道关闭（客户端断开），按节流间隔等待重连
        eprintln!("[远程IM][OneBot v11 事件] 渠道 {} 等待重新连接...", channel_id);
        tokio::select! {
            changed = stop_rx.changed() => {
                match changed {
                    Ok(()) => {
                        if *stop_rx.borrow() {
                            manager.add_log(&channel_id, "info", "事件消费器已停止").await;
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(NAPCAT_RECONNECT_INTERVAL_SECS)) => {}
        }
    }
}

impl OnebotV11WsManager {
    async fn stop_event_consumer_inner(&self, channel_id: &str) -> Result<(), String> {
        let stop_sender = {
            self.event_consumer_stop_senders
                .write()
                .await
                .remove(channel_id)
        };
        if let Some(tx) = stop_sender {
            let _ = tx.send(true);
        }
        let handle = { self.event_consumer_tasks.write().await.remove(channel_id) };
        if let Some(handle) = handle {
            let mut handle = handle;
            match tokio::time::timeout(Duration::from_secs(5), &mut handle).await {
                Ok(join_result) => {
                    if let Err(err) = join_result {
                        eprintln!(
                            "[远程IM][OneBot v11 事件] 停止消费器失败，已移除任务句柄: channel_id={}, error={}",
                            channel_id, err
                        );
                    }
                }
                Err(_) => {
                    handle.abort();
                    let _ = handle.await;
                    eprintln!(
                        "[远程IM][OneBot v11 事件] 停止消费器超时，已强制中止: {}",
                        channel_id
                    );
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn start_event_consumer(
        &self,
        channel_id: String,
        state: AppState,
    ) -> Result<(), String> {
        let _guard = self.channel_lifecycle_guard(&channel_id).await;
        self.stop_event_consumer_inner(&channel_id).await?;
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        self.event_consumer_stop_senders
            .write()
            .await
            .insert(channel_id.clone(), stop_tx);
        let tasks = self.event_consumer_tasks.clone();
        let stop_senders = self.event_consumer_stop_senders.clone();
        let task_channel_id = channel_id.clone();
        let manager = self.clone();
        let handle = tokio::spawn(async move {
            napcat_run_event_consumer_loop(manager, task_channel_id.clone(), state, stop_rx).await;
            stop_senders.write().await.remove(&task_channel_id);
            tasks.write().await.remove(&task_channel_id);
        });
        self.event_consumer_tasks
            .write()
            .await
            .insert(channel_id, handle);
        Ok(())
    }
}

