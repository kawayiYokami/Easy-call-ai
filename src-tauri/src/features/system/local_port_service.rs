#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPortServiceLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

pub type ChannelLogEntry = LocalPortServiceLogEntry;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPortServiceStatusSnapshot {
    pub listen_addr: String,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPortServiceStartOutcome {
    Started,
    SkippedAlreadyRunning,
}

#[derive(Clone)]
pub struct LocalPortServiceCore {
    listen_addrs: Arc<RwLock<HashMap<String, String>>>,
    status_texts: Arc<RwLock<HashMap<String, String>>>,
    last_errors: Arc<RwLock<HashMap<String, String>>>,
    logs: Arc<RwLock<HashMap<String, Vec<LocalPortServiceLogEntry>>>>,
    lifecycle_locks: Arc<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl LocalPortServiceCore {
    pub fn new() -> Self {
        Self {
            listen_addrs: Arc::new(RwLock::new(HashMap::new())),
            status_texts: Arc::new(RwLock::new(HashMap::new())),
            last_errors: Arc::new(RwLock::new(HashMap::new())),
            logs: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    pub async fn lifecycle_guard(
        &self,
        service_id: &str,
    ) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.lifecycle_locks.lock().await;
            locks
                .entry(service_id.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }

    pub async fn start_serialized<IsRunningFuture, StartFn, StartFuture>(
        &self,
        service_id: &str,
        is_running: IsRunningFuture,
        start_fn: StartFn,
    ) -> Result<LocalPortServiceStartOutcome, String>
    where
        IsRunningFuture: std::future::Future<Output = bool>,
        StartFn: FnOnce() -> StartFuture,
        StartFuture: std::future::Future<Output = Result<(), String>>,
    {
        let _guard = self.lifecycle_guard(service_id).await;
        if is_running.await {
            return Ok(LocalPortServiceStartOutcome::SkippedAlreadyRunning);
        }
        start_fn().await?;
        Ok(LocalPortServiceStartOutcome::Started)
    }

    pub async fn stop_serialized<StopFn, StopFuture>(
        &self,
        service_id: &str,
        stop_fn: StopFn,
    ) -> Result<(), String>
    where
        StopFn: FnOnce() -> StopFuture,
        StopFuture: std::future::Future<Output = Result<(), String>>,
    {
        let _guard = self.lifecycle_guard(service_id).await;
        stop_fn().await
    }

    pub async fn restart_serialized<RestartFn, RestartFuture>(
        &self,
        service_id: &str,
        restart_fn: RestartFn,
    ) -> Result<(), String>
    where
        RestartFn: FnOnce() -> RestartFuture,
        RestartFuture: std::future::Future<Output = Result<(), String>>,
    {
        let _guard = self.lifecycle_guard(service_id).await;
        restart_fn().await
    }

    pub async fn reconcile_serialized<ReconcileFn, ReconcileFuture>(
        &self,
        service_id: &str,
        reconcile_fn: ReconcileFn,
    ) -> Result<(), String>
    where
        ReconcileFn: FnOnce() -> ReconcileFuture,
        ReconcileFuture: std::future::Future<Output = Result<(), String>>,
    {
        let _guard = self.lifecycle_guard(service_id).await;
        reconcile_fn().await
    }

    pub async fn add_log(&self, service_id: &str, level: &str, message: &str) {
        let entry = LocalPortServiceLogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.to_string(),
        };
        let mut logs = self.logs.write().await;
        let service_logs = logs
            .entry(service_id.to_string())
            .or_insert_with(Vec::new);
        service_logs.push(entry);
        if service_logs.len() > 300 {
            let start = service_logs.len() - 300;
            service_logs.drain(0..start);
        }
    }

    pub async fn get_logs(&self, service_id: &str) -> Vec<LocalPortServiceLogEntry> {
        self.logs
            .read()
            .await
            .get(service_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn set_listen_addr(&self, service_id: &str, listen_addr: Option<String>) {
        let mut listen_addrs = self.listen_addrs.write().await;
        match listen_addr.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
            Some(value) => {
                listen_addrs.insert(service_id.to_string(), value);
            }
            None => {
                listen_addrs.remove(service_id);
            }
        }
    }

    pub async fn get_listen_addr(&self, service_id: &str) -> Option<String> {
        self.listen_addrs.read().await.get(service_id).cloned()
    }

    pub async fn set_status_text(&self, service_id: &str, status_text: Option<String>) {
        let mut status_texts = self.status_texts.write().await;
        match status_text.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
            Some(value) => {
                status_texts.insert(service_id.to_string(), value);
            }
            None => {
                status_texts.remove(service_id);
            }
        }
    }

    pub async fn get_status_text(&self, service_id: &str) -> Option<String> {
        self.status_texts.read().await.get(service_id).cloned()
    }

    pub async fn set_last_error(&self, service_id: &str, last_error: Option<String>) {
        let mut last_errors = self.last_errors.write().await;
        match last_error.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
            Some(value) => {
                last_errors.insert(service_id.to_string(), value);
            }
            None => {
                last_errors.remove(service_id);
            }
        }
    }

    pub async fn get_last_error(&self, service_id: &str) -> Option<String> {
        self.last_errors.read().await.get(service_id).cloned()
    }

    pub async fn status_snapshot(&self, service_id: &str) -> LocalPortServiceStatusSnapshot {
        LocalPortServiceStatusSnapshot {
            listen_addr: self.get_listen_addr(service_id).await.unwrap_or_default(),
            status_text: self.get_status_text(service_id).await,
            last_error: self.get_last_error(service_id).await,
        }
    }

    pub async fn clear_runtime_state(&self, service_id: &str) {
        self.listen_addrs.write().await.remove(service_id);
        self.status_texts.write().await.remove(service_id);
        self.last_errors.write().await.remove(service_id);
    }

    pub async fn runtime_state_is_clear(&self, service_id: &str) -> bool {
        !self.listen_addrs.read().await.contains_key(service_id)
            && !self.status_texts.read().await.contains_key(service_id)
            && !self.last_errors.read().await.contains_key(service_id)
    }
}

impl Default for LocalPortServiceCore {
    fn default() -> Self {
        Self::new()
    }
}
