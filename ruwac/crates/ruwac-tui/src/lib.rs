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
            io::stdout().write_all(b"\x1b[?1049h\x1b[?2004h\x1b[?25l")?;
            io::stdout().flush()?;
            Ok(guard)
        }
    }
}
impl Drop for Raw {
    fn drop(&mut self) {
        let _=io::stdout().write_all(b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1004l\x1b[?2004l\x1b[?25h\x1b[0m\x1b[?1049l");
        let _ = io::stdout().flush();
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.term);
            libc::signal(libc::SIGTERM, self.term_handler);
            libc::signal(libc::SIGINT, self.int_handler);
        }
    }
}
fn dimensions() -> (u16, u16) {
    unsafe {
        let mut s: libc::winsize = std::mem::zeroed();
        libc::ioctl(0, libc::TIOCGWINSZ, &mut s);
        (
            s.ws_row.saturating_sub(1).clamp(4, 100),
            s.ws_col.clamp(10, 200),
        )
    }
}
fn call(s: &mut UnixStream, m: &str, v: Value) -> io::Result<Value> {
    write_frame(s, &request(m, v))?;
    let r = read_frame(s)?;
    if let Some(e) = r.get("error") {
        return Err(io::Error::other(e.to_string()));
    }
    Ok(r["result"].clone())
}
fn sgr(c: &Value, fg: bool) -> String {
    let base = if fg { 38 } else { 48 };
    if let Some(a) = c.as_array() {
        format!("{base};2;{};{};{}", a[0], a[1], a[2])
    } else {
        let n = c.as_u64().unwrap_or(256);
        if n < 256 {
            format!("{base};5;{n}")
        } else if n >= 264 && n <= 271 {
            format!("{base};5;{}", n - 264)
        } else {
            if fg { "39" } else { "49" }.into()
        }
    }
}
fn safe(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}
fn draw(v: &Value, size: (u16, u16), status: &str) -> io::Result<()> {
    let mut out = String::from("\x1b[?25l\x1b[0m\x1b[H\x1b[2J");
    let mut offset = 0;
    let active = v["active_pane"].as_str().unwrap_or("");
    let mut cursor = None;
    let mut mouse = false;
    if let Some(panes) = v["panes"].as_array() {
        for p in panes {
            let g = &p["grid"];
            if p["id"] == active {
                let c = &g["cursor"];
                if c["visible"] == true {
                    cursor = Some((
                        offset + c["row"].as_u64().unwrap_or(0),
                        c["col"].as_u64().unwrap_or(0),
                    ))
                }
                mouse = g["mouse"] == true;
            }
            if let Some(rows) = g["cells"].as_array() {
                for (r, cells) in rows.iter().enumerate() {
                    out.push_str(&format!("\x1b[{};1H", offset + r as u64 + 1));
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
                        if st != style {
                            out.push_str(&format!("\x1b[{st}m"));
                            style = st;
                        }
                        out.push_str(&safe(cell[0].as_str().unwrap_or(" ")));
                    }
                }
                offset += rows.len() as u64;
            }
        }
    }
    out.push_str(&format!(
        "\x1b[{};1H\x1b[0;7m{}\x1b[0m\x1b[K",
        size.0 + 1,
        safe(&format!(
            "RUWAC {}  ^A c/n/p/d/a S Tab [ ] ?  {}",
            v["name"].as_str().unwrap_or(""),
            status
        ))
        .chars()
        .take(size.1 as usize)
        .collect::<String>()
    ));
    out.push_str(if mouse {
        "\x1b[?1000h\x1b[?1006h"
    } else {
        "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l"
    });
    if let Some((r, c)) = cursor {
        out.push_str(&format!("\x1b[{};{}H\x1b[?25h", r + 1, c + 1));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(out.as_bytes())?;
    stdout.flush()
}
pub fn attach(socket: &Path, session: &str) -> io::Result<()> {
    STOP.store(false, Ordering::Relaxed);
    let _raw = Raw::enter()?;
    let mut s = UnixStream::connect(socket)?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
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
    let mut snapshot = call(&mut s, "client.render", json!({}))?;
    draw(&snapshot, size, "")?;
    let mut rendered = Instant::now();
    let mut input_at = Instant::now();
    let mut status = String::new();
    let mut drawn_status = String::new();
    while !STOP.load(Ordering::Relaxed) {
        let events = match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(Some(b)) => {
                input_at = Instant::now();
                decoder.feed(&b)
            }
            Ok(None) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(_) => {
                if input_at.elapsed() > Duration::from_millis(40) {
                    decoder.escape_timeout()
                } else {
                    vec![]
                }
            }
        };
        for event in events {
            let result: io::Result<()> = (|| {
                match event {
                    Input::Rejected(e) => {
                        status = e.into();
                    }
                    e @ (Input::Mouse(_) | Input::Paste(_)) => {
                        let kind = if matches!(&e, Input::Mouse(_)) {
                            "mouse"
                        } else {
                            "paste"
                        };
                        let b = match e {
                            Input::Mouse(b) | Input::Paste(b) => b,
                            _ => unreachable!(),
                        };
                        let t = String::from_utf8(b).map_err(io::Error::other)?;
                        call(
                            &mut s,
                            "client.input",
                            json!({"pane":snapshot["active_pane"],"kind":kind,"text":t}),
                        )?;
                    }
                    Input::Keyboard(b) => {
                        let mut plain = Vec::new();
                        for x in b {
                            if prefix {
                                prefix = false;
                                if !plain.is_empty() {
                                    let t = String::from_utf8(std::mem::take(&mut plain))
                                        .map_err(io::Error::other)?;
                                    call(
                                        &mut s,
                                        "client.input",
                                        json!({"pane":snapshot["active_pane"],"kind":"keyboard","text":t}),
                                    )?;
                                }
                                match x {
                                    b'd' => {
                                        call(&mut s, "session.detach", json!({}))?;
                                        STOP.store(true, Ordering::Relaxed);
                                        break;
                                    }
                                    b'a' | 1 => {
                                        call(
                                            &mut s,
                                            "pane.sendKey",
                                            json!({"pane":snapshot["active_pane"],"key":"CtrlA"}),
                                        )?;
                                    }
                                    b'c' => {
                                        call(&mut s, "window.create", json!({"session":session}))?;
                                    }
                                    b'n' | b'p' => {
                                        call(
                                            &mut s,
                                            "window.cycle",
                                            json!({"session":session,"previous":x==b'p'}),
                                        )?;
                                    }
                                    b'S' => {
                                        call(
                                            &mut s,
                                            "pane.create",
                                            json!({"window":snapshot["window"]}),
                                        )?;
                                    }
                                    b'[' => {
                                        let c = call(
                                            &mut s,
                                            "pane.capture",
                                            json!({"pane":snapshot["active_pane"],"lines":250}),
                                        )?;
                                        call(
                                            &mut s,
                                            "buffer.set",
                                            json!({"session":session,"text":c["text"]}),
                                        )?;
                                        status="copied bounded capture internally; selection UI pending".into();
                                    }
                                    b']' => {
                                        call(
                                            &mut s,
                                            "buffer.paste",
                                            json!({"pane":snapshot["active_pane"]}),
                                        )?;
                                    }
                                    b'\t' => {
                                        let p = snapshot["panes"].as_array().unwrap();
                                        let i = p
                                            .iter()
                                            .position(|p| p["id"] == snapshot["active_pane"])
                                            .unwrap_or(0);
                                        call(
                                            &mut s,
                                            "pane.select",
                                            json!({"pane":p[(i+1)%p.len()]["id"]}),
                                        )?;
                                    }
                                    b'0'..=b'9' => {
                                        let w = call(
                                            &mut s,
                                            "window.list",
                                            json!({"session":session}),
                                        )?;
                                        if let Some(w) =
                                            w.as_array().unwrap().get((x - b'0') as usize)
                                        {
                                            call(
                                                &mut s,
                                                "window.select",
                                                json!({"window":w["id"]}),
                                            )?;
                                        }
                                    }
                                    b'?' => {
                                        status="spike: c n p 0-9 a d S Tab [capture ]paste; full Screen keys pending".into();
                                    }
                                    _ => {
                                        status =
                                            "unsupported prefix key; ^A ? lists implemented keys"
                                                .into();
                                    }
                                }
                                if STOP.load(Ordering::Relaxed) {
                                    break;
                                }
                                snapshot = call(&mut s, "client.render", json!({}))?;
                            } else if x == 1 {
                                if !plain.is_empty() {
                                    let t = String::from_utf8(std::mem::take(&mut plain))
                                        .map_err(io::Error::other)?;
                                    call(
                                        &mut s,
                                        "client.input",
                                        json!({"pane":snapshot["active_pane"],"kind":"keyboard","text":t}),
                                    )?;
                                }
                                prefix = true;
                            } else {
                                plain.push(x)
                            }
                        }
                        if !plain.is_empty() {
                            let t = String::from_utf8(plain).map_err(io::Error::other)?;
                            call(
                                &mut s,
                                "client.input",
                                json!({"pane":snapshot["active_pane"],"kind":"keyboard","text":t}),
                            )?;
                        }
                    }
                }
                Ok(())
            })();
            if let Err(e) = result {
                status = e.to_string();
            }
        }
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        let newsize = dimensions();
        if newsize != size {
            size = newsize;
            call(
                &mut s,
                "client.resize",
                json!({"rows":size.0,"cols":size.1}),
            )?;
            snapshot = call(&mut s, "client.render", json!({}))?;
            draw(&snapshot, size, &status)?;
        }
        if rendered.elapsed() > Duration::from_millis(25) {
            let fresh = call(
                &mut s,
                "client.render",
                json!({"revision":snapshot["revision"]}),
            )?;
            if fresh["unchanged"] != true {
                snapshot = fresh;
                draw(&snapshot, size, &status)?;
                drawn_status = status.clone();
            } else if status != drawn_status {
                draw(&snapshot, size, &status)?;
                drawn_status = status.clone();
            }
            rendered = Instant::now();
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
