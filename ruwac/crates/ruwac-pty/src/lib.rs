use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex, mpsc},
    thread,
};
pub struct Pty {
    pub pid: u32,
    master: Box<dyn MasterPty + Send>,
    input: Option<mpsc::SyncSender<Vec<u8>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
}
impl Pty {
    pub fn spawn(
        argv: &[String],
        rows: u16,
        cols: u16,
        mut output: impl FnMut(Vec<u8>) + Send + 'static,
    ) -> Result<Self, String> {
        if argv.is_empty() || argv.iter().any(|s| s.contains('\0')) {
            return Err("argv must be a nonempty NUL-free array".into());
        }
        let pair = portable_pty::native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        let mut cmd = CommandBuilder::new(&argv[0]);
        cmd.args(&argv[1..]);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env_remove("BASH_ENV");
        cmd.env_remove("ENV");
        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        let pid = child.process_id().ok_or("child PID unavailable")?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(64);
        thread::spawn(move || {
            while let Ok(b) = rx.recv() {
                if writer.write_all(&b).and_then(|_| writer.flush()).is_err() {
                    break;
                }
            }
        });
        thread::spawn(move || {
            let mut b = [0; 8192];
            loop {
                match reader.read(&mut b) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => output(b[..n].to_vec()),
                }
            }
            output(Vec::new());
        });
        let child = Arc::new(Mutex::new(child));
        let reaper = child.clone();
        thread::spawn(move || {
            loop {
                if reaper.lock().unwrap().try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        Ok(Self {
            pid,
            master: pair.master,
            input: Some(tx),
            child,
        })
    }
    pub fn send(&self, b: Vec<u8>) -> Result<(), String> {
        self.input
            .as_ref()
            .ok_or("pane closed")?
            .try_send(b)
            .map_err(|e| e.to_string())
    }
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }
    pub fn exited(&self) -> bool {
        self.child
            .lock()
            .unwrap()
            .try_wait()
            .ok()
            .flatten()
            .is_some()
    }
}
impl Drop for Pty {
    fn drop(&mut self) {
        self.input.take();
        let child = self.child.clone();
        let pid = self.pid;
        let fg = self
            .master
            .process_group_leader()
            .filter(|fg| unsafe { libc::getsid(*fg) == pid as i32 && libc::getpgid(*fg) == *fg });
        let mut c = child.lock().unwrap();
        if c.try_wait().ok().flatten().is_none() {
            // Only the process group established by this owned PTY child is signalled.
            unsafe {
                if let Some(fg) = fg {
                    libc::kill(-fg, libc::SIGTERM);
                }
                if libc::getpgid(pid as i32) == pid as i32 {
                    libc::kill(-(pid as i32), libc::SIGTERM);
                } else {
                    let _ = c.kill();
                }
            }
        }
        drop(c);
        thread::spawn(move || {
            for _ in 0..20 {
                if child.lock().unwrap().try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(25));
            }
            let mut c = child.lock().unwrap();
            unsafe {
                if let Some(fg) = fg {
                    if libc::getsid(fg) == pid as i32 && libc::getpgid(fg) == fg {
                        libc::kill(-fg, libc::SIGKILL);
                    }
                }
            }
            if c.try_wait().ok().flatten().is_none() {
                unsafe {
                    if libc::getpgid(pid as i32) == pid as i32 {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                    } else {
                        let _ = c.kill();
                    }
                }
            }
            let _ = c.wait();
        });
    }
}
