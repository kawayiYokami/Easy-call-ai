fn weixin_oc_contact_display_name(
    channel: &RemoteImChannelConfig,
    user_id: &str,
) -> String {
    let channel_name = channel.name.trim();
    if !channel_name.is_empty() {
        return channel_name.to_string();
    }
    let normalized_user_id = user_id.trim();
    if !normalized_user_id.is_empty() {
        return normalized_user_id.to_string();
    }
    "个人微信".to_string()
}

async fn handle_weixin_oc_inbound_message(
    channel: &RemoteImChannelConfig,
    state: &AppState,
    msg: WeixinOcInboundMessage,
) -> Result<(), String> {
    let from_user_id = msg
        .from_user_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if from_user_id.is_empty() {
        return Ok(());
    }
    // 群聊：group_id 非空时按群会话处理；私聊退化为 from_user_id
    let group_id = msg
        .group_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    let contact_type = if group_id.is_empty() { "private" } else { "group" };
    let contact_id = if group_id.is_empty() {
        from_user_id.to_string()
    } else {
        group_id.to_string()
    };
    if let Some(token) = msg
        .context_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        weixin_oc_manager()
            .set_context_token(state, &channel.id, &contact_id, token)
            .await;
    }
    let item_list = msg.item_list.unwrap_or_default();
    let creds = WeixinOcCredentials::from_value(&channel.credentials);
    let media = match build_weixin_oc_http_client(creds.normalized_api_timeout_ms()) {
        Ok(client) => weixin_oc_collect_media(&client, &creds, &item_list).await,
        Err(err) => {
            runtime_log_warn(format!(
                "[远程IM][个人微信事件] 媒体客户端初始化失败，保留文本并跳过附件继续，error={err}"
            ));
            let mut parts = Vec::<ChatIngressPart>::new();
            for item in &item_list {
                if item.item_type.unwrap_or(0) == 1 {
                    if let Some(text) = item
                        .text_item
                        .as_ref()
                        .and_then(|value| value.text.as_deref())
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        parts.push(ChatIngressPart::Text {
                            text: text.to_string(),
                        });
                    }
                } else {
                    parts.push(ChatIngressPart::Text {
                        text: "[附件不可用：微信媒体处理暂不可用，已跳过附件并继续]".to_string(),
                    });
                }
            }
            WeixinOcCollectedMedia { parts }
        }
    };
    let final_text = media.parts.iter().filter_map(|part| match part {
        ChatIngressPart::Text { text } => Some(text.trim()),
        ChatIngressPart::Attachment { .. } => None,
    }).filter(|text| !text.is_empty()).collect::<Vec<_>>().join("\n");
    let display_name = weixin_oc_contact_display_name(channel, &contact_id);
    let message_id = msg
        .message_id
        .or(msg.msg_id)
        .map(|value| value.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    remote_im_enqueue_message_internal(
        RemoteImEnqueueInput {
            channel_id: channel.id.clone(),
            platform: RemoteImPlatform::WeixinOc,
            im_name: "weixin".to_string(),
            remote_contact_type: contact_type.to_string(),
            remote_contact_id: contact_id.clone(),
            remote_contact_name: Some(display_name.clone()),
            sender_id: from_user_id.to_string(),
            sender_name: if contact_type == "group" {
                // 群聊中发送者是群成员，无独立昵称时用成员 id 标识
                if from_user_id == contact_id {
                    display_name.clone()
                } else {
                    from_user_id.to_string()
                }
            } else {
                display_name
            },
            sender_avatar_url: None,
            platform_message_id: Some(message_id),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            session: SessionSelector {
                api_config_id: None,
                conversation_id: None,
                department_id: None,
                agent_id: String::new(),
            },
            payload: ChatInputPayload {
                text: if final_text.is_empty() {
                    None
                } else {
                    Some(final_text.clone())
                },
                display_text: if final_text.is_empty() {
                    None
                } else {
                    Some(final_text)
                },
                parts: if media.parts.is_empty() { None } else { Some(media.parts) },
                images: None,
                audios: None,
                attachments: None,
                model: None,
                extra_text_blocks: None,
                mentions: None,
                provider_meta: msg.context_token.map(|token| {
                    serde_json::json!({
                        "contextToken": token,
                    })
                }),
            },
        },
        state,
    )?;
    Ok(())
}

