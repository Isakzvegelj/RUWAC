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
pub struct Terminal {
    term: Term<Events>,
    parser: Processor,
    events: Events,
    pub rows: u16,
    pub cols: u16,
}
impl Terminal {
    pub fn new(rows: u16, cols: u16) -> Self {
        let events = Events::default();
        let config = Config {
            scrolling_history: 2000,
            ..Config::default()
        };
        let term = Term::new(config, &Size(rows as usize, cols as usize), events.clone());
        Self {
            term,
            parser: Processor::new(),
            events,
            rows,
            cols,
        }
    }
    pub fn feed(&mut self, b: &[u8]) -> Vec<String> {
        self.parser.advance(&mut self.term, b);
        std::mem::take(&mut *self.events.0.lock().unwrap())
    }
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
        self.term.resize(Size(rows as usize, cols as usize))
    }
    pub fn mouse(&self) -> bool {
        self.term.mode().intersects(
            TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION,
        )
    }
    pub fn paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }
    pub fn capture(&self, lines: usize) -> String {
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
                    row.push(cell.c);
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
    pub fn snapshot(&self) -> Value {
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
        json!({"rows":self.rows,"cols":self.cols,"cells":rows,"cursor":{"row":g.cursor.point.line.0,"col":g.cursor.point.column.0,"visible":self.term.mode().contains(TermMode::SHOW_CURSOR)},"mouse":self.mouse(),"mouse_sgr":self.term.mode().contains(TermMode::SGR_MOUSE),"alternate_screen":self.term.mode().contains(TermMode::ALT_SCREEN),"application_cursor":self.term.mode().contains(TermMode::APP_CURSOR),"bracketed_paste":self.paste()})
    }
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
    fn replies_and_clipboard() {
        let mut t = Terminal::new(4, 20);
        assert!(!t.feed(b"\x1b[6n").is_empty());
        assert!(t.feed(b"\x1b]52;c;QUJD\x07").is_empty());
    }
}
