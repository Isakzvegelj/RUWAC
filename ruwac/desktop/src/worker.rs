//! All potentially blocking RPC and paste delivery run outside the GTK thread.
use ruwac_protocol::{read_frame, request, write_frame};
use serde_json::{Value, json};
use std::{
    fs, io,
    os::unix::{
        fs::{FileTypeExt, MetadataExt},
        io::AsRawFd,
        net::UnixStream,
    },
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, SyncSender},
    },
    thread,
    time::Duration,
};
pub struct Rpc(UnixStream);
impl Rpc {
    pub fn connect(path: &Path) -> io::Result<Self> {
        for (p, mode) in [
            (
                path.parent()
                    .ok_or_else(|| io::Error::other("socket parent missing"))?,
                0o700,
            ),
            (path, 0o600),
        ] {
            let m = fs::symlink_metadata(p)?;
            if m.uid() != unsafe { libc::geteuid() }
                || m.mode() & 0o777 != mode
                || m.file_type().is_symlink()
            {
                return Err(io::Error::other("socket ownership/mode rejected"));
            }
        }
        if !fs::symlink_metadata(path)?.file_type().is_socket() {
            return Err(io::Error::other("not a socket"));
        }
        let s = UnixStream::connect(path)?;
        s.set_read_timeout(Some(Duration::from_secs(2)))?;
        s.set_write_timeout(Some(Duration::from_secs(2)))?;
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut n = std::mem::size_of_val(&cred) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                s.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut cred as *mut _ as *mut _,
                &mut n,
            )
        } != 0
            || cred.uid != unsafe { libc::geteuid() }
        {
            return Err(io::Error::other("server peer UID rejected"));
        }
        Ok(Self(s))
    }
    pub fn call(&mut self, method: &str, params: Value) -> io::Result<Value> {
        let r = request(method, params);
        write_frame(&mut self.0, &r)?;
        let v = read_frame(&mut self.0)?;
        if v["version"] != 0 || v["id"] != r.id {
            return Err(io::Error::other("protocol/correlation rejected"));
        }
        if let Some(e) = v.get("error") {
            return Err(io::Error::other(e.to_string()));
        }
        Ok(v["result"].clone())
    }
}
#[derive(Debug)]
pub enum Command {
    Rpc(String, Value),
    Paste(String, String),
}
impl Command {
    fn bytes(&self) -> usize {
        match self {
            Self::Rpc(_, v) => v.to_string().len(),
            Self::Paste(_, s) => s.len(),
        }
    }
}
#[derive(Default)]
pub struct State {
    pub frame: Option<Value>,
    pub error: Option<String>,
    pub reply: Option<(String, Value)>,
    pub acknowledgements: u64,
    pub coalesced: u64,
}
pub struct Worker {
    tx: SyncSender<Command>,
    pub state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    bytes: Arc<AtomicUsize>,
}
impl Worker {
    pub fn start(path: std::path::PathBuf, session: String) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Command>(128);
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let bytes = Arc::new(AtomicUsize::new(0));
        let (st, halt, budget) = (state.clone(), stop.clone(), bytes.clone());
        thread::spawn(move || {
            let result = (|| -> io::Result<()> {
                let mut rpc = Rpc::connect(&path)?;
                let schema = rpc.call("schema", json!({}))?;
                if schema["protocol"] != 0 {
                    return Err(io::Error::other("unsupported daemon API"));
                }
                rpc.call("capabilities", json!({}))?;
                rpc.call(
                    "session.attach",
                    json!({"session":session,"rows":24,"cols":80}),
                )?;
                let mut revision = None;
                while !halt.load(Ordering::Relaxed) {
                    for _ in 0..16 {
                        let Ok(cmd) = rx.try_recv() else { break };
                        let size = cmd.bytes();
                        let result = match cmd {
                            Command::Rpc(m, p) => rpc.call(&m, p).map(|v| (m, v)),
                            Command::Paste(pane, text) => {
                                let mut rest = text.as_str();
                                let mut sent = 0;
                                let mut err = None;
                                while !rest.is_empty() {
                                    if halt.load(Ordering::Relaxed) {
                                        err = Some(io::Error::other(format!(
                                            "paste cancelled after {sent} bytes"
                                        )));
                                        break;
                                    }
                                    let mut n = rest.len().min(32 * 1024);
                                    while !rest.is_char_boundary(n) {
                                        n -= 1;
                                    }
                                    let chunk = &rest[..n];
                                    if let Err(e) = rpc.call(
                                        "client.input",
                                        json!({"pane":pane,"kind":"paste","text":chunk}),
                                    ) {
                                        err = Some(io::Error::other(format!(
                                            "paste failed after {sent} acknowledged bytes: {e}"
                                        )));
                                        break;
                                    }
                                    rest = &rest[n..];
                                    sent += n;
                                }
                                match err {
                                    Some(e) => Err(e),
                                    None => Ok(("paste".into(), json!({"bytes":sent}))),
                                }
                            }
                        };
                        budget.fetch_sub(size, Ordering::Relaxed);
                        let mut s = st.lock().unwrap();
                        match result {
                            Ok(reply) => {
                                s.acknowledgements += 1;
                                if reply.0 == "pane.capture"
                                    || reply.0 == "buffer.get"
                                    || reply.0 == "paste"
                                    || reply.0 == "window.list"
                                {
                                    s.reply = Some(reply);
                                }
                            }
                            Err(e) => s.error = Some(format!("Input/command delivery failed: {e}")),
                        };
                    }
                    let frame = rpc.call(
                        "client.render",
                        revision.map(|r| json!({"revision":r})).unwrap_or(json!({})),
                    )?;
                    if frame["unchanged"] != true {
                        let r = frame["revision"]
                            .as_u64()
                            .ok_or_else(|| io::Error::other("missing render revision"))?;
                        if revision.is_some_and(|old| r < old) {
                            return Err(io::Error::other("stale render revision"));
                        }
                        revision = Some(r);
                        let mut s = st.lock().unwrap();
                        if s.frame.is_some() {
                            s.coalesced += 1;
                        }
                        s.frame = Some(frame);
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                rpc.call("session.detach", json!({}))?;
                Ok(())
            })();
            if let Err(e) = result {
                st.lock().unwrap().error = Some(format!(
                    "Disconnected: {e}; queued input may be undelivered"
                ));
            }
            halt.store(true, Ordering::Relaxed);
        });
        Self {
            tx,
            state,
            stop,
            bytes,
        }
    }
    pub fn send(&self, cmd: Command) -> Result<(), String> {
        if self.stop.load(Ordering::Relaxed) {
            return Err("View disconnected; input not sent".into());
        }
        let size = cmd.bytes();
        if self
            .bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                n.checked_add(size).filter(|n| *n <= 2 * 1024 * 1024)
            })
            .is_err()
        {
            return Err("Queue byte limit reached; input not sent".into());
        }
        if self.tx.try_send(cmd).is_err() {
            self.bytes.fetch_sub(size, Ordering::Relaxed);
            return Err("Queue full; input not sent".into());
        }
        Ok(())
    }
    pub fn close(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.close();
    }
}