async fn run_single_weixin_oc_poll_cycle(
    channel_id: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = state_read_config_cached(state)?;
    let channel = config
        .remote_im_channels
        .iter()
        .find(|item| item.id == channel_id)
        .cloned()
        .ok_or_else(|| format!("个人微信渠道不存在: {channel_id}"))?;
    let channel = remote_im_channel_with_effective_credentials(state, &channel)?;
    let creds = WeixinOcCredentials::from_value(&channel.credentials);
    let token = creds.token.trim().to_string();
    if token.is_empty() {
        return Err("缺少 token，请先扫码登录".to_string());
    }
    let body = serde_json::json!({
        "base_info": weixin_oc_build_base_info(),
        "get_updates_buf": creds.sync_buf,
    });
    let body_text = serde_json::to_string(&body)
        .map_err(|err| format!("序列化 getupdates 请求失败: {err}"))?;
    let headers = weixin_oc_request_headers(&body_text, Some(&token))?;
    let client = build_weixin_oc_http_client(creds.normalized_long_poll_timeout_ms())?;
    let resp = client
        .post(format!(
            "{}/ilink/bot/getupdates",
            creds.normalized_base_url().trim_end_matches('/')
        ))
        .headers(headers)
        .body(body_text)
        .send()
        .await
        .map_err(|err| format!("请求 getupdates 失败: {err}"))?;
    let status_code = resp.status();
    if !status_code.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("请求 getupdates 失败: status={} body={}", status_code, text));
    }
    let data = resp
        .json::<WeixinOcGetUpdatesResp>()
        .await
        .map_err(|err| format!("解析 getupdates 响应失败: {err}"))?;
    if data.ret.unwrap_or(0) != 0 || data.errcode.unwrap_or(0) != 0 {
        return Err(format!(
            "getupdates 返回错误: ret={} errcode={} errmsg={}",
            data.ret.unwrap_or(0),
            data.errcode.unwrap_or(0),
            data.errmsg.unwrap_or_default()
        ));
    }
    if let Some(next_sync_buf) = data
        .get_updates_buf
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if creds.sync_buf.trim() != next_sync_buf {
            remote_im_patch_channel_private_state(
                state,
                &RemoteImPlatform::WeixinOc,
                &channel.id,
                |private| {
                    private.sync_buf = next_sync_buf.to_string();
                },
            )?;
        }
    }
    for msg in data.msgs.unwrap_or_default() {
        handle_weixin_oc_inbound_message(&channel, state, msg).await?;
    }
    Ok(())
}

fn upsert_weixin_oc_contact(
    runtime: &mut RuntimeStateFile,
    channel: &RemoteImChannelConfig,
    user_id: &str,
) -> (String, bool) {
    let normalized_user_id = user_id.trim();
    let display_name = weixin_oc_contact_display_name(channel, normalized_user_id);
    if let Some(contact) = runtime.remote_im_contacts.iter_mut().find(|item| {
        item.channel_id == channel.id
            && item.remote_contact_type == "private"
            && item.remote_contact_id == normalized_user_id
    }) {
        let current_name = contact.remote_contact_name.trim();
        if current_name.is_empty() || current_name == normalized_user_id {
            contact.remote_contact_name = display_name;
        }
        return (contact.id.clone(), false);
    }

    let contact_id = Uuid::new_v4().to_string();
    runtime.remote_im_contacts.push(RemoteImContact {
        id: contact_id.clone(),
        channel_id: channel.id.clone(),
        platform: RemoteImPlatform::WeixinOc,
        remote_contact_type: "private".to_string(),
        remote_contact_id: normalized_user_id.to_string(),
        remote_contact_name: display_name,
        avatar_url: String::new(),
        remark_name: String::new(),
        allow_send: true,
        allow_send_files: false,
        allow_receive: true,
        activation_mode: "never".to_string(),
        activation_keywords: Vec::new(),
        mute_keywords: default_remote_im_contact_mute_keywords(),
        unmute_keywords: default_remote_im_contact_unmute_keywords(),
        patience_seconds: default_remote_im_contact_patience_seconds(),
        mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
        activation_cooldown_seconds: 0,
        route_mode: "dedicated_contact_conversation".to_string(),
        bound_department_id: None,
        bound_agent_id: None,
        bound_conversation_id: None,
        processing_mode: "continuous".to_string(),
        response_strategy: default_remote_im_contact_response_strategy(),
        response_guidance: default_remote_im_contact_response_guidance(),
            blocked_message_prefixes: default_remote_im_contact_blocked_message_prefixes(),
            group_reply_pacing: RemoteImGroupReplyPacing::default(),
            last_activated_at: None,
        last_message_at: None,
        dingtalk_session_webhook: None,
        dingtalk_session_webhook_expired_time: None,
        onebot_group_members: Vec::new(),
        shell_workspaces: Vec::new(),
    });
    (contact_id, true)
}

#[cfg(test)]
mod weixin_oc_inbound_tests {
    use super::*;

