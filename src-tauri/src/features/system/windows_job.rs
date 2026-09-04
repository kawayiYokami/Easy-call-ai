#[cfg(target_os = "windows")]
#[derive(Debug)]
struct WindowsJobGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsJobGuard {}

#[cfg(target_os = "windows")]
unsafe impl Sync for WindowsJobGuard {}

#[cfg(target_os = "windows")]
impl Drop for WindowsJobGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;

        if !self.0.is_null() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[cfg(target_os = "windows")]
impl WindowsJobGuard {
    fn create_kill_on_close() -> Result<Self, String> {
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            SetInformationJobObject,
        };

        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err("CreateJobObjectW failed".to_string());
        }
        let guard = Self(job);

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let set_ok = unsafe {
            SetInformationJobObject(
                guard.0,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set_ok == 0 {
            return Err("SetInformationJobObject failed".to_string());
        }

        Ok(guard)
    }

    fn assign_raw_process_handle(
        &self,
        process: windows_sys::Win32::Foundation::HANDLE,
    ) -> Result<(), String> {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

        let assign_ok = unsafe { AssignProcessToJobObject(self.0, process) };
        if assign_ok == 0 {
            return Err("AssignProcessToJobObject failed".to_string());
        }
        Ok(())
    }

    fn terminate_job(&self) -> Result<(), String> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;

        let terminate_ok = unsafe { TerminateJobObject(self.0, 1) };
        if terminate_ok == 0 {
            return Err("TerminateJobObject failed".to_string());
        }
        Ok(())
    }

    fn assign_process_id(&self, pid: u32) -> Result<(), String> {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };

        struct ProcessHandleGuard(HANDLE);
        impl Drop for ProcessHandleGuard {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe {
                        let _ = CloseHandle(self.0);
                    }
                }
            }
        }

        let process = unsafe {
            OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        };
        if process.is_null() {
            return Err(format!("OpenProcess failed for pid={pid}"));
        }
        let process_guard = ProcessHandleGuard(process);
        self.assign_raw_process_handle(process_guard.0)
            .map_err(|_| format!("AssignProcessToJobObject failed for pid={pid}"))
    }
}
