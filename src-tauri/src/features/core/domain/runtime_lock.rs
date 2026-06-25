static LAST_PANIC_SNAPSHOT_SLOT: OnceLock<Arc<Mutex<Option<String>>>> = OnceLock::new();

fn init_last_panic_snapshot_slot(slot: Arc<Mutex<Option<String>>>) {
    let _ = LAST_PANIC_SNAPSHOT_SLOT.set(slot);
}

fn last_panic_snapshot_text() -> String {
    LAST_PANIC_SNAPSHOT_SLOT
        .get()
        .and_then(|slot| slot.lock().ok().and_then(|v| v.clone()))
        .unwrap_or_default()
}

fn state_lock_error_with_panic(
    file: &str,
    line: u32,
    module_path: &str,
    err: &dyn std::fmt::Display,
) -> String {
    let panic_snapshot = last_panic_snapshot_text();
    if panic_snapshot.trim().is_empty() {
        return format!(
            "无法获取状态锁：{}（位置：{}:{} 模块：{}）",
            err, file, line, module_path
        );
    }
    format!(
        "无法获取状态锁：{}（位置：{}:{} 模块：{}；最近 panic：{}）",
        err, file, line, module_path, panic_snapshot
    )
}

fn named_lock_error(
    lock_name: &str,
    file: &str,
    line: u32,
    module_path: &str,
    err: &dyn std::fmt::Display,
) -> String {
    format!(
        "无法获取 {} 锁：{}（位置：{}:{} 模块：{}）",
        lock_name, err, file, line, module_path
    )
}

const CONVERSATION_LOCK_SLOW_WAIT_MS: u128 = 20;
const CONVERSATION_LOCK_SLOW_HOLD_MS: u128 = 20;
const CONVERSATION_LOCK_WAIT_LOG_FIRST_MS: u128 = 200;
const CONVERSATION_LOCK_WAIT_LOG_REPEAT_MS: u128 = 1000;
const CONVERSATION_LOCK_MAX_WAIT_MS: u128 = 3000;

#[derive(Clone)]
struct ConversationLockOwnerSnapshot {
    task_name: String,
    acquired_at: std::time::Instant,
    thread: String,
}

struct ConversationDomainLock {
    inner: Mutex<()>,
    owner: Mutex<Option<ConversationLockOwnerSnapshot>>,
}

impl ConversationDomainLock {
    fn new() -> Self {
        Self {
            inner: Mutex::new(()),
            owner: Mutex::new(None),
        }
    }