    #[test]
    fn weixin_oc_contact_display_name_prefers_channel_name() {
        let channel = RemoteImChannelConfig {
            id: "channel-1".to_string(),
            name: "我的微信".to_string(),
            platform: RemoteImPlatform::WeixinOc,
            enabled: true,
            credentials: serde_json::json!({}),
            receive_files: true,
            streaming_send: false,
            show_tool_calls: false,
            filter_markdown: false,
            allow_send_files: false,
            behavior_settings: RemoteImChannelBehaviorSettings::default(),
        };

        let display_name = weixin_oc_contact_display_name(&channel, "wxid_123");

        assert_eq!(display_name, "我的微信".to_string());
    }

    #[test]
    fn inbound_message_parses_group_and_state_fields() {
        let raw = serde_json::json!({
            "seq": 12,
            "message_id": "msg-1",
            "from_user_id": "member_a",
            "to_user_id": "bot_id",
            "context_token": "token-x",
            "session_id": "session-1",
            "group_id": "group-888",
            "message_type": 1,
            "message_state": 1,
            "create_time_ms": 1750000000000i64,
            "run_id": "run-1",
            "client_id": "client-1",
            "item_list": [
                { "type": 1, "text_item": { "text": "群消息" } }
            ]
        });
        let msg: WeixinOcInboundMessage = serde_json::from_value(raw).unwrap();

        assert_eq!(msg.group_id.as_deref(), Some("group-888"));
        assert_eq!(msg.message_state, Some(1));
        assert_eq!(msg.run_id.as_deref(), Some("run-1"));
        assert_eq!(msg.session_id.as_deref(), Some("session-1"));
        assert_eq!(msg.seq, Some(12));
        let items = msg.item_list.unwrap_or_default();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, Some(1));
    }

    #[test]
    fn inbound_message_parses_voice_text_and_media_fields() {
        let raw = serde_json::json!({
            "message_id": "msg-2",
            "from_user_id": "member_b",
            "group_id": "group-888",
            "item_list": [
                {
                    "type": 3,
                    "voice_item": {
                        "media": {
                            "encrypt_query_param": "enc=1",
                            "aes_key": "base64key",
                            "encrypt_type": 1,
                            "full_url": "https://cdn.example.com/full/voice.silk"
                        },
                        "encode_type": 6,
                        "sample_rate": 24000,
                        "playtime": 3000,
                        "text": "你好，这是语音转文字"
                    }
                }
            ]
        });
        let msg: WeixinOcInboundMessage = serde_json::from_value(raw).unwrap();

        let items = msg.item_list.unwrap_or_default();
        let voice = items[0].voice_item.as_ref().unwrap();
        assert_eq!(voice.text.as_deref(), Some("你好，这是语音转文字"));
        assert_eq!(voice.encode_type, Some(6));
        assert_eq!(voice.sample_rate, Some(24000));
        assert_eq!(voice.playtime, Some(3000));
        let media = voice.media.as_ref().unwrap();
        assert_eq!(media.full_url.as_deref(), Some("https://cdn.example.com/full/voice.silk"));
        assert_eq!(media.encrypt_type, Some(1));
    }

    #[test]
    fn inbound_message_without_group_is_private() {
        let raw = serde_json::json!({
            "message_id": "msg-3",
            "from_user_id": "wxid_123",
            "item_list": []
        });
        let msg: WeixinOcInboundMessage = serde_json::from_value(raw).unwrap();
        assert!(msg.group_id.is_none());
        assert_eq!(msg.from_user_id.as_deref(), Some("wxid_123"));
    }
}

fn sync_weixin_oc_contact_from_user_id(
    state: &AppState,
    channel: &RemoteImChannelConfig,
    user_id: &str,
) -> Result<(String, bool), String> {
    let normalized_user_id = user_id.trim();
    if normalized_user_id.is_empty() {
        return Err("当前登录状态没有返回联系人 user_id，暂时无法补录联系人".to_string());
    }
    state_mutate_runtime_state_cached(state, |runtime| {
        Ok(upsert_weixin_oc_contact(
            runtime,
            channel,
            normalized_user_id,
        ))
    })
}

pub(crate) async fn weixin_oc_send_text_message(
    credentials: WeixinOcCredentials,
    to_user_id: &str,
    text: &str,
    context_token: Option<&str>,
) -> Result<String, RemoteImSdkSendError> {
    let item_list = vec![serde_json::json!({
        "type": WEIXIN_OC_TEXT_ITEM_TYPE,
        "text_item": {
            "text": text
        }
    })];
    weixin_oc_send_message_items(credentials, to_user_id, item_list, context_token).await
}

