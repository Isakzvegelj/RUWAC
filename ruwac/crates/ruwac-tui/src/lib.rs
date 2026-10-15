mod copy;
use ruwac_protocol::{MAX_TEXT, read_frame, request, write_frame};
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn signal(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}
#[derive(Debug, PartialEq)]
pub enum Input {
    Keyboard(Vec<u8>),
    Mouse(Vec<u8>),
    Paste(Vec<u8>),
    Rejected(&'static str),
}
#[derive(Default)]
pub struct Decoder {
    buf: Vec<u8>,
    paste: Option<Vec<u8>>,
    discard_paste: bool,
}
impl Decoder {
    pub fn feed(&mut self, b: &[u8]) -> Vec<Input> {
        self.buf.extend_from_slice(b);
        let mut out = Vec::new();
        loop {
            if self.paste.is_some() {
                if let Some(i) = find(&self.buf, b"\x1b[201~") {
                    let mut p = self.paste.take().unwrap();
                    if !self.discard_paste {
                        p.extend_from_slice(&self.buf[..i]);
                    }
                    self.buf.drain(..i + 6);
                    if self.discard_paste {
                        self.discard_paste = false;
                        continue;
                    }
                    if p.len() > MAX_TEXT {
                        out.push(Input::Rejected("paste exceeds 256 KiB"))
                    } else {
                        out.push(Input::Paste(p))
                    }
                    continue;
                }
                let n = self.buf.len().saturating_sub(5);
                if self.discard_paste {
                    self.buf.drain(..n);
                    break;
                }
                if self.paste.as_ref().unwrap().len() + n > MAX_TEXT {
                    self.paste = Some(Vec::new());
                    self.discard_paste = true;
                    self.buf.drain(..n);
                    out.push(Input::Rejected("paste exceeds 256 KiB"));
                    break;
                }
                self.paste.as_mut().unwrap().extend(self.buf.drain(..n));
                break;
            }
            if self.buf.is_empty() {
                break;
            }
            if self.buf.starts_with(b"\x1b[200~") {
                self.buf.drain(..6);
                self.paste = Some(Vec::new());
                continue;
            }
            if self.buf[0] != 27 {
                let n = self
                    .buf
                    .iter()
                    .position(|b| *b == 27)
                    .unwrap_or(self.buf.len());
                let b = &self.buf[..n];
                let valid = match std::str::from_utf8(b) {
                    Ok(_) => n,
                    Err(e) if e.error_len().is_none() => e.valid_up_to(),
                    Err(_) => {
                        self.buf.drain(..n);
                        out.push(Input::Rejected("input is not UTF-8"));
                        continue;
                    }
                };
                if valid == 0 {
                    break;
                }
                out.push(Input::Keyboard(self.buf.drain(..valid).collect()));
                continue;
            }
            if self.buf.len() == 1 {
                break;
            }
            if self.buf[1] == b'[' {
                if self.buf.len() < 3 {
                    break;
                }
                let end = self.buf[2..]
                    .iter()
                    .position(|b| (0x40..=0x7e).contains(b))
                    .map(|n| n + 3);
                let Some(mut n) = end else {
                    if self.buf.len() > 4096 {
                        out.push(Input::Keyboard(self.buf.drain(..).collect()));
                    }
                    break;
                };
                if self.buf[2] == b'M' {
                    if self.buf.len() < 6 {
                        break;
                    }
                    n = 6;
                }
                let b: Vec<_> = self.buf.drain(..n).collect();
                let mouse = b.starts_with(b"\x1b[<") && matches!(b.last(), Some(b'M' | b'm'))
                    || b.starts_with(b"\x1b[M");
                let cap = b.last() == Some(&b'c')
                    && (b.starts_with(b"\x1b[?") || b.starts_with(b"\x1b[>"))
                    && b[3..b.len() - 1]
                        .iter()
                        .all(|x| x.is_ascii_digit() || *x == b';');
                if mouse {
                    out.push(Input::Mouse(b))
                } else if !cap {
                    out.push(Input::Keyboard(b))
                }
                continue;
            }
            if self.buf[1] == b'O' {
                if self.buf.len() < 3 {
                    break;
                }
                out.push(Input::Keyboard(self.buf.drain(..3).collect()));
                continue;
            }
            if self.buf[1] == b']' {
                let end = self
                    .buf
                    .iter()
                    .position(|b| *b == 7)
                    .map(|i| i + 1)
                    .or_else(|| find(&self.buf, b"\x1b\\").map(|i| i + 2));
                let Some(n) = end else {
                    if self.buf.len() > 4096 {
                        out.push(Input::Keyboard(self.buf.drain(..).collect()));
                    }
                    break;
                };
                let b: Vec<_> = self.buf.drain(..n).collect();
                let cap = (b.starts_with(b"\x1b]10;rgb:") || b.starts_with(b"\x1b]11;rgb:"))
                    && b.iter().all(|x| x.is_ascii());
                if !cap {
                    out.push(Input::Keyboard(b))
                }
                continue;
            }
            out.push(Input::Keyboard(self.buf.drain(..2).collect()));
        }
        out
    }
    pub fn escape_timeout(&mut self) -> Vec<Input> {
        if self.paste.is_none() && self.buf == [27] {
            self.buf.clear();
            vec![Input::Keyboard(vec![27])]
        } else {
            vec![]
        }
    }
}
fn find(b: &[u8], needle: &[u8]) -> Option<usize> {
    b.windows(needle.len()).position(|x| x == needle)
}
struct Raw {
    term: libc::termios,
    term_handler: libc::sighandler_t,
    int_handler: libc::sighandler_t,
}
impl Raw {
    fn enter() -> io::Result<Self> {
        unsafe {
            if libc::isatty(0) != 1 || libc::isatty(1) != 1 {
                return Err(io::Error::other("attach requires a terminal"));
            }
            let mut old = std::mem::zeroed();
            if libc::tcgetattr(0, &mut old) != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut raw = old;
            libc::cfmakeraw(&mut raw);
            if libc::tcsetattr(0, libc::TCSANOW, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
            let th = libc::signal(libc::SIGTERM, signal as *const () as libc::sighandler_t);
            let ih = libc::signal(libc::SIGINT, signal as *const () as libc::sighandler_t);
            let guard = Self {
                term: old,
                term_handler: th,
                int_handler: ih,
            };
            io::stdout().write_all(b"\x1b[22;0t\x1b[?1049h\x1b[?2004h\x1b[?25l")?;
            io::stdout().flush()?;
            Ok(guard)
        }
    }
}
impl Drop for Raw {
    fn drop(&mut self) {
        let _=io::stdout().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1004l\x1b[?2004l\x1b[?25h\x1b[0m\x1b[?1049l\x1b[23;0t");
        let _ = io::stdout().flush();
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.term);
            libc::signal(libc::SIGTERM, self.term_handler);
            libc::signal(libc::SIGINT, self.int_handler);
        }
    }
}
fn terminal_dimensions() -> (u16, u16) {
    unsafe {
        let mut s: libc::winsize = std::mem::zeroed();
        if libc::ioctl(0, libc::TIOCGWINSZ, &mut s) != 0 {
            return (24, 80);
        }
        (
            if s.ws_row == 0 { 24 } else { s.ws_row },
            if s.ws_col == 0 { 80 } else { s.ws_col },
        )
    }
}
fn dimensions() -> (u16, u16) {
    let (rows, cols) = terminal_dimensions();
    (rows.saturating_sub(1).clamp(4, 100), cols.clamp(10, 200))
}

fn raw_call(s: &mut UnixStream, m: &str, v: Value) -> io::Result<Value> {
    let sent = request(m, v);
    write_frame(s, &sent)?;
    let r = read_frame(s)?;
    if r["version"] != 0 || r["id"] != sent.id {
        return Err(io::Error::other("Protocol/correlation rejected"));
    }
    if let Some(e) = r.get("error") {
        return Err(io::Error::other(e.to_string()));
    }
    Ok(r["result"].clone())
}
fn call(s: &mut UnixStream, m: &str, v: Value) -> io::Result<Value> {
    let mut result = ruwac_protocol::client_call(m, v, |m, v| raw_call(s, m, v))?;
    if m == "client.render" && result["unchanged"] != true && result.get("session_name").is_none() {
        if let Ok(sessions) = call(s, "session.list", json!({})) {
            if let Some(session) = sessions
                .as_array()
                .and_then(|rows| rows.iter().find(|row| row["id"] == result["session"]))
            {
                result["session_name"] = session["name"].clone();
                result["rows"] = session["rows"].clone();
                result["cols"] = session["cols"].clone();
            }
        }
        if let Ok(windows) = call(s, "window.list", json!({"session":result["session"]})) {
            if let Some(rows) = windows.as_array() {
                result["window_index"] =
                    json!(rows.iter().position(|row| row["id"] == result["window"]));
                result["windows"] = json!(
                    rows.iter()
                        .enumerate()
                        .map(
                            |(index, row)| json!({"id":row["id"],"index":index,"name":row["name"]})
                        )
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    Ok(result)
}
fn sgr(c: &Value, fg: bool) -> String {
    let base = if fg { 38 } else { 48 };
    if let Some(a) = c.as_array() {
        format!("{base};2;{};{};{}", a[0], a[1], a[2])
    } else {
        let n = c.as_u64().unwrap_or(256);
        if n < 8 {
            format!("{}", if fg { 30 } else { 40 } + n)
        } else if n < 16 {
            format!("{}", if fg { 90 } else { 100 } + n - 8)
        } else if n < 256 {
            format!("{base};5;{n}")
        } else if n >= 259 && n <= 266 {
            format!("{base};5;{}", n - 259)
        } else {
            if fg { "39" } else { "49" }.into()
        }
    }
}
fn safe(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}
fn window_identity(v: &Value) -> String {
    format!(
        "{} · {}:{}",
        safe(v["session_name"].as_str().unwrap_or("session")),
        v["window_index"].as_u64().unwrap_or(0),
        safe(v["name"].as_str().unwrap_or("shell"))
    )
}
fn status_line(v: &Value, size: (u16, u16), status: &str) -> String {
    let windows = v["windows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|w| {
                    format!(
                        "{}:{}{}",
                        w["index"].as_u64().unwrap_or(0),
                        safe(w["name"].as_str().unwrap_or("shell")),
                        if w["id"] == v["window"] { "*" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let rows = v["rows"].as_u64().unwrap_or(size.0 as u64);
    let cols = v["cols"].as_u64().unwrap_or(size.1 as u64);
    let shared = if rows != size.0 as u64 || cols != size.1 as u64 {
        format!(" · shared PTY {cols}×{rows}; view {}×{}", size.1, size.0)
    } else {
        String::new()
    };
    let line = if status.trim().is_empty() {
        format!("RUWAC {}{} · {}", window_identity(v), shared, windows)
    } else {
        format!(
            "RUWAC {} · {}{} · {}",
            copy::clip(&window_identity(v), (size.1 as usize / 3).max(1)),
            safe(status),
            shared,
            windows
        )
    };
    copy::clip(&line, size.1 as usize)
}

fn status_window_at(v: &Value, size: (u16, u16), status: &str, column: usize) -> Option<String> {
    let rows = v["windows"].as_array()?;
    let labels: Vec<String> = rows
        .iter()
        .map(|w| {
            format!(
                "{}:{}{}",
                w["index"].as_u64().unwrap_or(0),
                safe(w["name"].as_str().unwrap_or("shell")),
                if w["id"] == v["window"] { "*" } else { "" }
            )
        })
        .collect();
    let joined = labels.join(" ");
    if joined.is_empty() {
        return None;
    }
    let line = status_line(v, size, status);
    let at = line.rfind(&joined)?;
    let mut left = copy::width(&line[..at]);
    for (row, label) in rows.iter().zip(labels) {
        let right = left + copy::width(&label);
        if left <= column && column < right {
            return row["id"].as_str().map(str::to_owned);
        }
        left = right + 1;
    }
    None
}
fn status_mouse(
    v: &Value,
    bytes: &[u8],
    screen: (u16, u16),
    status: &str,
) -> (bool, Option<String>) {
    let Some(text) = std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.strip_prefix("\x1b[<"))
    else {
        return (false, None);
    };
    let Some(last) = text
        .as_bytes()
        .last()
        .copied()
        .filter(|b| matches!(b, b'M' | b'm'))
    else {
        return (false, None);
    };
    let parts: Vec<_> = text[..text.len() - 1]
        .split(';')
        .map(str::parse::<usize>)
        .collect();
    let [Ok(button), Ok(x), Ok(y)] = parts.as_slice() else {
        return (false, None);
    };
    if *y != screen.0 as usize {
        return (false, None);
    }
    let target = if last == b'M' && *button == 0 && *x > 0 {
        status_window_at(v, (screen.0.saturating_sub(1), screen.1), status, x - 1)
    } else {
        None
    };
    (true, target)
}

fn draw(v: &Value, status: &str) -> io::Result<()> {
    let mut out = format!(
        "\x1b]2;RUWAC {}\x07\x1b[?25l\x1b[0m\x1b[H\x1b[2J",
        window_identity(v).chars().take(160).collect::<String>()
    );

    let active = v["active_pane"].as_str().unwrap_or("");
    let mut cursor = None;
    if let Some(panes) = v["panes"].as_array() {
        for p in panes {
            let g = &p["grid"];
            let offset = p["rect"]["y"].as_u64().unwrap_or(0);
            let left = p["rect"]["x"].as_u64().unwrap_or(0);
            if p["id"] == active {
                let c = &g["cursor"];
                if c["visible"] == true {
                    cursor = Some((
                        offset + c["row"].as_u64().unwrap_or(0),
                        left + c["col"].as_u64().unwrap_or(0),
                    ))
                }
            }
            if let Some(rows) = g["cells"].as_array() {
                for (r, cells) in rows.iter().enumerate() {
                    out.push_str(&format!("\x1b[{};{}H", offset + r as u64 + 1, left + 1));
                    let mut style = String::new();
                    for cell in cells.as_array().unwrap() {
                        if cell[1] == true {
                            continue;
                        }
                        let st = format!(
                            "0;{};{};{};{}",
                            sgr(&cell[2], true),
                            sgr(&cell[3], false),
                            if cell[4] == true { 1 } else { 22 },
                            if cell[5] == true { 7 } else { 27 }
                        );
                        let mut st = st;
                        let extra = &cell[6];
                        if extra["italic"] == true {
                            st.push_str(";3");
                        }
                        if extra["dim"] == true {
                            st.push_str(";2");
                        }
                        if extra["hidden"] == true {
                            st.push_str(";8");
                        }
                        if extra["strike"] == true {
                            st.push_str(";9");
                        }
                        match extra["underline"].as_str() {
                            Some("single") => st.push_str(";4"),
                            Some("double") => st.push_str(";4:2"),
                            Some("curly") => st.push_str(";4:3"),
                            Some("dotted") => st.push_str(";4:4"),
                            Some("dashed") => st.push_str(";4:5"),
                            _ => (),
                        }
                        if let Some(rgb) = extra["underline_color"].as_array() {
                            st.push_str(&format!(";58;2;{};{};{}", rgb[0], rgb[1], rgb[2]));
                        } else if let Some(index) =
                            extra["underline_color"].as_u64().filter(|i| *i <= 255)
                        {
                            st.push_str(&format!(";58;5;{index}"));
                        }
                        if st != style {
                            out.push_str(&format!("\x1b[{st}m"));
                            style = st;
                        }
                        out.push_str(&safe(cell[0].as_str().unwrap_or(" ")));
                    }
                }
            }
        }
    }
    // The daemon bounds the PTY grid, but chrome belongs to the physical
    // terminal. Keep the bar at its bottom even beyond the engine grid limits.
    let (screen_rows, screen_cols) = terminal_dimensions();
    let line = status_line(v, (screen_rows.saturating_sub(1), screen_cols), status);
    out.push_str(&format!(
        "\x1b[{};1H\x1b[0;7m{}{}\x1b[0m",
        screen_rows,
        line,
        " ".repeat((screen_cols as usize).saturating_sub(copy::width(&line)))
    ));
    out.push_str("\x1b[?1000h\x1b[?1006h");
    if let Some((r, c)) = cursor {
        out.push_str(&format!("\x1b[{};{}H\x1b[?25h", r + 1, c + 1));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(out.as_bytes())?;
    stdout.flush()
}
#[derive(Default)]
struct Config {
    prefix: u8,
    remaps: std::collections::HashMap<char, char>,
    double_prefix_previous: bool,
}
impl Config {
    fn load() -> io::Result<Self> {
        let mut c = Self {
            prefix: 1,
            ..Self::default()
        };
        if let Ok(value) = std::env::var("RUWAC_SHORTCUTS") {
            let map: std::collections::HashMap<String, String> =
                serde_json::from_str(&value).map_err(io::Error::other)?;
            if let Some(action) = map.get("double_prefix") {
                c.double_prefix_previous = match action.as_str() {
                    "literal" => false,
                    "previous" => true,
                    _ => {
                        return Err(io::Error::other(
                            "double_prefix must be literal or previous",
                        ));
                    }
                };
            }
            if let Some(prefix) = map.get("prefix") {
                let letter = prefix
                    .strip_prefix("<Control>")
                    .or_else(|| prefix.strip_prefix("Ctrl+"))
                    .or_else(|| prefix.strip_prefix("Ctrl-"))
                    .or_else(|| prefix.strip_prefix('^'));
                c.prefix = letter
                    .filter(|s| s.len() == 1 && s.as_bytes()[0].is_ascii_alphabetic())
                    .map(|s| s.as_bytes()[0].to_ascii_lowercase() & 31)
                    .ok_or_else(|| {
                        io::Error::other("TUI prefix must be Control+A through Control+Z")
                    })?;
            }
        }
        if let Ok(value) = std::env::var("RUWAC_KEYMAP") {
            let map: std::collections::HashMap<String, String> =
                serde_json::from_str(&value).map_err(io::Error::other)?;
            for (from, to) in map {
                let from = if from == "Tab" {
                    Some('\t')
                } else {
                    from.chars().next().filter(|_| from.len() == 1)
                };
                if let Some(from) = from {
                    let to = if to == "Tab" {
                        '\t'
                    } else {
                        to.chars().next().filter(|_| to.len() == 1).ok_or_else(|| {
                            io::Error::other("TUI remaps require one implemented prefix command")
                        })?
                    };
                    if !"acnpodS|\t[]\"?:z0123456789".contains(to) {
                        return Err(io::Error::other("Unsupported TUI prefix remap"));
                    }
                    c.remaps.insert(from, to);
                }
            }
        }
        Ok(c)
    }
    fn label(&self) -> String {
        format!("^{}", (self.prefix + b'@') as char)
    }
}
struct WindowRow {
    id: String,
    name: String,
}
enum Mode {
    Live,
    Copy(copy::Copy),
    Chooser {
        rows: Vec<WindowRow>,
        index: usize,
        typed: String,
    },
    Prompt(String),
    Confirm {
        panes: Vec<String>,
        label: String,
    },
    Help(usize),
}
fn history(s: &mut UnixStream, pane: &str) -> io::Result<copy::Copy> {
    let metadata = call(s, "pane.history.freeze", json!({"pane":pane}))?;
    let pin = metadata["snapshot"]
        .as_str()
        .ok_or_else(|| io::Error::other("History snapshot ID missing"))?
        .to_owned();
    let result = (|| -> io::Result<copy::Copy> {
        let total = metadata["total_lines"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= 50000)
            .ok_or_else(|| io::Error::other("History snapshot row cap rejected"))?
            as usize;
        let cols = metadata["cols"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= 200)
            .ok_or_else(|| io::Error::other("History columns rejected"))?
            as usize;
        let mut lines = Vec::new();
        let mut offset = 0;
        let mut bytes = 0;
        while offset < total {
            let page = call(
                s,
                "pane.history",
                json!({"pane":pane,"snapshot":pin,"offset":offset,"lines":500}),
            )?;
            if page["epoch"] != metadata["epoch"] || page["total_lines"] != metadata["total_lines"]
            {
                return Err(io::Error::other("Inconsistent history snapshot rejected"));
            }
            let rows = page["rows"].as_array().ok_or_else(|| {
                io::Error::other("Canonical history rows unavailable; use a matching daemon")
            })?;
            for row in rows {
                let text = row["text"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("Invalid canonical row"))?;
                if text.chars().any(char::is_control) {
                    return Err(io::Error::other("Canonical row controls rejected"));
                }
                bytes += text.len() + 1;
                if bytes > 8 * 1024 * 1024 {
                    return Err(io::Error::other("Copy view exceeds 8 MiB"));
                }
                let wrapped = row["wrapped"]
                    .as_bool()
                    .ok_or_else(|| io::Error::other("Canonical wrap flag missing"))?;
                lines.push(copy::Line {
                    text: text.into(),
                    wrapped,
                });
            }
            let next = page["next_offset"].as_u64().unwrap_or(0) as usize;
            if next != offset + rows.len() || next <= offset || next > total {
                return Err(io::Error::other("History paging failed"));
            }
            offset = next;
        }
        Ok(copy::Copy::new(
            lines,
            cols,
            copy::Position {
                row: metadata["cursor_row"].as_u64().unwrap_or(0) as usize,
                col: metadata["cursor_col"].as_u64().unwrap_or(0) as usize,
            },
            metadata["dropped_lines"].as_u64().unwrap_or(0) as usize,
            pane.into(),
        ))
    })();
    let release = call(s, "pane.history.release", json!({"snapshot":pin}));
    match result {
        Ok(value) => {
            release?;
            Ok(value)
        }
        Err(e) => Err(e),
    }
}
fn windows(s: &mut UnixStream, session: &str, current: &Value) -> io::Result<Mode> {
    let v = call(s, "window.list", json!({"session":session}))?;
    let data = v
        .as_array()
        .ok_or_else(|| io::Error::other("Window list missing"))?;
    if data.is_empty() || data.len() > 128 {
        return Err(io::Error::other("Window chooser bound rejected"));
    }
    let mut rows = Vec::new();
    for row in data {
        let id = row["id"]
            .as_str()
            .filter(|id| ruwac_protocol::valid_id(id, "window"))
            .ok_or_else(|| io::Error::other("Window ID rejected"))?;
        rows.push(WindowRow {
            id: id.into(),
            name: safe(row["name"].as_str().unwrap_or("Window")),
        });
    }
    let index = rows
        .iter()
        .position(|row| current.as_str() == Some(&row.id))
        .unwrap_or(0);
    Ok(Mode::Chooser {
        rows,
        index,
        typed: String::new(),
    })
}
fn plain(s: &mut UnixStream, v: &Value, text: &mut String) -> io::Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    call(
        s,
        "client.input",
        json!({"pane":v["active_pane"],"kind":"keyboard","text":std::mem::take(text)}),
    )?;
    Ok(())
}
fn command(
    s: &mut UnixStream,
    session: &str,
    snapshot: &Value,
    text: &str,
    mode: &mut Mode,
) -> io::Result<()> {
    let trimmed = text.trim();
    let (name, arg) = trimmed
        .split_once(char::is_whitespace)
        .unwrap_or((trimmed, ""));
    let name = ruwac_protocol::screen_command(name);
    let arg = if name == "split" && arg.trim() == "-v" {
        "vertical"
    } else if name == "split" && arg.trim().is_empty() {
        "horizontal"
    } else {
        arg.trim()
    };
    if !arg.is_empty() && !matches!(name, "select" | "rename" | "split") {
        return Err(io::Error::other("This command takes no arguments"));
    }
    match name {
        "" => (),
        "new" => {
            call(s, "window.create", json!({"session":session}))?;
        }
        "next" | "prev" => {
            call(
                s,
                "window.cycle",
                json!({"session":session,"previous":name=="prev"}),
            )?;
        }
        "other" => {
            call(s, "window.previous", json!({"session":session}))?;
        }
        "select" => {
            let index: usize = arg
                .parse()
                .map_err(|_| io::Error::other("select requires a window index"))?;
            let rows = call(s, "window.list", json!({"session":session}))?;
            let row = rows
                .as_array()
                .and_then(|a| a.get(index))
                .ok_or_else(|| io::Error::other("Window index out of range"))?;
            call(s, "window.select", json!({"window":row["id"]}))?;
        }
        "rename" => {
            if arg.is_empty() {
                return Err(io::Error::other("rename requires a name"));
            }
            call(
                s,
                "window.rename",
                json!({"window":snapshot["window"],"name":arg}),
            )?;
        }
        "split" => {
            if !matches!(arg, "horizontal" | "vertical") {
                return Err(io::Error::other("split requires horizontal or vertical"));
            }
            call(
                s,
                "pane.create",
                json!({"window":snapshot["window"],"orientation":arg}),
            )?;
        }
        "zoom" | "only" => {
            call(
                s,
                "pane.layout",
                json!({"window":snapshot["window"],"zoom":name == "only" || snapshot["zoom"]!=true}),
            )?;
        }
        "copy" => {
            *mode = Mode::Copy(history(
                s,
                snapshot["active_pane"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("Active pane missing"))?,
            )?)
        }
        "paste" => {
            call(s, "buffer.paste", json!({"pane":snapshot["active_pane"]}))?;
        }
        "kill" | "quit" => {
            let rows = call(s, "pane.list", json!({"session":session}))?;
            let panes = rows
                .as_array()
                .ok_or_else(|| io::Error::other("Pane list unavailable"))?
                .iter()
                .filter(|p| name == "quit" || p["window"] == snapshot["window"])
                .filter_map(|p| p["id"].as_str().map(str::to_owned))
                .collect();
            *mode = Mode::Confirm {
                panes,
                label: if name == "quit" {
                    "Close ALL windows in this session"
                } else {
                    "Kill this window and its programs"
                }
                .into(),
            };
        }
        "focus" => {
            let panes = snapshot["panes"]
                .as_array()
                .filter(|p| !p.is_empty())
                .ok_or_else(|| io::Error::other("Pane list empty"))?;
            let i = panes
                .iter()
                .position(|p| p["id"] == snapshot["active_pane"])
                .unwrap_or(0);
            call(
                s,
                "pane.select",
                json!({"pane":panes[(i+1)%panes.len()]["id"]}),
            )?;
        }
        "windows" => *mode = windows(s, session, &snapshot["window"])?,
        "help" => *mode = Mode::Help(0),
        "detach" => {
            call(s, "session.detach", json!({}))?;
            STOP.store(true, Ordering::Relaxed);
        }
        _ => {
            return Err(io::Error::other(
                "Unknown command; help lists implemented commands",
            ));
        }
    }
    Ok(())
}
fn draw_mode(
    mode: &mut Mode,
    snapshot: &Value,
    size: (u16, u16),
    status: &str,
    config: &Config,
) -> io::Result<()> {
    if matches!(mode, Mode::Live) {
        return draw(snapshot, &format!("{} · {}", config.label(), status));
    }
    let height = size.0 as usize;
    let columns = size.1 as usize;
    let mut out = if let Mode::Copy(copy) = mode {
        copy.render(height, columns)
    } else {
        let mut out = String::from(
            "\x1b[?25l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[0m\x1b[H\x1b[2J",
        );
        match mode {
            Mode::Chooser { rows, index, .. } => {
                let top = index.saturating_sub(height.saturating_sub(1));
                for (y, row) in rows.iter().enumerate().skip(top).take(height) {
                    out.push_str(&format!(
                        "\x1b[{};1H{}{}\x1b[0m",
                        y - top + 1,
                        if y == *index { "\x1b[7m" } else { "" },
                        copy::clip(&format!("{:>3}  {}", y, row.name), columns)
                    ));
                }
                out.push_str(&format!(
                    "\x1b[{};1H\x1b[7m{}\x1b[0m",
                    height + 1,
                    copy::clip(
                        &format!(
                            "WINDOWS · arrows/digits, Enter select, Esc cancel · {}",
                            status
                        ),
                        columns
                    )
                ));
            }
            Mode::Confirm { label, .. } => {
                out.push_str(&format!(
                    "\x1b[{};1H\x1b[7m{}\x1b[0m",
                    height + 1,
                    copy::clip(
                        &format!("{label}? y confirms; any other key cancels"),
                        columns
                    )
                ));
            }
            Mode::Prompt(text) => {
                out.push_str(&copy::clip("RUWAC command · help · new · next/prev · other · select N · rename NAME · split horizontal/vertical · zoom · copy · paste · detach",columns));
                out.push_str(&format!(
                    "\x1b[{};1H:{}\x1b[?25h",
                    height + 1,
                    copy::clip(text, columns.saturating_sub(1))
                ));
            }
            Mode::Help(offset) => {
                let text = format!(
                    "RUWAC Screen controls (prefix {})\nc new window; n/p next/previous; double prefix forwards to nested terminal; o previous window\na literal configured prefix; d detach; 0–9 select window\nS horizontal split; | vertical split; Tab change pane; z zoom\n[ copy/search; ] internal paste; \" window chooser; : command\nCopy: arrows/hjkl; g/G and Home/End; 0/^/$; w/b/e\nPage Up/Down or Ctrl+B/F; Ctrl+U/D half page; H/M/L viewport\nSpace or Enter first mark; move then mark again to copy\n/ forward search; ? reverse; n/N repeat; searches wrap\nv rectangular selection; c/C left/right margins; Esc cancel\nCounts before movement; Unicode graphemes stay intact\nInternal copy buffer is separate from desktop clipboard\nCopy snapshot: newest 8 MiB/50,000 rows; selection cap 256 KiB\nCommands: new, next, prev, other, select N, rename NAME\nsplit horizontal/vertical, zoom, copy, paste, windows, help, detach\nRUWAC_SHORTCUTS prefix: <Control>a through <Control>z\nRUWAC_KEYMAP remaps one prefix key to an implemented key\nCtrl/arrow escape sequences go to applications outside modals\nModal paste/mouse cannot reach a child; search/prompt paste is text\nEsc/Enter close help; arrows/Page Up/Down scroll",
                    config.label()
                );
                let lines: Vec<_> = text.lines().collect();
                *offset = (*offset).min(lines.len().saturating_sub(height));
                for (y, line) in lines.iter().skip(*offset).take(height).enumerate() {
                    out.push_str(&format!("\x1b[{};1H{}", y + 1, copy::clip(line, columns)));
                }
                out.push_str(&format!(
                    "\x1b[{};1H\x1b[7mHELP · Esc/Enter exit\x1b[0m",
                    height + 1
                ));
            }
            _ => (),
        }
        out
    };
    if !status.is_empty() && !matches!(mode, Mode::Copy(_) | Mode::Chooser { .. }) {
        out.push_str(&format!(
            "\x1b[{};1H\x1b[7m{}\x1b[0m",
            height + 1,
            copy::clip(status, columns)
        ));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(out.as_bytes())?;
    stdout.flush()
}

fn verified_connect(socket: &Path) -> io::Result<UnixStream> {
    use std::os::unix::{
        fs::{FileTypeExt, MetadataExt},
        io::AsRawFd,
    };
    for (path, mode) in [
        (
            socket
                .parent()
                .ok_or_else(|| io::Error::other("Socket parent missing"))?,
            0o700,
        ),
        (socket, 0o600),
    ] {
        let m = std::fs::symlink_metadata(path)?;
        if m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != mode
            || m.file_type().is_symlink()
        {
            return Err(io::Error::other("Socket ownership/mode rejected"));
        }
    }
    if !std::fs::symlink_metadata(socket)?.file_type().is_socket() {
        return Err(io::Error::other("Not a socket"));
    }
    let s = UnixStream::connect(socket)?;
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
        return Err(io::Error::other("Server peer UID rejected"));
    }
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
    Ok(s)
}
pub fn attach(socket: &Path, session: &str) -> io::Result<()> {
    STOP.store(false, Ordering::Relaxed);
    let config = Config::load()?;
    let mut s = verified_connect(socket)?;
    let panes = call(&mut s, "pane.list", json!({"session":session}))?;
    let mut ancestor = std::process::id();
    for _ in 0..128 {
        if panes.as_array().is_some_and(|rows| {
            rows.iter()
                .any(|p| p["ended"] != true && p["pid"].as_u64() == Some(ancestor as u64))
        }) {
            return Err(io::Error::other(
                "Cannot attach a session from inside its own pane; use -S with a different session name for nesting",
            ));
        }
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{ancestor}/stat")) else {
            break;
        };
        let Some(parent) = stat
            .rsplit_once(") ")
            .and_then(|(_, tail)| tail.split_whitespace().nth(1))
            .and_then(|p| p.parse::<u32>().ok())
        else {
            break;
        };
        if parent == 0 || parent == ancestor {
            break;
        }
        ancestor = parent;
    }
    let _raw = Raw::enter()?;
    let mut screen_size = terminal_dimensions();
    let mut size = dimensions();
    call(
        &mut s,
        "session.attach",
        json!({"session":session,"rows":size.0,"cols":size.1}),
    )?;
    let (tx, rx) = mpsc::sync_channel(64);
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut b = [0; 8192];
        loop {
            match stdin.read(&mut b) {
                Ok(0) | Err(_) => {
                    let _ = tx.send(None);
                    break;
                }
                Ok(n) => {
                    if tx.send(Some(b[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let mut decoder = Decoder::default();
    let mut prefix = false;
    let mut mode = Mode::Live;
    let mut snapshot = call(&mut s, "client.render", json!({}))?;
    let mut seen_running = snapshot["panes"]
        .as_array()
        .is_some_and(|p| p.iter().any(|p| p["ended"] != true));
    let mut status = String::new();
    draw_mode(&mut mode, &snapshot, size, &status, &config)?;
    let mut rendered = Instant::now();
    let mut input_at = Instant::now();
    while !STOP.load(Ordering::Relaxed) {
        let events = match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(Some(b)) => {
                input_at = Instant::now();
                decoder.feed(&b)
            }
            Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(_) => {
                if input_at.elapsed() > Duration::from_millis(40) {
                    decoder.escape_timeout()
                } else {
                    vec![]
                }
            }
        };
        let mut dirty = !events.is_empty();
        for event in events {
            match event {
                Input::Rejected(e) => {
                    prefix = false;
                    status = e.into();
                    if let Mode::Copy(c) = &mut mode {
                        c.message = status.clone();
                    }
                }
                Input::Mouse(b) => {
                    if matches!(mode, Mode::Live) {
                        let (bar, target) = status_mouse(
                            &snapshot,
                            &b,
                            screen_size,
                            &format!("{} · {}", config.label(), status),
                        );
                        if bar {
                            if let Some(window) = target {
                                call(&mut s, "window.select", json!({"window":window}))?;
                                snapshot = call(&mut s, "client.render", json!({}))?;
                                prefix = false;
                                status.clear();
                            }
                            continue;
                        }
                        let result=String::from_utf8(b).map_err(io::Error::other).and_then(|text|call(&mut s,"client.input",json!({"pane":snapshot["active_pane"],"kind":"mouse","text":text})));
                        if let Err(e) = result {
                            status = e.to_string();
                        }
                    }
                }
                Input::Paste(b) => {
                    prefix = false;
                    match String::from_utf8(b) {
                        Ok(text) => match &mut mode {
                            Mode::Live => {
                                if let Err(e) = call(
                                    &mut s,
                                    "client.input",
                                    json!({"pane":snapshot["active_pane"],"kind":"paste","text":text}),
                                ) {
                                    status = e.to_string();
                                }
                            }
                            Mode::Copy(c) => c.paste_query(&text),
                            Mode::Prompt(prompt) => {
                                for c in text.chars().filter(|c| !c.is_control()) {
                                    if prompt.len() + c.len_utf8() > 4096 {
                                        break;
                                    }
                                    prompt.push(c);
                                }
                                status.clear();
                            }
                            _ => status = "Paste ignored in this view".into(),
                        },
                        Err(e) => status = e.to_string(),
                    }
                }
                Input::Keyboard(b) => {
                    let text = String::from_utf8(b).map_err(io::Error::other)?;
                    let mut pending = String::new();
                    for key in copy::keys(&text) {
                        let mut refresh = false;
                        let result = (|| -> io::Result<()> {
                            match &mut mode {
                                Mode::Confirm { panes, .. } => {
                                    if matches!(key, copy::Key::Char('y' | 'Y')) {
                                        for pane in std::mem::take(panes) {
                                            call(&mut s, "pane.close", json!({"pane":pane}))?;
                                        }
                                        if call(&mut s, "session.list", json!({}))?
                                            .as_array()
                                            .is_none_or(|rows| {
                                                !rows.iter().any(|r| r["id"] == session)
                                            })
                                        {
                                            STOP.store(true, Ordering::Relaxed);
                                        }
                                    }
                                    mode = Mode::Live;
                                    refresh = true;
                                }
                                Mode::Copy(c) => match c.key(&key, size.0 as usize) {
                                    copy::Action::Cancel => {
                                        mode = Mode::Live;
                                        status = "Copy cancelled; buffer unchanged".into();
                                        refresh = true;
                                    }
                                    copy::Action::Save => {
                                        let text =
                                            c.selected(MAX_TEXT).map_err(io::Error::other)?;
                                        let bytes = text.len();
                                        call(
                                            &mut s,
                                            "buffer.set",
                                            json!({"session":session,"text":text}),
                                        )?;
                                        mode = Mode::Live;
                                        status = format!("Copied {bytes} bytes internally");
                                        refresh = true;
                                    }
                                    _ => (),
                                },
                                Mode::Chooser { rows, index, typed } => {
                                    if key.escape() || matches!(key, copy::Key::Char('q' | '\x03'))
                                    {
                                        mode = Mode::Live;
                                        status = "Window choice cancelled".into();
                                        refresh = true;
                                    } else {
                                        match &key {
                                            copy::Key::Char('\r' | '\n') => {
                                                call(
                                                    &mut s,
                                                    "window.select",
                                                    json!({"window":rows[*index].id}),
                                                )?;
                                                mode = Mode::Live;
                                                status.clear();
                                                refresh = true;
                                            }
                                            copy::Key::Char('j' | '\x0e') => {
                                                *index = (*index + 1).min(rows.len() - 1);
                                                typed.clear();
                                            }
                                            copy::Key::Char('k' | '\x10') => {
                                                *index = index.saturating_sub(1);
                                                typed.clear();
                                            }
                                            copy::Key::Char('g') => {
                                                *index = 0;
                                                typed.clear();
                                            }
                                            copy::Key::Char('G') => {
                                                *index = rows.len() - 1;
                                                typed.clear();
                                            }
                                            copy::Key::Char(c @ '0'..='9') => {
                                                if typed.len() >= 3 {
                                                    typed.clear();
                                                }
                                                typed.push(*c);
                                                if let Ok(n) = typed.parse::<usize>() {
                                                    if n < rows.len() {
                                                        *index = n;
                                                    }
                                                }
                                            }
                                            copy::Key::Sequence(seq) => {
                                                typed.clear();
                                                match seq.as_str() {
                                                    "\x1b[A" | "\x1bOA" => {
                                                        *index = index.saturating_sub(1)
                                                    }
                                                    "\x1b[B" | "\x1bOB" => {
                                                        *index = (*index + 1).min(rows.len() - 1)
                                                    }
                                                    "\x1b[5~" => {
                                                        *index =
                                                            index.saturating_sub(size.0 as usize)
                                                    }
                                                    "\x1b[6~" => {
                                                        *index = (*index + size.0 as usize)
                                                            .min(rows.len() - 1)
                                                    }
                                                    _ => (),
                                                }
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                                Mode::Prompt(prompt) => {
                                    if key.escape() || matches!(key, copy::Key::Char('\x03')) {
                                        mode = Mode::Live;
                                        status = "Command cancelled".into();
                                        refresh = true;
                                    } else {
                                        match &key {
                                            copy::Key::Char('\r' | '\n') => {
                                                let text = prompt.clone();
                                                let mut next = Mode::Live;
                                                command(
                                                    &mut s, session, &snapshot, &text, &mut next,
                                                )?;
                                                mode = next;
                                                status.clear();
                                                refresh = true;
                                            }
                                            copy::Key::Char('\x7f' | '\x08') => {
                                                copy::backspace(prompt);
                                                status.clear();
                                            }
                                            copy::Key::Char(c) if !c.is_control() => {
                                                if prompt.len() + c.len_utf8() <= 4096 {
                                                    prompt.push(*c);
                                                }
                                                status.clear();
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                                Mode::Help(offset) => {
                                    if key.escape()
                                        || matches!(key, copy::Key::Char('\r' | '\n' | 'q' | '?'))
                                    {
                                        mode = Mode::Live;
                                        status.clear();
                                        refresh = true;
                                    } else {
                                        match &key {
                                            copy::Key::Char('j') => *offset += 1,
                                            copy::Key::Char('k') => {
                                                *offset = offset.saturating_sub(1)
                                            }
                                            copy::Key::Sequence(seq) => match seq.as_str() {
                                                "\x1b[A" => *offset = offset.saturating_sub(1),
                                                "\x1b[B" => *offset += 1,
                                                "\x1b[5~" => {
                                                    *offset = offset.saturating_sub(size.0 as usize)
                                                }
                                                "\x1b[6~" => *offset += size.0 as usize,
                                                _ => (),
                                            },
                                            _ => (),
                                        }
                                    }
                                }
                                Mode::Live => {
                                    if prefix {
                                        prefix = false;
                                        plain(&mut s, &snapshot, &mut pending)?;
                                        status.clear();
                                        if matches!(&key,copy::Key::Char(c)if *c as u32==config.prefix as u32)
                                        {
                                            if config.double_prefix_previous {
                                                call(
                                                    &mut s,
                                                    "window.previous",
                                                    json!({"session":session}),
                                                )?;
                                            } else {
                                                call(
                                                    &mut s,
                                                    "client.input",
                                                    json!({"pane":snapshot["active_pane"],"kind":"keyboard","text":(config.prefix as char).to_string()}),
                                                )?;
                                            }
                                        } else {
                                            let key = if key.escape() {
                                                copy::Key::Char('\x1b')
                                            } else {
                                                key
                                            };
                                            let copy::Key::Char(mut binding) = key else {
                                                return Err(io::Error::other(
                                                    "Unsupported prefix key; ? opens help",
                                                ));
                                            };
                                            binding = ruwac_protocol::screen_key(binding);
                                            if binding != '\t'
                                                && (1..=26).contains(&(binding as u32))
                                            {
                                                binding = (binding as u8 + b'a' - 1) as char;
                                            }
                                            binding = ruwac_protocol::screen_key(
                                                *config.remaps.get(&binding).unwrap_or(&binding),
                                            );
                                            match binding {
                                                'a' => {
                                                    call(
                                                        &mut s,
                                                        "client.input",
                                                        json!({"pane":snapshot["active_pane"],"kind":"keyboard","text":(config.prefix as char).to_string()}),
                                                    )?;
                                                }
                                                'k' => command(
                                                    &mut s, session, &snapshot, "kill", &mut mode,
                                                )?,
                                                'A' => mode = Mode::Prompt("rename ".into()),
                                                'c' => {
                                                    call(
                                                        &mut s,
                                                        "window.create",
                                                        json!({"session":session}),
                                                    )?;
                                                }
                                                'o' => {
                                                    call(
                                                        &mut s,
                                                        "window.previous",
                                                        json!({"session":session}),
                                                    )?;
                                                }
                                                'n' | 'p' => {
                                                    call(
                                                        &mut s,
                                                        "window.cycle",
                                                        json!({"session":session,"previous":binding=='p'}),
                                                    )?;
                                                }
                                                'd' => {
                                                    call(&mut s, "session.detach", json!({}))?;
                                                    STOP.store(true, Ordering::Relaxed);
                                                }
                                                'S' | 's' | '|' => {
                                                    call(
                                                        &mut s,
                                                        "pane.create",
                                                        json!({"window":snapshot["window"],"orientation":if binding=='|'{"vertical"}else{"horizontal"}}),
                                                    )?;
                                                }
                                                '[' => {
                                                    mode = Mode::Copy(history(
                                                        &mut s,
                                                        snapshot["active_pane"]
                                                            .as_str()
                                                            .ok_or_else(|| {
                                                                io::Error::other(
                                                                    "Active pane missing",
                                                                )
                                                            })?,
                                                    )?)
                                                }
                                                ']' => {
                                                    call(
                                                        &mut s,
                                                        "buffer.paste",
                                                        json!({"pane":snapshot["active_pane"]}),
                                                    )?;
                                                }
                                                'z' => {
                                                    call(
                                                        &mut s,
                                                        "pane.layout",
                                                        json!({"window":snapshot["window"],"zoom":snapshot["zoom"]!=true}),
                                                    )?;
                                                }
                                                '"' => {
                                                    mode = windows(
                                                        &mut s,
                                                        session,
                                                        &snapshot["window"],
                                                    )?
                                                }
                                                ':' => mode = Mode::Prompt(String::new()),
                                                '?' => mode = Mode::Help(0),
                                                '\t' => {
                                                    let panes = snapshot["panes"]
                                                        .as_array()
                                                        .filter(|a| !a.is_empty())
                                                        .ok_or_else(|| {
                                                            io::Error::other("Pane list empty")
                                                        })?;
                                                    let index = panes
                                                        .iter()
                                                        .position(|p| {
                                                            p["id"] == snapshot["active_pane"]
                                                        })
                                                        .unwrap_or(0);
                                                    call(
                                                        &mut s,
                                                        "pane.select",
                                                        json!({"pane":panes[(index+1)%panes.len()]["id"]}),
                                                    )?;
                                                }
                                                '0'..='9' => {
                                                    let windows = call(
                                                        &mut s,
                                                        "window.list",
                                                        json!({"session":session}),
                                                    )?;
                                                    let window = windows
                                                        .as_array()
                                                        .and_then(|a| {
                                                            a.get(binding as usize - '0' as usize)
                                                        })
                                                        .ok_or_else(|| {
                                                            io::Error::other(
                                                                "Window index unavailable",
                                                            )
                                                        })?;
                                                    call(
                                                        &mut s,
                                                        "window.select",
                                                        json!({"window":window["id"]}),
                                                    )?;
                                                }
                                                _ => {
                                                    return Err(io::Error::other(
                                                        "Unsupported prefix key; ? opens help",
                                                    ));
                                                }
                                            }
                                        }
                                        refresh = true;
                                    } else if matches!(&key,copy::Key::Char(c)if *c as u32==config.prefix as u32)
                                    {
                                        plain(&mut s, &snapshot, &mut pending)?;
                                        prefix = true;
                                    } else {
                                        pending.push_str(&key.bytes());
                                    }
                                }
                            }
                            Ok(())
                        })();
                        if let Err(e) = result {
                            status = e.to_string();
                            if let Mode::Copy(c) = &mut mode {
                                c.message = status.clone();
                            }
                        }
                        if STOP.load(Ordering::Relaxed) {
                            break;
                        }
                        if refresh {
                            snapshot = call(&mut s, "client.render", json!({}))?;
                        }
                    }
                    if !pending.is_empty() {
                        if let Err(e) = plain(&mut s, &snapshot, &mut pending) {
                            status = e.to_string();
                        }
                    }
                }
            }
            if STOP.load(Ordering::Relaxed) {
                break;
            }
        }
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        let new_screen_size = terminal_dimensions();
        if new_screen_size != screen_size {
            screen_size = new_screen_size;
            dirty = true;
        }
        let newsize = dimensions();
        if newsize != size {
            size = newsize;
            call(
                &mut s,
                "client.resize",
                json!({"rows":size.0,"cols":size.1}),
            )?;
            dirty = true;
        }
        if rendered.elapsed() > Duration::from_millis(25) {
            if matches!(mode, Mode::Live) {
                let fresh = call(
                    &mut s,
                    "client.render",
                    json!({"revision":snapshot["revision"]}),
                )?;
                if fresh["unchanged"] != true {
                    snapshot = fresh;
                    let ended = snapshot["panes"]
                        .as_array()
                        .is_some_and(|p| !p.is_empty() && p.iter().all(|p| p["ended"] == true));
                    if seen_running && ended {
                        let panes = call(&mut s, "pane.list", json!({"session":session}))?;
                        if let Some(live) = panes
                            .as_array()
                            .and_then(|p| p.iter().find(|p| p["ended"] != true))
                        {
                            call(&mut s, "window.select", json!({"window":live["window"]}))?;
                            call(&mut s, "pane.select", json!({"pane":live["id"]}))?;
                            snapshot = call(&mut s, "client.render", json!({}))?;
                        } else {
                            break;
                        }
                    }
                    seen_running |= !ended;
                    dirty = true;
                }
            }
            rendered = Instant::now();
        }
        if dirty {
            draw_mode(&mut mode, &snapshot, size, &status, &config)?;
        }
    }
    let _ = call(&mut s, "session.detach", json!({}));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_input_and_replies() {
        let mut d = Decoder::default();
        assert_eq!(
            d.feed(b"EARLY\x1b[?"),
            vec![Input::Keyboard(b"EARLY".to_vec())]
        );
        assert_eq!(d.feed(b"1;2cLATE"), vec![Input::Keyboard(b"LATE".to_vec())]);
        assert!(d.feed(&[0xc5]).is_empty());
        assert_eq!(
            d.feed(&[0xbe]),
            vec![Input::Keyboard("ž".as_bytes().to_vec())]
        );
        assert!(d.feed(b"\x1b[<0;").is_empty());
        assert_eq!(
            d.feed(b"1;2M"),
            vec![Input::Mouse(b"\x1b[<0;1;2M".to_vec())]
        );
    }
    #[test]
    fn oversized_paste_drained() {
        let mut d = Decoder::default();
        let mut b = b"\x1b[200~".to_vec();
        b.extend(vec![b'x'; MAX_TEXT + 6]);
        assert_eq!(d.feed(&b), vec![Input::Rejected("paste exceeds 256 KiB")]);
        assert!(d.feed(b"\x01d").is_empty());
        assert_eq!(
            d.feed(b"\x1b[201~OK"),
            vec![Input::Keyboard(b"OK".to_vec())]
        );
    }
    #[test]
    fn paste_not_prefix() {
        let mut d = Decoder::default();
        assert!(d.feed(b"\x1b[200~a\x01d").is_empty());
        assert_eq!(d.feed(b"\x1b[201~"), vec![Input::Paste(b"a\x01d".to_vec())]);
        assert_eq!(
            d.feed(b"\x1b]11;rgb:ffff/0000/0000\x07x"),
            vec![Input::Keyboard(b"x".to_vec())]
        );
    }
}

#[cfg(test)]
mod display_identity_tests {
    use super::*;
    #[test]
    fn colors_preserve_native_ansi_bright_indexed_and_rgb_semantics() {
        assert_eq!(sgr(&json!(1), true), "31");
        assert_eq!(sgr(&json!(9), true), "91");
        assert_eq!(sgr(&json!(1), false), "41");
        assert_eq!(sgr(&json!(9), false), "101");
        assert_eq!(sgr(&json!(123), true), "38;5;123");
        assert_eq!(sgr(&json!([1, 2, 3]), true), "38;2;1;2;3");
        assert_eq!(sgr(&json!(256), true), "39");
        assert_eq!(sgr(&json!(257), false), "49");
        assert_eq!(sgr(&json!(260), true), "38;5;1");
    }
    #[test]
    fn selected_window_identity_and_shared_size_remain_visible_and_safe() {
        let v = json!({"session_name":"d\u{1b}\u{7}","window_index":1,"window":"b","name":"editor","rows":23,"cols":80,"windows":[{"id":"a","name":"shell","index":0},{"id":"b","name":"editor","index":1}]});
        let line = status_line(&v, (30, 160), "");
        assert!(line.starts_with("RUWAC d · 1:editor"));
        assert!(line.contains("1:editor*"));
        assert!(line.contains("shared PTY 80×23"));
        assert!(!window_identity(&v).contains('\u{1b}'));
        assert!(!window_identity(&v).contains('\u{7}'));
        assert!(
            status_line(&v, (23, 80), "Window choice cancelled")
                .contains("Window choice cancelled")
        );
    }
}
