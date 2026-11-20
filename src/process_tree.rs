//! Stopping every process a `verify` command starts, not only its shell (T-172; R-VERIFY-1).
//!
//! A `verify` line runs in a tree of processes: the shell and whatever it starts. On a timeout the
//! whole tree must stop, and a check that ends must not leave processes behind. `std` has no call
//! for that, so this module is the only place that calls the operating system directly:
//!
//! - Unix: the shell leads its own process group (`process_group(0)`, in `std`), and `killpg`
//!   sends SIGKILL to the whole group (libc, NFR-9).
//! - Windows: the shell is put in a Job Object that kills its processes when the job closes
//!   (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), and `TerminateJobObject` stops them at once
//!   (windows-sys, NFR-9).
//!
//! Known limit (Windows): the shell joins its job just after it has started, so a process the shell
//! starts before that moment stays outside the job. A `verify` line starts its own processes only
//! after the shell is running, so in practice every process it starts is inside the job.

use std::io;
use std::process::{Child, Command};

/// The shell of one `verify` run and every process it starts. Dropping it releases the tree; on
/// Windows that also kills any process still in the job.
pub struct ProcessTree {
    #[cfg(unix)]
    group: libc::pid_t,
    #[cfg(windows)]
    job: windows_job::OwnedJob,
}

#[cfg(unix)]
impl ProcessTree {
    /// Makes the process that `command` starts the leader of a new process group. Call it before
    /// the command is spawned.
    pub fn prepare(command: &mut Command) {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    /// The tree of a child started after [`ProcessTree::prepare`]: its process group is its own
    /// process id.
    pub fn attach(child: &Child) -> io::Result<Self> {
        let group = libc::pid_t::try_from(child.id())
            .map_err(|e| io::Error::other(format!("process id out of range: {e}")))?;
        Ok(Self { group })
    }

    /// Kills every process of the group, the shell included. A group whose processes have all
    /// ended is not an error.
    pub fn stop_all(&self) {
        // SAFETY: `killpg` takes a process group id and a signal number and reads no memory from
        // us. The group is the one `prepare` made for this shell.
        let _ = unsafe { libc::killpg(self.group, libc::SIGKILL) };
    }
}

#[cfg(windows)]
impl ProcessTree {
    /// Nothing to set up on Windows: the job is made after the child has started, in `attach`.
    pub fn prepare(_command: &mut Command) {}

    /// Puts the started child in a new Job Object that kills its processes when it closes.
    pub fn attach(child: &Child) -> io::Result<Self> {
        Ok(Self {
            job: windows_job::OwnedJob::with_process(child)?,
        })
    }

    /// Terminates every process in the job, the shell included.
    pub fn stop_all(&self) {
        self.job.terminate();
    }
}

#[cfg(windows)]
mod windows_job {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };

    /// A Job Object handle that this process owns. Dropping it closes the handle, and with
    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` that kills what is still in the job.
    pub struct OwnedJob(HANDLE);

    impl OwnedJob {
        /// A new job with `child` in it, set to kill its processes when the job closes.
        pub fn with_process(child: &Child) -> io::Result<Self> {
            // SAFETY: null security attributes and a null name ask for an unnamed job object; the
            // call keeps no pointer of ours.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Self(handle);

            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let size = u32::try_from(std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                .map_err(io::Error::other)?;
            // SAFETY: `limits` lives until the call returns, and `size` is its exact size in bytes.
            let set = unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                    size,
                )
            };
            if set == 0 {
                return Err(io::Error::last_os_error());
            }

            // SAFETY: `job.0` is the job made above, and the child's handle is valid because the
            // child is not yet waited for.
            let assigned = unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle()) };
            if assigned == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        }

        /// Terminates every process in the job with exit code 1.
        pub fn terminate(&self) {
            // SAFETY: `self.0` is a job handle this value owns until it is dropped.
            let _ = unsafe { TerminateJobObject(self.0, 1) };
        }
    }

    impl Drop for OwnedJob {
        fn drop(&mut self) {
            // SAFETY: the handle is owned by this value and is closed exactly once, here.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use super::ProcessTree;

    /// Whether `child` ends within ten seconds. Polls, so it does not wait for the tree to be dropped.
    fn ends_soon(child: &mut Child) -> bool {
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// `stop_all` ends the process while the tree is still held, so the test does not rely on the
    /// tree being dropped (on Windows, dropping it also kills the job's processes).
    #[test]
    fn stop_all_ends_the_started_process_while_the_tree_is_still_held() {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("powershell.exe");
            command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"]);
            command
        } else {
            let mut command = Command::new("sh");
            command.args(["-c", "sleep 60"]);
            command
        };
        ProcessTree::prepare(&mut command);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start the test process");
        let tree = ProcessTree::attach(&child).expect("attach the test process");

        tree.stop_all();

        assert!(
            ends_soon(&mut child),
            "the process was still running after stop_all"
        );
    }
}
