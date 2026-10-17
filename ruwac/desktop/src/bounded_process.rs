//! Bounded full-duplex exchange with an owned process group. No pipe operation
//! waits for the peer, and the same deadline covers upload, output and exit.
use std::{
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static OWNED: std::sync::Mutex<(bool, Vec<u32>)> = std::sync::Mutex::new((false, Vec::new()));
/// Normal application shutdown kills registered groups before Rust tears down
/// worker threads. A reaped PID is removed while holding the same lock.
pub fn shutdown() {
    let mut registry = OWNED.lock().unwrap_or_else(|e| e.into_inner());
    registry.0 = true;
    for pid in &registry.1 {
        unsafe {
            libc::kill(-(*pid as i32), libc::SIGKILL);
        }
    }
}
struct Owned(Child, bool);
impl Owned {
    fn finish(&mut self) -> io::Result<std::process::ExitStatus> {
        let mut registry = OWNED.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        let result = self.0.wait();
        registry.1.retain(|pid| *pid != self.0.id());
        self.1 = true;
        result
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // The group was created by this spawn, never supplied by a caller.
        if !self.1 {
            let _ = self.finish();
        }
    }
}
fn nonblocking(fd: &impl AsRawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn drain(pipe: &mut Option<impl Read>, bytes: &mut Vec<u8>, cap: usize) -> Result<(), String> {
    let mut block = [0; 4096];
    for _ in 0..4 {
        let Some(stream) = pipe else {
            break;
        };
        match stream.read(&mut block) {
            Ok(0) => {
                *pipe = None;
                break;
            }
            Ok(n) => {
                if bytes.len() + n > cap {
                    return Err("SSH response exceeded its bound".into());
                }
                bytes.extend_from_slice(&block[..n]);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err("SSH response could not be read".into()),
        }
    }
    Ok(())
}
pub fn exchange(
    command: &mut Command,
    input: &[u8],
    allowed: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let end = Instant::now() + timeout;
    if !allowed.load(Ordering::Acquire) {
        return Err("Image upload cancelled".into());
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut registry = OWNED.lock().unwrap_or_else(|e| e.into_inner());
    if registry.0 {
        return Err("Image uploader is shutting down".into());
    }
    #[cfg(target_os = "linux")]
    unsafe {
        let parent = std::process::id() as libc::pid_t;
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 || libc::getppid() != parent
            {
                return Err(io::Error::other("Image upload parent changed"));
            }
            Ok(())
        });
    }
    let mut child = Owned(
        command.spawn().map_err(|_| "SSH upload could not start")?,
        false,
    );
    registry.1.push(child.0.id());
    drop(registry);
    let mut stdin = child.0.stdin.take();
    let mut stdout = child.0.stdout.take();
    let mut stderr = child.0.stderr.take();
    nonblocking(stdin.as_ref().unwrap()).map_err(|_| "SSH input setup failed")?;
    nonblocking(stdout.as_ref().unwrap()).map_err(|_| "SSH output setup failed")?;
    nonblocking(stderr.as_ref().unwrap()).map_err(|_| "SSH error setup failed")?;
    let (mut offset, mut out, mut err) = (0, Vec::new(), Vec::new());
    loop {
        if !allowed.load(Ordering::Acquire) {
            return Err("Image upload cancelled".into());
        }
        if Instant::now() >= end {
            return Err("Image upload timed out".into());
        }
        if let Some(stream) = stdin.as_mut() {
            match stream.write(&input[offset..(offset + 65536).min(input.len())]) {
                Ok(0) if offset < input.len() => return Err("SSH closed image input".into()),
                Ok(n) => offset += n,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    ()
                }
                Err(_) => return Err("SSH image input failed".into()),
            }
            if offset == input.len() {
                stdin = None;
            }
        }
        drain(&mut stdout, &mut out, 4096)?;
        drain(&mut stderr, &mut err, 4096)?;
        // Observe without reaping: its PID/PGID cannot be reused before group
        // cleanup, even when the main child exits before its descendants.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let observed = unsafe {
            libc::waitid(
                libc::P_PID,
                child.0.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if observed < 0 {
            return Err("SSH upload status unavailable".into());
        }
        if unsafe { info.si_pid() } != 0 && stdout.is_none() && stderr.is_none() {
            let status = child
                .finish()
                .map_err(|_| "SSH upload status unavailable")?;
            if !status.success() || offset != input.len() {
                return Err(
                    "SSH image upload failed; verify endpoint authentication and permissions"
                        .into(),
                );
            }
            return Ok(out);
        }
        // poll wakes for any pipe, not only the upload pipe; bounded reads above
        // prevent a flooding peer from starving cancellation and deadline checks.
        let mut fds = Vec::new();
        if let Some(p) = &stdin {
            fds.push(libc::pollfd {
                fd: p.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            });
        }
        if let Some(p) = &stdout {
            fds.push(libc::pollfd {
                fd: p.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }
        if let Some(p) = &stderr {
            fds.push(libc::pollfd {
                fd: p.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }
        unsafe {
            libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 10);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn python(code: &str) -> Command {
        let mut c = Command::new("python3");
        c.args(["-c", code]);
        c
    }
    #[test]
    fn stalled_input_is_inside_deadline() {
        let start = Instant::now();
        let e = exchange(
            &mut python("import time; time.sleep(60)"),
            &vec![0; 2 * 1024 * 1024],
            &AtomicBool::new(true),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(e.contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn drains_both_output_pipes_during_upload() {
        let out = exchange(&mut python("import os,sys; os.write(2,b'x'*4096); os.write(1,b'p'*2048); d=sys.stdin.buffer.read(); print(len(d))"), &vec![0; 2*1024*1024], &AtomicBool::new(true), Duration::from_secs(5)).unwrap();
        assert!(out.ends_with(b"2097152\n"));
    }
    #[test]
    fn output_flood_is_bounded() {
        assert!(
            exchange(
                &mut python("import os; os.write(1,b'x'*65536)"),
                b"",
                &AtomicBool::new(true),
                Duration::from_secs(5)
            )
            .unwrap_err()
            .contains("bound")
        );
    }
    #[test]
    fn exited_parent_cannot_hide_descendant_pipe_timeout() {
        let started = Instant::now();
        let error = exchange(
            &mut python("import os,time; pid=os.fork(); os._exit(0) if pid else time.sleep(60)"),
            b"",
            &AtomicBool::new(true),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(error.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn cancellation_during_blocked_input_reaps_the_child() {
        let allowed = std::sync::Arc::new(AtomicBool::new(true));
        let other = allowed.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            other.store(false, Ordering::Release);
        });
        assert!(
            exchange(
                &mut python("import time; time.sleep(60)"),
                &vec![0; 2 * 1024 * 1024],
                &allowed,
                Duration::from_secs(5)
            )
            .unwrap_err()
            .contains("cancelled")
        );
        t.join().unwrap();
    }
}
