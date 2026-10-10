use ruwac_protocol::{Error, Request, Result, checked_text, id, target, text};
use ruwac_pty::Pty;
use ruwac_term::Terminal;
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use tokio::sync::{mpsc, oneshot, watch};
struct Pane {
    id: String,
    window: String,
    pty: Pty,
    term: Terminal,
    ended: bool,
}
struct Window {
    id: String,
    session: String,
    name: String,
    panes: Vec<String>,
    active: String,
}
struct Session {
    id: String,
    name: String,
    windows: Vec<String>,
    active: String,
    rows: u16,
    cols: u16,
    buffer: String,
}
struct Client {
    session: String,
    rows: u16,
    cols: u16,
}
pub enum Message {
    Call(Request, String, oneshot::Sender<Result<Value>>),
    Output(String, Vec<u8>),
    Disconnect(String),
}
#[derive(Clone)]
pub struct Handle {
    pub tx: mpsc::Sender<Message>,
    pub revision: watch::Receiver<u64>,
}
impl Handle {
    pub async fn call(&self, r: Request, client: &str) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Message::Call(r, client.into(), tx))
            .await
            .map_err(|_| Error::new("UNAVAILABLE", "daemon stopped"))?;
        rx.await
            .map_err(|_| Error::new("UNAVAILABLE", "daemon stopped"))?
    }
}
struct State {
    panes: HashMap<String, Pane>,
    windows: HashMap<String, Window>,
    sessions: HashMap<String, Session>,
    clients: HashMap<String, Client>,
    tx: mpsc::Sender<Message>,
    cache: HashMap<String, (String, Result<Value>)>,
    order: VecDeque<String>,
    revision: u64,
}
pub fn start() -> Handle {
    let (tx, mut rx) = mpsc::channel(128);
    let (rev, watch) = watch::channel(0);
    let handle = Handle {
        tx: tx.clone(),
        revision: watch,
    };
    tokio::spawn(async move {
        let mut s = State {
            panes: HashMap::new(),
            windows: HashMap::new(),
            sessions: HashMap::new(),
            clients: HashMap::new(),
            tx,
            cache: HashMap::new(),
            order: VecDeque::new(),
            revision: 0,
        };
        while let Some(m) = rx.recv().await {
            match m {
                Message::Call(r, c, reply) => {
                    let answer = s.call(&r, &c);
                    let stop = r.method == "daemon.stop" && answer.is_ok();
                    let _ = reply.send(answer);
                    if !matches!(
                        r.method.as_str(),
                        "health"
                            | "version"
                            | "capabilities"
                            | "schema"
                            | "session.list"
                            | "window.list"
                            | "pane.list"
                            | "pane.capture"
                            | "pane.wait"
                            | "buffer.get"
                            | "client.render"
                    ) {
                        s.revision += 1;
                        let _ = rev.send(s.revision);
                    }
                    if stop {
                        break;
                    }
                }
                Message::Output(p, b) => {
                    if let Some(p) = s.panes.get_mut(&p) {
                        if b.is_empty() {
                            p.ended = true;
                        } else {
                            for reply in p.term.feed(&b) {
                                let _ = p.pty.send(reply.into_bytes());
                            }
                        }
                        s.revision += 1;
                        let _ = rev.send(s.revision);
                    }
                }
                Message::Disconnect(c) => {
                    if let Some(c) = s.clients.remove(&c) {
                        s.layout(&c.session);
                        s.revision += 1;
                        let _ = rev.send(s.revision);
                    }
                }
            }
        }
    });
    handle
}
fn num(v: &Value, k: &str, default: u64, min: u64, max: u64) -> Result<u16> {
    let n = match v.get(k) {
        Some(x) => x
            .as_u64()
            .ok_or_else(|| Error::new("INVALID_PARAMS", format!("{k} must be an integer")))?,
        None => default,
    };
    if n < min || n > max {
        return Err(Error::new("LIMIT", format!("{k} must be {min}..{max}")));
    }
    Ok(n as u16)
}
fn argv(v: &Value) -> Result<Vec<String>> {
    match v.get("argv") {
        None => Ok(vec![
            "/bin/bash".into(),
            "--noprofile".into(),
            "--norc".into(),
        ]),
        Some(Value::Array(a)) if !a.is_empty() && a.len() <= 64 => a
            .iter()
            .map(|x| {
                let s = x
                    .as_str()
                    .ok_or_else(|| Error::new("INVALID_PARAMS", "argv elements must be strings"))?;
                checked_text(s)?;
                Ok(s.into())
            })
            .collect(),
        _ => Err(Error::new(
            "INVALID_PARAMS",
            "argv must be a nonempty string array, max 64",
        )),
    }
}
impl State {
    fn call(&mut self, r: &Request, client: &str) -> Result<Value> {
        if r.version != 0 {
            return Err(Error::new("VERSION", "supported protocol version is 0"));
        }
        if !r.params.is_object() && !r.params.is_null() {
            return Err(Error::new("INVALID_PARAMS", "params must be an object"));
        }
        if let Some(k) = &r.idempotency_key {
            if k.is_empty() || k.len() > 128 {
                return Err(Error::new(
                    "INVALID_PARAMS",
                    "idempotency key must be 1..128 bytes",
                ));
            }
            if matches!(
                r.method.as_str(),
                "session.attach"
                    | "session.detach"
                    | "client.render"
                    | "client.input"
                    | "daemon.stop"
            ) {
                return Err(Error::new(
                    "INVALID_PARAMS",
                    "idempotency unsupported for connection-local operations",
                ));
            }
            let signature = format!("{}:{}", r.method, r.params);
            if let Some((old, result)) = self.cache.get(k) {
                return if *old == signature {
                    result.clone()
                } else {
                    Err(Error::new(
                        "IDEMPOTENCY_CONFLICT",
                        "key reused with different request",
                    ))
                };
            }
            let result = self.dispatch(&r.method, &r.params, client);
            self.cache.insert(k.clone(), (signature, result.clone()));
            self.order.push_back(k.clone());
            if self.order.len() > 1024 {
                if let Some(k) = self.order.pop_front() {
                    self.cache.remove(&k);
                }
            }
            return result;
        }
        self.dispatch(&r.method, &r.params, client)
    }
    fn pane(&self, v: &Value) -> Result<&Pane> {
        let p = target(v, "pane", "pane")?;
        self.panes
            .get(&p)
            .ok_or_else(|| Error::new("STALE_ID", "pane does not exist"))
    }
    fn create_pane(&mut self, w: &str, args: Vec<String>, rows: u16, cols: u16) -> Result<String> {
        if self.panes.len() >= 128 {
            return Err(Error::new("LIMIT", "maximum 128 panes"));
        }
        let pid = id("pane");
        let tx = self.tx.clone();
        let outid = pid.clone();
        let pty = Pty::spawn(&args, rows, cols, move |b| {
            let _ = tx.blocking_send(Message::Output(outid.clone(), b));
        })
        .map_err(|e| Error::new("SPAWN", e))?;
        self.panes.insert(
            pid.clone(),
            Pane {
                id: pid.clone(),
                window: w.into(),
                pty,
                term: Terminal::new(rows, cols),
                ended: false,
            },
        );
        Ok(pid)
    }
    fn create_window(&mut self, s: &str, v: &Value) -> Result<Value> {
        let session = self
            .sessions
            .get(s)
            .ok_or_else(|| Error::new("STALE_ID", "session does not exist"))?;
        let (rows, cols) = (session.rows, session.cols);
        if let Some(name) = v.get("name").and_then(Value::as_str) {
            checked_text(name)?;
        }
        let w = id("window");
        let p = self.create_pane(&w, argv(v)?, rows, cols)?;
        let name = v
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("shell")
            .to_owned();
        self.windows.insert(
            w.clone(),
            Window {
                id: w.clone(),
                session: s.into(),
                name,
                panes: vec![p.clone()],
                active: p.clone(),
            },
        );
        let session = self.sessions.get_mut(s).unwrap();
        session.windows.push(w.clone());
        session.active = w.clone();
        Ok(json!({"window":w,"pane":p,"pid":self.panes[&p].pty.pid}))
    }
    fn layout(&mut self, s: &str) {
        let sizes: Vec<_> = self
            .clients
            .values()
            .filter(|c| c.session == s)
            .map(|c| (c.rows, c.cols))
            .collect();
        let Some(session) = self.sessions.get_mut(s) else {
            return;
        };
        if !sizes.is_empty() {
            session.rows = sizes.iter().map(|s| s.0).min().unwrap();
            session.cols = sizes.iter().map(|s| s.1).min().unwrap();
        }
        let rows = session.rows;
        let cols = session.cols;
        let windows = session.windows.clone();
        for w in windows {
            let Some(w) = self.windows.get(&w) else {
                continue;
            };
            let n = w.panes.len().max(1) as u16;
            for (i, p) in w.panes.iter().enumerate() {
                let h = (rows / n + if i == 0 { rows % n } else { 0 }).max(1);
                if let Some(p) = self.panes.get_mut(p) {
                    if p.term.rows != h || p.term.cols != cols {
                        let _ = p.pty.resize(h, cols);
                        p.term.resize(h, cols)
                    }
                }
            }
        }
    }
    fn dispatch(&mut self, m: &str, v: &Value, client: &str) -> Result<Value> {
        match m {
            "health" => Ok(
                json!({"ok":true,"panes":self.panes.len(),"clients":self.clients.len(),"revision":self.revision}),
            ),
            "version" => Ok(
                json!({"name":"ruwac","implementation":"spike","package":env!("CARGO_PKG_VERSION"),"protocol":0}),
            ),
            "capabilities" => Ok(
                json!({"desktop_clipboard":false,"primary":false,"graphics":false,"image_attachment":false,"ssh_transport":"external authenticated SSH only","shared_focus":true,"split":"horizontal, two panes","capture_limit":2000,"max_text_bytes":ruwac_protocol::MAX_TEXT}),
            ),
            "schema" => Ok(schema()),
            "daemon.stop" => Ok(json!({"stopped":true})),
            "session.create" => {
                if self.sessions.len() >= 64 {
                    return Err(Error::new("LIMIT", "maximum 64 sessions"));
                }
                let s = id("session");
                let name = text(v, "name")?;
                checked_text(&name)?;
                let rows = num(v, "rows", 23, 4, 100)?;
                let cols = num(v, "cols", 80, 10, 200)?;
                self.sessions.insert(
                    s.clone(),
                    Session {
                        id: s.clone(),
                        name,
                        windows: vec![],
                        active: String::new(),
                        rows,
                        cols,
                        buffer: String::new(),
                    },
                );
                match self.create_window(&s, v) {
                    Ok(w) => Ok(
                        json!({"session":s,"window":w["window"],"pane":w["pane"],"pid":w["pid"]}),
                    ),
                    Err(e) => {
                        self.sessions.remove(&s);
                        Err(e)
                    }
                }
            }
            "session.list" => {
                let mut a:Vec<_>=self.sessions.values().map(|s|json!({"id":s.id,"name":s.name,"active_window":s.active,"windows":s.windows,"rows":s.rows,"cols":s.cols})).collect();
                a.sort_by_key(|v| v["id"].as_str().unwrap().to_owned());
                Ok(json!(a))
            }
            "session.attach" => {
                let s = target(v, "session", "session")?;
                if !self.sessions.contains_key(&s) {
                    return Err(Error::new("STALE_ID", "session does not exist"));
                }
                let rows = num(v, "rows", 23, 4, 100)?;
                let cols = num(v, "cols", 80, 10, 200)?;
                if let Some(old) = self.clients.remove(client) {
                    self.layout(&old.session);
                }
                self.clients.insert(
                    client.into(),
                    Client {
                        session: s.clone(),
                        rows,
                        cols,
                    },
                );
                self.layout(&s);
                Ok(json!({"client":client,"session":s}))
            }
            "session.detach" => {
                if let Some(c) = self.clients.remove(client) {
                    self.layout(&c.session);
                }
                Ok(json!({"detached":true}))
            }
            "window.create" => {
                let s = target(v, "session", "session")?;
                self.create_window(&s, v)
            }
            "window.list" => {
                let s = target(v, "session", "session")?;
                let s = self
                    .sessions
                    .get(&s)
                    .ok_or_else(|| Error::new("STALE_ID", "session does not exist"))?;
                Ok(json!(s.windows.iter().filter_map(|w|self.windows.get(w)).map(|w|json!({"id":w.id,"name":w.name,"panes":w.panes,"active_pane":w.active})).collect::<Vec<_>>()))
            }
            "window.rename" => {
                let w = target(v, "window", "window")?;
                let name = text(v, "name")?;
                checked_text(&name)?;
                let w = self
                    .windows
                    .get_mut(&w)
                    .ok_or_else(|| Error::new("STALE_ID", "window does not exist"))?;
                w.name = name;
                Ok(json!({"renamed":true}))
            }
            "window.select" => {
                let w = target(v, "window", "window")?;
                let w = self
                    .windows
                    .get(&w)
                    .ok_or_else(|| Error::new("STALE_ID", "window does not exist"))?;
                self.sessions.get_mut(&w.session).unwrap().active = w.id.clone();
                Ok(json!({"selected":w.id}))
            }
            "window.cycle" => {
                let s = target(v, "session", "session")?;
                let s = self
                    .sessions
                    .get_mut(&s)
                    .ok_or_else(|| Error::new("STALE_ID", "session does not exist"))?;
                let i = s.windows.iter().position(|w| w == &s.active).unwrap_or(0);
                let n = s.windows.len();
                let j = if v.get("previous").and_then(Value::as_bool).unwrap_or(false) {
                    (i + n - 1) % n
                } else {
                    (i + 1) % n
                };
                s.active = s.windows[j].clone();
                Ok(json!({"selected":s.active}))
            }
            "pane.create" => {
                let w = target(v, "window", "window")?;
                let window = self
                    .windows
                    .get(&w)
                    .ok_or_else(|| Error::new("STALE_ID", "window does not exist"))?;
                if window.panes.len() >= 2 {
                    return Err(Error::new("LIMIT", "spike allows two panes per window"));
                }
                let s = window.session.clone();
                let ss = &self.sessions[&s];
                let p = self.create_pane(&w, argv(v)?, ss.rows / 2, ss.cols)?;
                let w = self.windows.get_mut(&w).unwrap();
                w.panes.push(p.clone());
                w.active = p.clone();
                self.layout(&s);
                Ok(json!({"pane":p,"pid":self.panes[&p].pty.pid}))
            }
            "pane.list" => {
                let s = target(v, "session", "session")?;
                let s = self
                    .sessions
                    .get(&s)
                    .ok_or_else(|| Error::new("STALE_ID", "session does not exist"))?;
                Ok(json!(s.windows.iter().filter_map(|w|self.windows.get(w)).flat_map(|w|w.panes.iter()).filter_map(|p|self.panes.get(p)).map(|p|json!({"id":p.id,"window":p.window,"pid":p.pty.pid,"ended":p.ended,"rows":p.term.rows,"cols":p.term.cols})).collect::<Vec<_>>()))
            }
            "pane.close" => {
                let p = self.pane(v)?;
                let pid = p.id.clone();
                let w = p.window.clone();
                let session = self.windows[&w].session.clone();
                self.panes.remove(&pid);
                let window = self.windows.get_mut(&w).unwrap();
                window.panes.retain(|p| p != &pid);
                if let Some(first) = window.panes.first() {
                    if window.active == pid {
                        window.active = first.clone();
                    }
                } else {
                    self.windows.remove(&w);
                    let ss = self.sessions.get_mut(&session).unwrap();
                    ss.windows.retain(|x| x != &w);
                    if let Some(first) = ss.windows.first() {
                        if ss.active == w {
                            ss.active = first.clone();
                        }
                    } else {
                        self.sessions.remove(&session);
                        self.clients.retain(|_, c| c.session != session);
                    }
                }
                self.layout(&session);
                Ok(json!({"closed":pid}))
            }
            "pane.select" => {
                let p = self.pane(v)?;
                let id = p.id.clone();
                let w = p.window.clone();
                self.windows.get_mut(&w).unwrap().active = id.clone();
                Ok(json!({"selected":id}))
            }
            "pane.capture" | "pane.wait" => {
                let p = self.pane(v)?;
                let n = num(v, "lines", 250, 1, 2000)?;
                let t = p.term.capture(n as usize);
                let matched = v
                    .get("contains")
                    .and_then(Value::as_str)
                    .map(|s| t.contains(s));
                Ok(
                    json!({"pane":p.id,"text":t,"lines":t.lines().count(),"matched":matched,"ended":p.ended}),
                )
            }
            "pane.sendText" => {
                let t = text(v, "text")?;
                checked_text(&t)?;
                let p = self.pane(v)?;
                if p.ended {
                    return Err(Error::new("PANE_EXITED", "child exited"));
                }
                p.pty
                    .send(t.as_bytes().to_vec())
                    .map_err(|e| Error::new("BACKPRESSURE", e))?;
                Ok(json!({"queued_bytes":t.len()}))
            }
            "pane.sendKey" => {
                let key = text(v, "key")?;
                let b = match key.as_str() {
                    "Enter" => b"\r".to_vec(),
                    "Tab" => b"\t".to_vec(),
                    "Escape" => b"\x1b".to_vec(),
                    "CtrlA" => vec![1],
                    "CtrlC" => vec![3],
                    "Up" => b"\x1b[A".to_vec(),
                    "Down" => b"\x1b[B".to_vec(),
                    _ => {
                        return Err(Error::new(
                            "INVALID_KEY",
                            "Enter, Tab, Escape, CtrlA, CtrlC, Up, Down supported",
                        ));
                    }
                };
                let p = self.pane(v)?;
                if p.ended {
                    return Err(Error::new("PANE_EXITED", "child exited"));
                }
                p.pty.send(b).map_err(|e| Error::new("BACKPRESSURE", e))?;
                Ok(json!({"queued":true}))
            }
            "buffer.set" => {
                let s = target(v, "session", "session")?;
                let t = text(v, "text")?;
                checked_text(&t)?;
                let s = self
                    .sessions
                    .get_mut(&s)
                    .ok_or_else(|| Error::new("STALE_ID", "session does not exist"))?;
                s.buffer = t;
                Ok(json!({"stored":true}))
            }
            "buffer.get" => {
                let s = target(v, "session", "session")?;
                let s = self
                    .sessions
                    .get(&s)
                    .ok_or_else(|| Error::new("STALE_ID", "session does not exist"))?;
                Ok(json!({"text":s.buffer}))
            }
            "buffer.paste" => {
                let p = self.pane(v)?;
                let w = &self.windows[&p.window];
                let t = self.sessions[&w.session].buffer.clone();
                let mut b = t.into_bytes();
                if p.term.paste() {
                    let mut framed = b"\x1b[200~".to_vec();
                    framed.extend(b);
                    framed.extend(b"\x1b[201~");
                    b = framed;
                }
                p.pty.send(b).map_err(|e| Error::new("BACKPRESSURE", e))?;
                Ok(json!({"queued":true}))
            }
            "client.resize" => {
                let rows = num(v, "rows", 23, 4, 100)?;
                let cols = num(v, "cols", 80, 10, 200)?;
                let c = self
                    .clients
                    .get_mut(client)
                    .ok_or_else(|| Error::new("NOT_ATTACHED", "attach first"))?;
                c.rows = rows;
                c.cols = cols;
                let s = c.session.clone();
                self.layout(&s);
                Ok(json!({"resized":true}))
            }
            "client.render" => {
                let c = self
                    .clients
                    .get(client)
                    .ok_or_else(|| Error::new("NOT_ATTACHED", "attach first"))?;
                let s = &self.sessions[&c.session];
                let w = &self.windows[&s.active];
                if v.get("revision").and_then(Value::as_u64) == Some(self.revision) {
                    return Ok(json!({"revision":self.revision,"unchanged":true}));
                }
                let panes = w
                    .panes
                    .iter()
                    .filter_map(|p| self.panes.get(p))
                    .map(|p| json!({"id":p.id,"grid":p.term.snapshot(),"ended":p.ended}))
                    .collect::<Vec<_>>();
                Ok(
                    json!({"revision":self.revision,"session":s.id,"window":w.id,"name":w.name,"active_pane":w.active,"panes":panes}),
                )
            }
            "client.input" => {
                let p = self.pane(v)?;
                let c = self
                    .clients
                    .get(client)
                    .ok_or_else(|| Error::new("NOT_ATTACHED", "attach first"))?;
                if self.windows[&p.window].session != c.session {
                    return Err(Error::new(
                        "TARGET_SESSION",
                        "pane belongs to another session",
                    ));
                }
                let kind = text(v, "kind")?;
                let t = text(v, "text")?;
                checked_text(&t)?;
                let b = match kind.as_str() {
                    "mouse" if !p.term.mouse() => return Ok(json!({"discarded":"mouse disabled"})),
                    "mouse" | "keyboard" => t.into_bytes(),
                    "paste" => {
                        if t.contains("\x1b[201~") {
                            return Err(Error::new(
                                "INVALID_TEXT",
                                "paste contains embedded end marker",
                            ));
                        }
                        if p.term.paste() {
                            format!("\x1b[200~{t}\x1b[201~").into_bytes()
                        } else {
                            t.into_bytes()
                        }
                    }
                    _ => {
                        return Err(Error::new(
                            "INVALID_PARAMS",
                            "kind must be keyboard, mouse or paste",
                        ));
                    }
                };
                if p.ended {
                    return Err(Error::new("PANE_EXITED", "child exited"));
                }
                p.pty.send(b).map_err(|e| Error::new("BACKPRESSURE", e))?;
                Ok(json!({"queued":true}))
            }
            _ => Err(Error::new(
                "METHOD_NOT_FOUND",
                format!("unknown method {m}"),
            )),
        }
    }
}
pub fn schema() -> Value {
    json!({"protocol":0,"framing":"u32 big-endian byte length + UTF-8 JSON","max_frame_bytes":ruwac_protocol::MAX_FRAME,"methods":["health","version","capabilities","schema","session.create","session.list","session.attach","session.detach","window.create","window.list","window.rename","window.select","window.cycle","pane.create","pane.close","pane.list","pane.select","pane.sendText","pane.sendKey","pane.capture","pane.wait","buffer.set","buffer.get","buffer.paste","client.resize","client.render","client.input","events.subscribe","daemon.stop"],"targets":{"session":"session_ + 32 hex","window":"window_ + 32 hex","pane":"pane_ + 32 hex"},"request":{"version":0,"id":"JSON request correlation value","method":"method name","params":"object","idempotency_key":"optional 1..128 byte daemon-lifetime key"}})
}
