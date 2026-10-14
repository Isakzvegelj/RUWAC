//! All potentially blocking RPC and paste delivery run outside the GTK thread.
use ruwac_protocol::{read_frame, request, write_frame};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
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
        let mut result = v["result"].clone();
        if method == "client.render"
            && result["unchanged"] != true
            && result.get("session_name").is_none()
        {
            if let Ok(sessions) = self.call("session.list", json!({})) {
                if let Some(session) = sessions
                    .as_array()
                    .and_then(|rows| rows.iter().find(|row| row["id"] == result["session"]))
                {
                    result["session_name"] = session["name"].clone();
                    result["rows"] = session["rows"].clone();
                    result["cols"] = session["cols"].clone();
                }
            }
            if let Ok(windows) = self.call("window.list", json!({"session":result["session"]})) {
                if let Some(rows) = windows.as_array() {
                    result["window_index"] =
                        json!(rows.iter().position(|row| row["id"] == result["window"]));
                    result["windows"]=json!(rows.iter().enumerate().map(|(index,row)|json!({"id":row["id"],"index":index,"name":row["name"]})).collect::<Vec<_>>());
                }
            }
        }
        Ok(result)
    }
}
fn paste(rpc: &mut Rpc, pane: &str, text: &str, atomic: bool, halt: &AtomicBool) -> io::Result<()> {
    if text.len() > 1024 * 1024 || text.contains('\0') || text.contains("\x1b[201~") {
        return Err(io::Error::other(
            "paste text rejected before input delivery",
        ));
    }
    if halt.load(Ordering::Relaxed) {
        return Err(io::Error::other("paste cancelled before delivery"));
    }
    if !atomic {
        if text.len() > 32 * 1024 {
            return Err(io::Error::other(
                "This daemon cannot accept a single large paste; update the daemon before retrying",
            ));
        }
        rpc.call(
            "client.input",
            json!({"pane":pane,"kind":"paste","text":text}),
        )?;
        return Ok(());
    }
    let metadata = rpc.call(
        "client.paste",
        json!({"operation":"begin","pane":pane,"bytes":text.len()}),
    )?;
    let token = metadata["token"]
        .as_str()
        .ok_or_else(|| io::Error::other("paste token missing"))?;
    let result = (|| -> io::Result<()> {
        let mut offset = 0;
        while offset < text.len() {
            if halt.load(Ordering::Relaxed) {
                return Err(io::Error::other("paste cancelled; no input committed"));
            }
            let mut end = (offset + 32 * 1024).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            rpc.call("client.paste", json!({"operation":"chunk","token":token,"offset":offset,"text":&text[offset..end]}))?;
            offset = end;
        }
        if halt.load(Ordering::Relaxed) {
            return Err(io::Error::other("paste cancelled; no input committed"));
        }
        rpc.call("client.paste", json!({"operation":"commit","token":token}))?;
        Ok(())
    })();
    if result.is_err() {
        // A timed-out commit may have reached the daemon. Never replay it.
        let _ = rpc.call("client.paste", json!({"operation":"abort","token":token}));
    }
    result
}
#[derive(Debug)]
pub enum Command {
    Rpc(String, Value),
    Paste(String, String),
    History(String, bool),
    AttachImage {
        pane: String,
        path: std::path::PathBuf,
        allowed: Arc<AtomicBool>,
        done: SyncSender<Result<Value, String>>,
    },
    Close,
}
impl Command {
    fn bytes(&self) -> usize {
        match self {
            Self::Rpc(_, v) => v.to_string().len(),
            Self::Paste(_, s) => s.len(),
            Self::History(s, _) => s.len(),
            Self::AttachImage { pane, path, .. } => pane.len() + path.as_os_str().len(),
            Self::Close => 0,
        }
    }
}
#[derive(Default)]
pub struct State {
    pub frame: Option<Value>,
    pub error: Option<String>,
    pub reply: VecDeque<(String, Value)>,
    pub acknowledgements: u64,
    pub coalesced: u64,
    pub supports_previous_window: bool,
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
                st.lock().unwrap().supports_previous_window = schema["methods"]
                    .as_array()
                    .is_some_and(|methods| methods.iter().any(|m| m == "window.previous"));
                let capabilities = rpc.call("capabilities", json!({}))?;
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
                            Command::Close => {
                                halt.store(true, Ordering::Relaxed);
                                Ok(("close".into(), json!({})))
                            }
                            Command::AttachImage {
                                pane,
                                path,
                                allowed,
                                done,
                            } => {
                                let native_api = schema["methods"]
                                    .as_array()
                                    .is_some_and(|m| m.iter().any(|m| m == "pane.attachImage"));
                                let result = crate::image_attach::attach(
                                    &mut rpc, &session, &pane, &path, &allowed, native_api,
                                )
                                .map_err(|e| e.to_string());
                                let _ = done.try_send(result.clone());
                                result
                                    .map(|v| ("image-attach".into(), v))
                                    .map_err(io::Error::other)
                            }
                            Command::History(pane, primary) => {
                                let mut text = String::new();
                                let mut offset = 0;
                                let mut total = 0;
                                let mut truncated = false;
                                let mut epoch = None;
                                let mut pin = None;
                                let mut result = (|| -> io::Result<()> {
                                    if primary && capabilities["primary_buffer_history"] != true {
                                        return Err(io::Error::other(
                                            "This daemon does not expose primary history; update it before selecting that buffer",
                                        ));
                                    }

                                    if schema["methods"].as_array().is_some_and(|methods| {
                                        methods.iter().any(|m| m == "pane.history.freeze")
                                    }) {
                                        let metadata = rpc.call(
                                            "pane.history.freeze",
                                            if primary {
                                                json!({"pane":pane,"buffer":"primary"})
                                            } else {
                                                json!({"pane":pane})
                                            },
                                        )?;
                                        pin = Some(
                                            metadata["snapshot"]
                                                .as_str()
                                                .ok_or_else(|| {
                                                    io::Error::other("Copy snapshot ID missing")
                                                })?
                                                .to_owned(),
                                        );
                                        truncated =
                                            metadata["dropped_lines"].as_u64().unwrap_or(0) > 0;
                                    }
                                    loop {
                                        let mut params = json!({"pane":pane,"offset":offset,"lines":500,"epoch":epoch});
                                        if let Some(pin) = &pin {
                                            params["snapshot"] = json!(pin);
                                        }
                                        if primary {
                                            params["buffer"] = json!("primary");
                                        }
                                        let page = rpc.call("pane.history", params)?;
                                        epoch = page["epoch"].as_u64();
                                        let part = page["text"].as_str().unwrap_or("");
                                        let next = page["next_offset"].as_u64().unwrap_or(0);
                                        total = page["total_lines"].as_u64().unwrap_or(0);
                                        if text.len() + part.len() > 8 * 1024 * 1024 {
                                            truncated = true;
                                            break;
                                        }
                                        text.push_str(part);
                                        if next >= total || halt.load(Ordering::Relaxed) {
                                            break;
                                        }
                                        if next <= offset {
                                            return Err(io::Error::other(
                                                "Copy history paging stalled",
                                            ));
                                        }
                                        offset = next;
                                    }
                                    Ok(())
                                })();
                                if let Some(pin) = pin {
                                    let release =
                                        rpc.call("pane.history.release", json!({"snapshot":pin}));
                                    if result.is_ok() {
                                        result = release.map(|_| ());
                                    }
                                }
                                result.map(|()|("pane.history".into(),json!({"pane":pane,"text":text,"total_lines":total,"truncated":truncated,"buffer":if primary {"primary"}else{"active"}})))
                            }
                            Command::Paste(pane, text) => paste(
                                &mut rpc,
                                &pane,
                                &text,
                                capabilities["atomic_paste"] == true,
                                &halt,
                            )
                            .map(|()| ("paste".into(), json!({"bytes":text.len()}))),
                        };
                        budget.fetch_sub(size, Ordering::Relaxed);
                        let mut s = st.lock().unwrap();
                        match result {
                            Ok(reply) => {
                                s.acknowledgements += 1;
                                if reply.0 == "pane.history"
                                    || reply.0.starts_with("window.")
                                    || reply.0 == "pane.create"
                                    || reply.0 == "pane.capture"
                                    || reply.0 == "buffer.get"
                                    || reply.0 == "paste"
                                    || reply.0 == "window.list"
                                {
                                    if s.reply.len() < 32 {
                                        s.reply.push_back(reply);
                                    } else {
                                        s.error = Some(
                                            "Reply queue full; command response unavailable".into(),
                                        );
                                    }
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
        if self.tx.try_send(Command::Close).is_err() {
            self.stop.store(true, Ordering::Relaxed);
            self.state.lock().unwrap().error =
                Some("Detach queue saturated; pending input may be undelivered".into());
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.close();
    }
}