    #[track_caller]
    fn lock(&self) -> Result<TimedConversationLockGuard<'_>, String> {
        let location = std::panic::Location::caller();
        let task_name = format!("{}:{}", location.file(), location.line());
        self.lock_named(&task_name)
    }

    fn lock_named(&self, task_name: &str) -> Result<TimedConversationLockGuard<'_>, String> {
        let wait_started_at = std::time::Instant::now();
        let waiter_thread = current_thread_descriptor();
        let mut next_wait_log_ms = CONVERSATION_LOCK_WAIT_LOG_FIRST_MS;
        let owner_before_wait = match self.inner.try_lock() {
            Ok(guard) => {
                return Ok(self.build_guard(guard, task_name.to_string()));
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                self.owner.lock().ok().and_then(|owner| owner.clone())
            }
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err("conversation lock poisoned".to_string());
            }
        };

        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            match self.inner.try_lock() {
                Ok(guard) => {
                    let waited_ms = wait_started_at.elapsed().as_millis();
                    if waited_ms >= CONVERSATION_LOCK_SLOW_WAIT_MS {
                        if let Some(owner) = owner_before_wait {
                            let owner_held_ms = owner.acquired_at.elapsed().as_millis();
                            runtime_log_debug(format!(
                                "[会话锁] 等待完成，等待任务={}，等待线程={}，等待毫秒={}，占用任务={}，占用线程={}，占用毫秒={}",
                                task_name,
                                waiter_thread,
                                waited_ms,
                                owner.task_name,
                                owner.thread,
                                owner_held_ms
                            ));
                        } else {
                            runtime_log_debug(format!(
                                "[会话锁] 等待完成，等待任务={}，等待线程={}，等待毫秒={}",
                                task_name, waiter_thread, waited_ms
                            ));
                        }
                    }
                    return Ok(self.build_guard(guard, task_name.to_string()));
                }
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err("conversation lock poisoned".to_string());
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    let waited_ms = wait_started_at.elapsed().as_millis();
                    if waited_ms >= CONVERSATION_LOCK_MAX_WAIT_MS {
                        let owner = self.owner.lock().ok().and_then(|owner| owner.clone());
                        let message = if let Some(owner) = owner {
                            let owner_held_ms = owner.acquired_at.elapsed().as_millis();
                            format!(
                                "会话锁等待超时，等待任务={}，等待线程={}，等待毫秒={}，占用任务={}，占用线程={}，占用毫秒={}",
                                task_name,
                                waiter_thread,
                                waited_ms,
                                owner.task_name,
                                owner.thread,
                                owner_held_ms
                            )
                        } else {
                            format!(
                                "会话锁等待超时，等待任务={}，等待线程={}，等待毫秒={}",
                                task_name, waiter_thread, waited_ms
                            )
                        };
                        runtime_log_error(format!("[会话锁] 失败，任务=等待会话锁，error={message}"));
                        return Err(message);
                    }
                    if waited_ms >= next_wait_log_ms {
                        let owner = self.owner.lock().ok().and_then(|owner| owner.clone());
                        if let Some(owner) = owner {
                            let owner_held_ms = owner.acquired_at.elapsed().as_millis();
                            runtime_log_warn(format!(
                                "[会话锁] 等待中，等待任务={}，等待线程={}，等待毫秒={}，占用任务={}，占用线程={}，占用毫秒={}",
                                task_name,
                                waiter_thread,
                                waited_ms,
                                owner.task_name,
                                owner.thread,
                                owner_held_ms
                            ));
                        } else {
                            runtime_log_warn(format!(
                                "[会话锁] 等待中，等待任务={}，等待线程={}，等待毫秒={}",
                                task_name, waiter_thread, waited_ms
                            ));
                        }
                        next_wait_log_ms += CONVERSATION_LOCK_WAIT_LOG_REPEAT_MS;
                    }
                }
            }
        }
    }

    fn build_guard<'a>(
        &'a self,
        guard: std::sync::MutexGuard<'a, ()>,
        task_name: String,
    ) -> TimedConversationLockGuard<'a> {
        let acquired_at = std::time::Instant::now();
        if let Ok(mut owner) = self.owner.lock() {
            *owner = Some(ConversationLockOwnerSnapshot {
                task_name: task_name.clone(),
                acquired_at,
                thread: current_thread_descriptor(),
            });
        }
        TimedConversationLockGuard {
            task_name,
            acquired_at,
            thread: current_thread_descriptor(),
            lock: self,
            _guard: guard,
        }
    }
}

struct TimedConversationLockGuard<'a> {
    task_name: String,
    acquired_at: std::time::Instant,
    thread: String,
    lock: &'a ConversationDomainLock,
    _guard: std::sync::MutexGuard<'a, ()>,
}

impl Drop for TimedConversationLockGuard<'_> {
    fn drop(&mut self) {
        let held_ms = self.acquired_at.elapsed().as_millis();
        if let Ok(mut owner) = self.lock.owner.lock() {
            owner.take();
        }
        if held_ms >= CONVERSATION_LOCK_SLOW_HOLD_MS {
            runtime_log_debug(format!(
                "[会话锁] 持有完成，任务={}，线程={}，持有毫秒={}",
                self.task_name, self.thread, held_ms
            ));
        }
    }
}

fn current_thread_descriptor() -> String {
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    format!("{}:{:?}", name, thread.id())
}

#[track_caller]
fn lock_conversation_with_metrics<'a>(
    state: &'a AppState,
    task_name: &str,
) -> Result<TimedConversationLockGuard<'a>, String> {
    let location = std::panic::Location::caller();
    let task_name = format!("{} @ {}:{}", task_name, location.file(), location.line());
    state
        .conversation_lock
        .lock_named(&task_name)
        .map_err(|err| {
            state_lock_error_with_panic(location.file(), location.line(), module_path!(), &err)
        })
}