pub(crate) async fn weixin_oc_send_message_items(
    credentials: WeixinOcCredentials,
    to_user_id: &str,
    item_list: Vec<Value>,
    context_token: Option<&str>,
) -> Result<String, RemoteImSdkSendError> {
    let client = build_weixin_oc_http_client(credentials.normalized_api_timeout_ms())
        .map_err(RemoteImSdkSendError::definitely_not_sent)?;
    let client_id = Uuid::new_v4().simple().to_string();
    let body = serde_json::json!({
        "base_info": weixin_oc_build_base_info(),
        "msg": {
            "from_user_id": "",
            "to_user_id": to_user_id,
            "client_id": client_id,
            "message_type": 2,
            "message_state": 2,
            "context_token": context_token.map(str::trim).filter(|value| !value.is_empty()),
            "item_list": item_list
        }
    });
    let body_text = serde_json::to_string(&body)
        .map_err(|err| {
            RemoteImSdkSendError::definitely_not_sent(format!(
                "序列化 sendmessage 请求失败: {err}"
            ))
        })?;
    let headers = weixin_oc_request_headers(&body_text, Some(credentials.token.as_str()))
        .map_err(RemoteImSdkSendError::definitely_not_sent)?;
    let resp = client
        .post(format!(
            "{}/ilink/bot/sendmessage",
            credentials.normalized_base_url().trim_end_matches('/')
        ))
        .headers(headers)
        .body(body_text)
        .send()
        .await
        .map_err(|err| {
            RemoteImSdkSendError::uncertain(format!("请求 sendmessage 失败: {err}"))
        })?;
    let status_code = resp.status();
    if !status_code.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(remote_im_http_rejection_error(
            status_code,
            format!(
                "请求 sendmessage 失败: status={} body={}",
                status_code, body
            ),
        ));
    }
    let resp_body: serde_json::Value = resp
        .json()
        .await
        .map_err(|err| {
            RemoteImSdkSendError::uncertain(format!(
                "解析 sendmessage 响应失败: {err}"
            ))
        })?;
    let ret = resp_body
        .get("ret")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let errcode = resp_body
        .get("errcode")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    if ret != 0 || errcode != 0 {
        let errmsg = resp_body
            .get("errmsg")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        return Err(RemoteImSdkSendError::definitely_not_sent(format!(
            "请求 sendmessage 失败: ret={} errcode={} errmsg={} resp={}",
            ret, errcode, errmsg, resp_body
        )));
    }
    Ok(client_id)
}

#[tauri::command]
async fn remote_im_weixin_oc_start_login(
    input: WeixinOcLoginStartInput,
    state: State<'_, AppState>,
) -> Result<WeixinOcLoginStartResult, String> {
    weixin_oc_manager().start_login(state.inner(), input).await
}

#[tauri::command]
async fn remote_im_weixin_oc_get_login_status(
    input: WeixinOcLoginStatusInput,
    state: State<'_, AppState>,
) -> Result<WeixinOcLoginStatusResult, String> {
    weixin_oc_manager()
        .poll_login_status(state.inner(), input)
        .await
}

#[tauri::command]
async fn remote_im_weixin_oc_logout(
    input: WeixinOcLoginStatusInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    weixin_oc_manager()
        .logout(state.inner(), input.channel_id.as_str())
        .await?;
    Ok(true)
}

fn remote_im_weixin_oc_sync_contacts_inner(
    state: &AppState,
    input: WeixinOcLoginStatusInput,
) -> Result<WeixinOcSyncContactsResult, String> {
    let config = state_read_config_cached(state)?;
    let channel = remote_im_channel_by_id(&config, &input.channel_id)
        .ok_or_else(|| format!("渠道不存在: {}", input.channel_id))?;
    if channel.platform != RemoteImPlatform::WeixinOc {
        return Err("该渠道不是个人微信渠道".to_string());
    }
    let credentials = remote_im_effective_credentials(state, channel)?;
    let creds = WeixinOcCredentials::from_value(&credentials);
    if creds.account_id.trim().is_empty() || creds.token.trim().is_empty() {
        return Ok(WeixinOcSyncContactsResult {
            channel_id: input.channel_id,
            synced_count: 0,
            message: "当前还没有完成扫码登录，请先登录后再同步联系人。".to_string(),
        });
    }
    let user_id = creds.user_id.trim().to_string();
    let (_, created) = sync_weixin_oc_contact_from_user_id(state, channel, &user_id)?;
    Ok(WeixinOcSyncContactsResult {
        channel_id: input.channel_id,
        synced_count: 1,
        message: if created {
            format!("已同步个人微信联系人：{}", user_id)
        } else {
            format!("联系人已存在，无需重复同步：{}", user_id)
        },
    })
}

#[tauri::command]
async fn remote_im_weixin_oc_sync_contacts(
    input: WeixinOcLoginStatusInput,
    state: State<'_, AppState>,
) -> Result<WeixinOcSyncContactsResult, String> {
    remote_im_weixin_oc_sync_contacts_inner(state.inner(), input)
}
