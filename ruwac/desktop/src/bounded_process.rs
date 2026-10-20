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
    exchange_inner(command, input, allowed, timeout, None)
}

pub struct Capture {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Capture a short command without trusting it to close pipes or exit. The
/// deadline and cancellation cover input, both output streams and descendants.
pub fn capture(
    command: &mut Command,
    input: &[u8],
    allowed: &AtomicBool,
    timeout: Duration,
    stdout_cap: usize,
    stderr_cap: usize,
) -> Result<Capture, String> {
    let end = Instant::now() + timeout;
    if !allowed.load(Ordering::Acquire) {
        return Err("SSH check cancelled".into());
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut registry = OWNED.lock().unwrap_or_else(|e| e.into_inner());
    if registry.0 {
        return Err("SSH checks are shutting down".into());
    }
    #[cfg(target_os = "linux")]
    unsafe {
        let parent = std::process::id() as libc::pid_t;
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 || libc::getppid() != parent
            {
                return Err(io::Error::other("SSH check parent changed"));
            }
            Ok(())
        });
    }
    let mut child = Owned(
        command.spawn().map_err(|_| "SSH check could not start")?,
        false,
    );
    registry.1.push(child.0.id());
    drop(registry);
    let (mut stdin, mut stdout, mut stderr) = (
        child.0.stdin.take(),
        child.0.stdout.take(),
        child.0.stderr.take(),
    );
    nonblocking(stdin.as_ref().unwrap()).map_err(|_| "SSH input setup failed")?;
    nonblocking(stdout.as_ref().unwrap()).map_err(|_| "SSH output setup failed")?;
    nonblocking(stderr.as_ref().unwrap()).map_err(|_| "SSH error setup failed")?;
    let (mut offset, mut out, mut err, mut status) = (0, Vec::new(), Vec::new(), None);
    loop {
        if !allowed.load(Ordering::Acquire) {
            return Err("SSH check cancelled".into());
        }
        if Instant::now() >= end {
            return Err("SSH check timed out".into());
        }
        if let Some(stream) = stdin.as_mut().filter(|_| offset < input.len()) {
            match stream.write(&input[offset..(offset + 16384).min(input.len())]) {
                Ok(0) => return Err("SSH check closed its input".into()),
                Ok(n) => offset += n,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    ()
                }
                Err(_) => return Err("SSH check input failed".into()),
            }
        }
        if offset == input.len() {
            stdin = None;
        }
        drain(&mut stdout, &mut out, stdout_cap)?;
        drain(&mut stderr, &mut err, stderr_cap)?;
        if status.is_none() {
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
                return Err("SSH check status unavailable".into());
            }
            if unsafe { info.si_pid() } != 0 {
                // Reap only after killing this owned group. A background proxy
                // cannot retain our pipes after its foreground command exits.
                status = Some(child.finish().map_err(|_| "SSH check status unavailable")?);
            }
        }
        if stdout.is_none() && stderr.is_none() {
            if let Some(status) = status {
                return Ok(Capture {
                    status,
                    stdout: out,
                    stderr: err,
                });
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub fn exchange_confirmed(
    command: &mut Command,
    input: &[u8],
    allowed: &AtomicBool,
    timeout: Duration,
    ready: &[u8],
    commit: &[u8],
) -> Result<Vec<u8>, String> {
    if ready.is_empty() || commit.is_empty() {
        return Err("Invalid image publication handshake".into());
    }
    exchange_inner(command, input, allowed, timeout, Some((ready, commit)))
}

fn exchange_inner(
    command: &mut Command,
    input: &[u8],
    allowed: &AtomicBool,
    timeout: Duration,
    confirmation: Option<(&[u8], &[u8])>,
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
    let (mut ready_received, mut commit_offset) = (false, 0);
    loop {
        if !allowed.load(Ordering::Acquire) {
            return Err("Image upload cancelled".into());
        }
        if Instant::now() >= end {
            return Err("Image upload timed out".into());
        }
        let sending_image = offset < input.len();
        let sending_commit =
            confirmation.is_some_and(|(_, commit)| ready_received && commit_offset < commit.len());
        if let Some(stream) = stdin.as_mut().filter(|_| sending_image || sending_commit) {
            let (payload, written) = if sending_image {
                (input, &mut offset)
            } else {
                (confirmation.unwrap().1, &mut commit_offset)
            };
            match stream.write(&payload[*written..(*written + 65536).min(payload.len())]) {
                Ok(0) => return Err("SSH closed image input".into()),
                Ok(n) => *written += n,
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
        }
        if offset == input.len()
            && confirmation
                .is_none_or(|(_, commit)| ready_received && commit_offset == commit.len())
        {
            stdin = None;
        }
        drain(&mut stdout, &mut out, 4096)?;
        drain(&mut stderr, &mut err, 4096)?;
        if let Some((ready, _)) = confirmation.filter(|_| !ready_received && offset == input.len())
        {
            if !ready.starts_with(&out) {
                return Err("Invalid SSH image publication readiness".into());
            }
            if out == ready {
                // Commit is written on the next loop, after checking revocation
                // and the deadline again. EOF/cancel before it cannot publish.
                ready_received = true;
                out.clear();
            }
        }
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
            if !status.success()
                || offset != input.len()
                || confirmation
                    .is_some_and(|(_, commit)| !ready_received || commit_offset != commit.len())
            {
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
        if let Some(p) = stdin.as_ref().filter(|_| {
            offset < input.len()
                || confirmation
                    .is_some_and(|(_, commit)| ready_received && commit_offset < commit.len())
        }) {
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
    fn capture_drains_both_streams_before_waiting_for_exit() {
        let result = capture(
            &mut python("import os,sys; os.write(1,b'o'*131072); os.write(2,b'e'*131072); assert len(sys.stdin.buffer.read())==65536; sys.exit(7)"),
            &[0; 65536], &AtomicBool::new(true), Duration::from_secs(5), 262144, 262144,
        ).unwrap();
        assert_eq!(result.status.code(), Some(7));
        assert_eq!(result.stdout, vec![b'o'; 131072]);
        assert_eq!(result.stderr, vec![b'e'; 131072]);
    }
    #[test]
    fn capture_refuses_flooding_output_and_obeys_one_deadline() {
        let start = Instant::now();
        let error = capture(
            &mut python("import os;\nwhile True: os.write(2,b'e'*65536)"),
            &[],
            &AtomicBool::new(true),
            Duration::from_secs(3),
            4096,
            8192,
        )
        .err()
        .unwrap();
        assert!(error.contains("exceeded"));
        assert!(start.elapsed() < Duration::from_secs(2));
        let error = capture(
            &mut python("import time; time.sleep(60)"),
            &[],
            &AtomicBool::new(true),
            Duration::from_millis(100),
            4096,
            4096,
        )
        .err()
        .unwrap();
        assert!(error.contains("timed out"));
    }
    #[test]
    fn capture_cancels_and_cleans_a_background_pipe_holder() {
        let allowed = AtomicBool::new(true);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                allowed.store(false, Ordering::Release);
            });
            let error = capture(
                &mut python("import time; time.sleep(60)"),
                &[],
                &allowed,
                Duration::from_secs(3),
                4096,
                4096,
            )
            .err()
            .unwrap();
            assert!(error.contains("cancelled"));
        });
        let start = Instant::now();
        let result = capture(
            &mut python("import os,subprocess; p=subprocess.Popen(['sleep','60']); print(p.pid,flush=True); os._exit(0)"),
            &[], &AtomicBool::new(true), Duration::from_secs(3), 4096, 4096,
        ).unwrap();
        assert!(result.status.success());
        let pid = String::from_utf8(result.stdout)
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();
        let state = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok();
        assert!(state.is_none_or(|s| s.rsplit(')').next().unwrap().trim_start().starts_with('Z')));
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn publication_commit_waits_for_verified_peer_readiness() {
        let out = exchange_confirmed(
            &mut python("import os,sys,select; d=sys.stdin.buffer.read(131072); assert not select.select([sys.stdin],[],[],0.05)[0]; os.write(1,b'READY\\n'); assert sys.stdin.buffer.readline()==b'COMMIT\\n'; os.write(1,b'/generated/path\\n')"),
            &vec![0; 131072], &AtomicBool::new(true), Duration::from_secs(5), b"READY\n", b"COMMIT\n",
        ).unwrap();
        assert_eq!(out, b"/generated/path\n");
    }
    #[test]
    fn cancellation_revokes_commit_while_peer_verifies_payload() {
        let allowed = AtomicBool::new(true);
        let started = Instant::now();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                allowed.store(false, Ordering::Release);
            });
            let error = exchange_confirmed(
                &mut python("import sys,time; sys.stdin.buffer.read(4096); time.sleep(60)"),
                &[0; 4096],
                &allowed,
                Duration::from_secs(5),
                b"READY\n",
                b"COMMIT\n",
            )
            .unwrap_err();
            assert!(error.contains("cancelled"));
        });
        assert!(started.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn unexpected_readiness_never_authorizes_publication() {
        let error = exchange_confirmed(
            &mut python("import os,sys; sys.stdin.buffer.read(3); os.write(1,b'WRONG\\n'); sys.stdin.buffer.read()"),
            b"png", &AtomicBool::new(true), Duration::from_secs(5), b"READY\n", b"COMMIT\n",
        ).unwrap_err();
        assert!(error.contains("readiness"));
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
