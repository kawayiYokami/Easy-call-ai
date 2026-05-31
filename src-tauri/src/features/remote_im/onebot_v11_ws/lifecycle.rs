impl OnebotV11WsManager {
    /// 订阅事件流
    pub async fn subscribe_events(&self, channel_id: &str) -> Option<broadcast::Receiver<Value>> {
        let senders = self.channel_event_senders.read().await;
        senders.get(channel_id).map(|tx| tx.subscribe())
    }

    /// 关闭所有服务器（通过取消所有 runtime 的 CancellationToken 触发 axum graceful shutdown）
    pub async fn shutdown(&self) {
        let runtimes: Vec<_> = self.channel_runtimes.read().await.values().cloned().collect();
        for runtime in runtimes {
            runtime.cancel.cancel();
        }
    }

    /// 检查渠道是否已连接
    pub async fn is_connected(&self, channel_id: &str) -> bool {
        self.connections.read().await.contains_key(channel_id)
    }

    /// 获取渠道的 CancellationToken，事件消费器用它来感知渠道停止
    pub(crate) async fn get_channel_cancel_token(&self, channel_id: &str) -> Option<CancellationToken> {
        self.channel_runtimes
            .read()
            .await
            .get(channel_id)
            .map(|runtime| runtime.cancel.clone())
    }
}

impl Default for OnebotV11WsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局 OneBot v11 WebSocket 管理器
static ONEBOT_V11_WS_MANAGER: once_cell::sync::Lazy<Arc<OnebotV11WsManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(OnebotV11WsManager::new()));

pub fn onebot_v11_ws_manager() -> Arc<OnebotV11WsManager> {
    ONEBOT_V11_WS_MANAGER.clone()
}

/// 启动 OneBot v11 WebSocket 服务器
pub(crate) async fn onebot_v11_ws_server_start(
    channel: RemoteImChannelConfig,
) -> Result<(), String> {
    let credentials = OnebotV11WsCredentials::from_credentials(&channel.credentials);
    let manager = onebot_v11_ws_manager();
    let channel_id = channel.id.clone();
    manager.start(channel_id, credentials).await
}

/// 获取渠道连接状态
pub async fn get_channel_connection_status(channel_id: String) -> Result<ChannelConnectionStatus, String> {
    let manager = onebot_v11_ws_manager();
    Ok(manager.get_connection_status(&channel_id).await)
}

/// 获取渠道日志
pub async fn get_channel_logs(channel_id: String) -> Result<Vec<ChannelLogEntry>, String> {
    let manager = onebot_v11_ws_manager();
    Ok(manager.get_logs(&channel_id).await)
}
