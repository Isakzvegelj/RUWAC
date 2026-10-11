use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::Dimensions,
    index::{Column, Line, Point},
    term::{Config, TermMode, cell::Flags},
    vte::ansi::{Color, Processor},
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<String>>>);
impl EventListener for Events {
    fn send_event(&self, e: Event) {
        if let Event::PtyWrite(s) = e {
            self.0.lock().unwrap().push(s)
        }
    }
}
#[derive(Clone, Copy)]
struct Size(usize, usize);
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.0
    }
    fn screen_lines(&self) -> usize {
        self.0
    }
    fn columns(&self) -> usize {
        self.1
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub rows: u16,
    pub cols: u16,
    pub epoch: u64,
    pub history: Vec<(String, bool)>,
    pub grid: Value,
    pub dropped_lines: usize,
}

pub struct Terminal {
    archived: Option<Archive>,
    term: Term<Events>,
    parser: Processor,
    events: Events,
    pub rows: u16,
    pub cols: u16,
    pub history_epoch: u64,
}
impl Terminal {
    pub fn new(rows: u16, cols: u16) -> Self {
        let events = Events::default();
        let config = Config {
            scrolling_history: history_limit(cols),
            ..Config::default()
        };
        let term = Term::new(config, &Size(rows as usize, cols as usize), events.clone());
        Self {
            archived: None,
            term,
            parser: Processor::new(),
            events,
            rows,
            cols,
            history_epoch: 0,
        }
    }
    pub fn feed(&mut self, b: &[u8]) -> Vec<String> {
        if self.archived.is_some() {
            return Vec::new();
        }
        self.history_epoch = self.history_epoch.wrapping_add(1);
        self.parser.advance(&mut self.term, b);
        std::mem::take(&mut *self.events.0.lock().unwrap())
    }
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.history_epoch = self.history_epoch.wrapping_add(1);
        self.rows = rows;
        self.cols = cols;
        if self.archived.is_some() {
            return;
        }
        self.term.set_options(Config {
            scrolling_history: history_limit(cols),
            ..Config::default()
        });
        self.term.resize(Size(rows as usize, cols as usize))
    }
    pub fn reset_input_modes(&mut self) {
        self.feed(
            b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\x1b[?1l\x1b[?1049l\x1b[0m",
        );
    }
    pub fn sgr_mouse(&self) -> bool {
        self.archived.is_none() && self.term.mode().contains(TermMode::SGR_MOUSE)
    }
    pub fn mouse(&self) -> bool {
        self.archived.is_none()
            && self.term.mode().intersects(
                TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION,
            )
    }
    pub fn paste(&self) -> bool {
        self.archived.is_none() && self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }
    /// Persist canonical cells/text, never raw terminal escape sequences or input.
    /// Each pane gets a bounded tail; truncation is explicit on restored history.
    pub fn archive(&self) -> Archive {
        if let Some(a) = &self.archived {
            let mut archive = a.clone();
            archive.epoch = self.history_epoch;
            return archive;
        }
        let g = self.term.grid();
        let top = g.topmost_line().0;
        let bottom = g.bottommost_line().0;
        let mut history = Vec::new();
        let mut bytes = 0usize;
        for r in (top..=bottom).rev().take(50000) {
            let mut row = String::new();
            for c in 0..self.cols as usize {
                let cell = &g[Point::new(Line(r), Column(c))];
                if !cell
                    .flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    row.push(if cell.c=='\t' {' '} else {cell.c});
                    if let Some(z) = cell.zerowidth() {
                        row.extend(z.iter());
                    }
                }
            }
            let wrap = g[Point::new(Line(r), Column(self.cols as usize - 1))]
                .flags
                .contains(Flags::WRAPLINE);
            let row = if wrap { row } else { row.trim_end().to_owned() };
            if bytes + row.len() > 512 * 1024 {
                break;
            }
            bytes += row.len();
            history.push((row, wrap));
        }
        history.reverse();
        Archive {
            rows: self.rows,
            cols: self.cols,
            epoch: self.history_epoch,
            dropped_lines: (bottom - top + 1) as usize - history.len(),
            history,
            grid: self.snapshot(),
        }
    }
    pub fn restore(mut a: Archive) -> std::result::Result<Self, String> {
        if !(1..=100).contains(&a.rows)
            || !(1..=200).contains(&a.cols)
            || a.history.len() > 50000
            || a.history.iter().map(|(s, _)| s.len()).sum::<usize>() > 512 * 1024
            || a.history
                .iter()
                .any(|(s, _)| s.chars().any(|c|c.is_control() && c!='\t'))
        {
            return Err("invalid terminal archive dimensions/history".into());
        }
        let rows = a.grid["cells"].as_array().ok_or("invalid cells")?;
        if a.grid["rows"] != a.rows || a.grid["cols"] != a.cols || rows.len() != a.rows as usize {
            return Err("invalid terminal archive grid".into());
        }
        for row in rows {
            let cells = row.as_array().ok_or("invalid row")?;
            if cells.len() != a.cols as usize {
                return Err("invalid column count".into());
            }
            for cell in cells {
                let c = cell.as_array().ok_or("invalid cell")?;
                if c.len() != 6
                    || !c[0].is_string()
                    || c[0].as_str().unwrap().len() > 4096
                    || c[0].as_str().unwrap().chars().any(|c|c.is_control() && c!='\t')
                    || ![1, 4, 5].iter().all(|i| c[*i].is_boolean())
                    || ![2, 3].iter().all(|i| valid_color(&c[*i]))
                {
                    return Err("invalid cell value".into());
                }
            }
        }
        // Legacy checkpoints contain the tab placeholder plus already expanded cells.
        // One blank cell preserves displayed columns without expanding it twice.
        for (row,_) in &mut a.history { *row=row.replace('\t'," "); }
        let mut term = Self::new(a.rows, a.cols);
        term.history_epoch = a.epoch.wrapping_add(1);
        term.archived = Some(a);
        Ok(term)
    }
    pub fn capture(&self, lines: usize) -> String {
        if let Some(a) = &self.archived {
            return a
                .history
                .iter()
                .skip(a.history.len().saturating_sub(lines))
                .map(|(s, _)| format!("{}\n", s.trim_end()))
                .collect();
        }
        let g = self.term.grid();
        let bottom = g.bottommost_line().0;
        let start = (bottom + 1 - lines as i32).max(g.topmost_line().0);
        let mut out = String::new();
        for r in start..=bottom {
            let mut row = String::new();
            for c in 0..self.cols as usize {
                let cell = &g[Point::new(Line(r), Column(c))];
                if !cell
                    .flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    row.push(if cell.c=='\t' {' '} else {cell.c});
                    if let Some(z) = cell.zerowidth() {
                        row.extend(z.iter())
                    }
                }
            }
            out.push_str(row.trim_end());
            out.push('\n');
        }
        out
    }
    pub fn history_page(&self, offset: usize, lines: usize) -> Value {
        if let Some(a) = &self.archived {
            let mut text = String::new();
            let mut wrapped = Vec::new();
            let mut rows = Vec::new();
            for (row, wrap) in a
                .history
                .iter()
                .skip(offset.min(a.history.len()))
                .take(lines.min(500))
            {
                if text.len() + row.len() + 1 > 128 * 1024 {
                    break;
                }
                text.push_str(row);
                if !wrap {
                    text.push('\n');
                }
                wrapped.push(*wrap);
                rows.push(json!({"text":row,"wrapped":wrap}));
            }
            return json!({"text":text,"offset":offset,"next_offset":offset.saturating_add(wrapped.len()),"epoch":self.history_epoch,"total_lines":a.history.len(),"wrapped":wrapped,"rows":rows,"alternate_screen":a.grid["alternate_screen"] == true,"restored":true,"dropped_lines":a.dropped_lines});
        }
        let g = self.term.grid();
        let start = g.topmost_line().0;
        let bottom = g.bottommost_line().0;
        let total = (bottom - start + 1) as usize;
        let mut text = String::new();
        let mut count = 0;
        let mut wrapped = Vec::new();
        let mut rows = Vec::new();
        for r in (start + offset.min(total) as i32)..=bottom {
            if count >= lines.min(500) {
                break;
            }
            let mut row = String::new();
            for c in 0..self.cols as usize {
                let cell = &g[Point::new(Line(r), Column(c))];
                if !cell
                    .flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    row.push(if cell.c=='\t' {' '} else {cell.c});
                    if let Some(z) = cell.zerowidth() {
                        row.extend(z.iter());
                    }
                }
            }
            let wrap = g[Point::new(Line(r), Column(self.cols as usize - 1))]
                .flags
                .contains(Flags::WRAPLINE);
            let row = if wrap { row.as_str() } else { row.trim_end() };
            if text.len() + row.len() + 1 > 128 * 1024 {
                break;
            }
            text.push_str(row);
            if !wrap {
                text.push('\n');
            }
            wrapped.push(wrap);
            rows.push(json!({"text":row,"wrapped":wrap}));
            count += 1;
        }
        json!({"text":text,"offset":offset,"next_offset":offset.saturating_add(count),"epoch":self.history_epoch,"total_lines":total,"wrapped":wrapped,"rows":rows,"alternate_screen":self.term.mode().contains(TermMode::ALT_SCREEN)})
    }
    pub fn snapshot(&self) -> Value {
        if let Some(a) = &self.archived {
            let mut grid = a.grid.clone();
            let blank = json!([" ", false, 256, 257, false, false]);
            let cells = grid["cells"].as_array_mut().unwrap();
            cells.truncate(self.rows as usize);
            cells.resize_with(self.rows as usize, || json!([]));
            for row in cells {
                let row = row.as_array_mut().unwrap();
                row.truncate(self.cols as usize);
                row.resize(self.cols as usize, blank.clone());
            }
            grid["rows"] = json!(self.rows);
            grid["cols"] = json!(self.cols);
            grid["cursor"]["visible"] = json!(false);
            for mode in [
                "mouse",
                "mouse_drag",
                "mouse_motion",
                "mouse_sgr",
                "application_cursor",
                "bracketed_paste",
            ] {
                grid[mode] = json!(false);
            }
            grid["restored"] = json!(true);
            return grid;
        }
        let g = self.term.grid();
        let mut rows = Vec::new();
        for r in 0..self.rows as usize {
            let mut cells = Vec::new();
            for c in 0..self.cols as usize {
                let cell = &g[Point::new(Line(r as i32), Column(c))];
                let mut s = cell.c.to_string();
                if let Some(z) = cell.zerowidth() {
                    s.extend(z.iter())
                }
                cells.push(json!([
                    s,
                    cell.flags
                        .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER),
                    color(cell.fg),
                    color(cell.bg),
                    cell.flags.contains(Flags::BOLD),
                    cell.flags.contains(Flags::INVERSE)
                ]));
            }
            rows.push(cells)
        }
        json!({"rows":self.rows,"cols":self.cols,"cells":rows,"cursor":{"row":g.cursor.point.line.0,"col":g.cursor.point.column.0,"visible":self.term.mode().contains(TermMode::SHOW_CURSOR)},"mouse":self.mouse(),"mouse_drag":self.term.mode().contains(TermMode::MOUSE_DRAG),"mouse_motion":self.term.mode().contains(TermMode::MOUSE_MOTION),"mouse_sgr":self.term.mode().contains(TermMode::SGR_MOUSE),"alternate_screen":self.term.mode().contains(TermMode::ALT_SCREEN),"application_cursor":self.term.mode().contains(TermMode::APP_CURSOR),"bracketed_paste":self.paste()})
    }
}
fn valid_color(c: &Value) -> bool {
    c.as_u64().is_some_and(|i| i <= 268)
        || c.as_array()
            .is_some_and(|a| a.len() == 3 && a.iter().all(|n| n.as_u64().is_some_and(|n| n <= 255)))
}
fn history_limit(cols: u16) -> usize {
    let lines = std::env::var("RUWAC_SCROLLBACK_LINES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50000)
        .clamp(2000, 1000000);
    let budget = std::env::var("RUWAC_SCROLLBACK_MIB")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(256)
        .clamp(8, 512)
        * 1024
        * 1024;
    lines
        .min(
            budget
                / (cols.max(1) as usize
                    * std::mem::size_of::<alacritty_terminal::term::cell::Cell>()),
        )
        .max(1)
}
fn color(c: Color) -> Value {
    match c {
        Color::Spec(rgb) => json!([rgb.r, rgb.g, rgb.b]),
        Color::Indexed(i) => json!(i),
        Color::Named(n) => json!(n as u16),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_modes_unicode() {
        let mut t = Terminal::new(4, 20);
        t.feed("ž界e\u{301}🧪".as_bytes());
        assert!(t.capture(4).contains("ž界e\u{301}🧪"));
        t.feed(b"\x1b[38;2;1;2;3mX\x1b[?1000h\x1b[?2004h");
        let s = t.snapshot();
        assert_eq!(s["cells"][0][6][2], json!([1, 2, 3]));
        assert!(t.mouse() && t.paste());
        t.feed(b"\x1b[?1000l\x1b[?2004l");
        assert!(!t.mouse() && !t.paste());
        t.feed(b"\x1b[?1049hALT");
        assert!(t.capture(4).contains("ALT"));
        t.feed(b"\x1b[?1049l");
        assert!(t.capture(4).contains("ž界"));
    }
    #[test]
    fn tabs_copy_at_canonical_columns() {
        let mut t=Terminal::new(4,16);
        t.feed(b"A\tB\r\n");
        assert_eq!(t.capture(4).lines().next(),Some("A       B"));
        let page=t.history_page(0,500);
        assert_eq!(page["rows"][0]["text"],"A       B");
        assert_eq!(page["rows"][0]["wrapped"],false);
        assert_eq!(t.snapshot()["cells"][0][1][0],"\t");
        let restored=Terminal::restore(t.archive()).unwrap();
        assert_eq!(restored.history_page(0,500)["rows"],page["rows"]);
        assert_eq!(restored.capture(4),t.capture(4));
    }
    #[test]
    fn paged_scrollback_and_wrapping() {
        let mut t = Terminal::new(4, 80);
        for n in 0..51000 {
            t.feed(format!("line-{n:05} ž界\r\n").as_bytes());
        }
        let p = t.history_page(0, 500);
        assert!(p["total_lines"].as_u64().unwrap() >= 50000);
        assert_eq!(p["next_offset"], 500);
        let next = t.history_page(500, 500);
        assert_ne!(p["text"], next["text"]);
        assert!(next["text"].as_str().unwrap().contains("ž界"));
        t.feed(b"\x1b[?1049hALT");
        assert_eq!(t.history_page(0, 500)["total_lines"], 4);
        t.feed(b"\x1b[?1049l");
        assert!(t.history_page(0, 500)["total_lines"].as_u64().unwrap() >= 50000);
        let mut wrapped = Terminal::new(2, 4);
        wrapped.feed(b"abcdefghX");
        let p = wrapped.history_page(0, 500);
        assert!(p["wrapped"].as_array().unwrap().iter().any(|v| v == true));
        assert!(p["text"].as_str().unwrap().contains("abcdefghX"));
    }
    #[test]
    fn archived_canonical_unicode_cells_and_read_only_modes() {
        let mut t = Terminal::new(4, 10);
        t.feed(
            "ž界e\u{301}🧪abcdefghijk\r\n\x1b[38;2;1;2;3mCOLOUR\x1b[?1000h\x1b[?2004h".as_bytes(),
        );
        t.feed(b"\r\nTAB\tEND");
        let before = t.history_page(0, 500);
        assert!(before["text"].as_str().unwrap().contains("TAB"));
        assert!(!before["text"].as_str().unwrap().contains('\t'));
        let a: Archive =
            serde_json::from_slice(&serde_json::to_vec(&t.archive()).unwrap()).unwrap();
        let mut restored = Terminal::restore(a.clone()).unwrap();
        assert_eq!(restored.capture(4), t.capture(4));
        assert_eq!(restored.snapshot()["cells"], t.snapshot()["cells"]);
        assert_eq!(restored.history_page(0, 500)["text"], before["text"]);
        assert_eq!(restored.history_page(0, 500)["wrapped"], before["wrapped"]);
        assert!(!restored.mouse() && !restored.paste());
        restored.feed(b"FORGED-INPUT");
        assert_eq!(restored.capture(4), t.capture(4));
        restored.resize(6, 20);
        assert_eq!(restored.snapshot()["cells"].as_array().unwrap().len(), 6);
        assert_eq!(
            restored.snapshot()["cells"][0].as_array().unwrap().len(),
            20
        );
        let mut bad = a;
        bad.grid["cells"][0][0][0] = json!("\x1b");
        assert!(Terminal::restore(bad).is_err());
    }
    #[test]
    fn archive_has_explicit_bounded_tail() {
        let mut t = Terminal::new(4, 80);
        for i in 0..20000 {
            t.feed(format!("line-{i:05} {}\r\n", "X".repeat(60)).as_bytes());
        }
        let a = t.archive();
        assert!(a.dropped_lines > 0);
        assert!(a.history.iter().map(|(s, _)| s.len()).sum::<usize>() <= 512 * 1024);
        let restored = Terminal::restore(a).unwrap();
        assert!(restored.capture(4).contains("line-19999"));
        assert!(
            restored.history_page(0, 500)["dropped_lines"]
                .as_u64()
                .unwrap()
                > 0
        );
    }
    #[test]
    fn replies_and_clipboard() {
        let mut t = Terminal::new(4, 20);
        assert!(!t.feed(b"\x1b[6n").is_empty());
        assert!(t.feed(b"\x1b]52;c;QUJD\x07").is_empty());
    }
}
