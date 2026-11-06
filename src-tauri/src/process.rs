use crate::domain::*;
use std::{path::Path, process::Stdio, sync::Arc};
use tokio::{io::AsyncReadExt, process::Command};
use tokio_util::sync::CancellationToken;

pub type Progress = Arc<dyn Fn(i64) + Send + Sync>;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const STDOUT_LIMIT_BYTES: usize = 16 * 1024 * 1024;
const STDERR_LIMIT_BYTES: usize = 64 * 1024;

/// Converts string literals into an owned argument list.
pub fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

pub async fn run(
    executable: &Path,
    args: &[String],
    cancel: &CancellationToken,
    progress: Option<Progress>,
) -> Result<String> {
    if !executable.is_file() {
        return Err(AppError::new(
            "DEPENDENCY",
            format!(
                "Missing bundled {}. Run the dependency preparation script, then restart.",
                executable.file_name().unwrap_or_default().to_string_lossy()
            ),
        ));
    }
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn()?;

    #[cfg(windows)]
    let _tree = ProcessTree::attach(&child)?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out_task = tokio::spawn(read_bounded(stdout, STDOUT_LIMIT_BYTES, progress));
    let err_task = tokio::spawn(read_bounded(stderr, STDERR_LIMIT_BYTES, None));
    let status = tokio::select! {
        status = child.wait() => status?,
        _ = cancel.cancelled() => {
            #[cfg(windows)]
            _tree.terminate();
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = out_task.await;
            let _ = err_task.await;
            return Err(AppError::new(
                "CANCELLED",
                "Job cancelled; completed outputs were retained",
            ));
        }
    };
    let output = out_task
        .await
        .map_err(|_| AppError::new("PROCESS", "Output reader failed"))??;
    let errors = err_task
        .await
        .map_err(|_| AppError::new("PROCESS", "Error reader failed"))??;
    if !status.success() {
        return Err(AppError::new("TOOL_FAILED", failure_message(&errors)));
    }
    Ok(output)
}

/// Maps tool stderr to a user-facing message.
///
/// Do not surface signed URLs, tokens, or full untrusted extractor diagnostics.
fn failure_message(errors: &str) -> &'static str {
    if errors.contains("No space left") {
        "Destination is full. Free space and retry."
    } else if errors.contains("Permission denied") || errors.contains("Access is denied") {
        "Permission denied. Choose a writable destination."
    } else if errors.contains("Unknown encoder") {
        "The bundled FFmpeg is missing a required encoder."
    } else if errors.contains("Sign in") || errors.contains("Private video") {
        "This source requires authentication. Use an accessible source or a local file."
    } else {
        "The media tool failed. Check the source, destination, and bundled tool versions, \
         then retry."
    }
}

async fn read_bounded<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    limit: usize,
    progress: Option<Progress>,
) -> Result<String> {
    let mut all = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut pending = String::new();
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        if progress.is_some() {
            pending.push_str(&String::from_utf8_lossy(&chunk[..n]));
            while let Some(end) = pending.find('\n') {
                let line: String = pending.drain(..=end).collect();
                if let Some(value) = line.trim().strip_prefix("out_time_us=") {
                    if let Ok(time) = value.parse() {
                        if let Some(f) = &progress {
                            f(time)
                        }
                    }
                }
            }
            if pending.len() > 65536 {
                pending.clear()
            }
        }
        all.extend_from_slice(&chunk[..n]);
        if all.len() > limit {
            all.drain(..all.len() - limit);
        }
    }
    Ok(String::from_utf8_lossy(&all).into_owned())
}

#[cfg(windows)]
struct ProcessTree(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for ProcessTree {}
#[cfg(windows)]
impl ProcessTree {
    fn attach(child: &tokio::process::Child) -> Result<Self> {
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.raw_handle().unwrap() as _) == 0
            {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(error.into());
            }
            Ok(Self(job))
        }
    }

    fn terminate(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
