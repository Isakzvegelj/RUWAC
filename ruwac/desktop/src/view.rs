use crate::{
    clipboard::{self, Broker},
    worker::{Command, Worker},
};
use adw::prelude::*;
use gtk::{gdk, glib};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};
const CW: f64 = 10.0;
const CH: f64 = 20.0;
pub struct View {
    pub session: String,
    pub root: gtk::Box,
    pub area: gtk::DrawingArea,
    pub status: gtk::Label,
    pub worker: Worker,
    pub frame: RefCell<Option<Value>>,
    pub target: RefCell<String>,
    pub drawn: Cell<u64>,
    pub prefix: Cell<bool>,
    pub detached: Cell<bool>,
    pub broker: Broker,
    window_index: Cell<Option<usize>>,
    pub warning: RefCell<Option<String>>,
    selection: RefCell<Option<((usize, usize), (usize, usize))>>,
    drag_start: Cell<(f64, f64)>,
    pub copy: gtk::TextView,
    stack: gtk::Stack,
}
impl View {
    pub fn new(socket: PathBuf, session: String, broker: Broker) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let status = gtk::Label::new(Some("Connecting to daemon…"));
        status.set_xalign(0.0);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        root.append(&status);
        let area = gtk::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .focusable(true)
            .content_width(800)
            .content_height(500)
            .build();
        let stack = gtk::Stack::builder().hexpand(true).vexpand(true).build();
        stack.set_transition_type(gtk::StackTransitionType::None);
        stack.add_named(&area, Some("terminal"));
        let copy = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::None)
            .build();
        let scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&copy)
            .build();
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search up to 2,000 canonical history lines"));
        let cb = copy.buffer();
        search.connect_search_changed(move |entry| {
            let needle = entry.text();
            if needle.is_empty() {
                return;
            }
            let all = cb.text(&cb.start_iter(), &cb.end_iter(), true);
            if let Some(byte) = all.find(needle.as_str()) {
                let start = all[..byte].chars().count() as i32;
                cb.select_range(
                    &cb.iter_at_offset(start),
                    &cb.iter_at_offset(start + needle.chars().count() as i32),
                );
            }
        });
        let copybox = gtk::Box::new(gtk::Orientation::Vertical, 4);
        copybox.append(&search);
        copybox.append(&scroll);
        stack.add_named(&copybox, Some("copy"));
        root.append(&stack);
        let worker = Worker::start(socket, session.clone());
        let v = Rc::new(Self {
            session,
            root,
            area,
            status,
            worker,
            frame: RefCell::new(None),
            target: RefCell::new(String::new()),
            drawn: Cell::new(0),
            prefix: Cell::new(false),
            detached: Cell::new(false),
            broker,
            window_index: Cell::new(None),
            warning: RefCell::new(None),
            selection: RefCell::new(None),
            drag_start: Cell::new((0.0, 0.0)),
            copy,
            stack,
        });
        let w = Rc::downgrade(&v);
        v.area.set_draw_func(move |area, cr, width, height| {
            if let Some(v) = w.upgrade() {
                v.draw(area, cr, width, height)
            }
        });
        let key = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&v);
        key.connect_key_pressed(move |_, key, _, mods| {
            if let Some(v) = w.upgrade() {
                v.key(key, mods);
            }
            glib::Propagation::Stop
        });
        v.area.add_controller(key);
        let key = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&v);
        key.connect_key_pressed(move |_, key, _, mods| {
            let Some(v) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if key == gdk::Key::Escape {
                v.stack.set_visible_child_name("terminal");
                v.area.grab_focus();
                return glib::Propagation::Stop;
            }
            if key == gdk::Key::Return {
                v.internal_copy();
                return glib::Propagation::Stop;
            }
            if mods.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
                && key
                    .to_unicode()
                    .is_some_and(|c| c.eq_ignore_ascii_case(&'c'))
            {
                v.copy_clipboard(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        v.copy.add_controller(key);
        let click = gtk::GestureClick::new();
        click.set_button(0);
        let w = Rc::downgrade(&v);
        click.connect_pressed(move |g, _, x, y| {
            if let Some(v) = w.upgrade() {
                v.click(x, y, g.current_button(), g.current_event_state(), false)
            }
        });
        let w = Rc::downgrade(&v);
        click.connect_released(move |g, _, x, y| {
            if let Some(v) = w.upgrade() {
                v.click(x, y, g.current_button(), g.current_event_state(), true)
            }
        });
        v.area.add_controller(click);
        let drag = gtk::GestureDrag::new();
        let w = Rc::downgrade(&v);
        drag.connect_drag_begin(move |g, x, y| {
            if let Some(v) = w.upgrade() {
                v.drag_start.set((x, y));
                if v.mouse_enabled()
                    && !g
                        .current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    return;
                }
                v.focus_at(x, y);
                if let Some((_, r, c)) = v.point(x, y) {
                    *v.selection.borrow_mut() = Some(((r, c), (r, c)));
                    v.area.queue_draw();
                }
            }
        });
        let w = Rc::downgrade(&v);
        drag.connect_drag_update(move |g, dx, dy| {
            if let Some(v) = w.upgrade() {
                let (x, y) = v.drag_start.get();
                if v.mouse_enabled()
                    && !g
                        .current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    v.mouse(32, x + dx, y + dy, false);
                    return;
                }
                if let Some((pane, r, c)) = v.point(x + dx, y + dy) {
                    if pane == *v.target.borrow() {
                        if let Some(selection) = v.selection.borrow_mut().as_mut() {
                            selection.1 = (r, c);
                        }
                        v.area.queue_draw();
                    }
                }
            }
        });
        let w = Rc::downgrade(&v);
        drag.connect_drag_end(move |g, _, _| {
            if let Some(v) = w.upgrade() {
                if !v.mouse_enabled()
                    || g.current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    v.copy_clipboard(true);
                }
            }
        });
        v.area.add_controller(drag);
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let w = Rc::downgrade(&v);
        wheel.connect_scroll(move |g, _, dy| {
            if let Some(v) = w.upgrade() {
                if v.mouse_enabled()
                    && !g
                        .current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    if let Some((x, y)) = g.current_event().and_then(|e| e.position()) {
                        v.mouse(if dy < 0.0 { 64 } else { 65 }, x, y, false);
                    }
                } else {
                    v.command(
                        "pane.capture",
                        json!({"pane":*v.target.borrow(),"lines":2000}),
                    );
                }
            }
            glib::Propagation::Stop
        });
        v.area.add_controller(wheel);
        let w = Rc::downgrade(&v);
        v.area.connect_resize(move|_,width,height|if let Some(v)=w.upgrade(){v.command("client.resize",json!({"rows":((height as f64/CH)as i32-2).clamp(4,100),"cols":((width as f64/CW)as i32).clamp(10,180)}));});
        v
    }
    pub fn warn(&self, message: &str) {
        *self.warning.borrow_mut() = Some(message.into());
        self.status.set_text(message);
    }
    pub fn command(&self, m: &str, p: Value) {
        if let Err(e) = self.worker.send(Command::Rpc(m.into(), p)) {
            self.warn(&e);
        }
    }
    pub fn input(&self, text: &str) {
        self.command(
            "client.input",
            json!({"pane":*self.target.borrow(),"kind":"keyboard","text":text}),
        );
    }
    pub fn paste(&self, text: String) {
        if text.len() > ruwac_clipboard::MAX_DESKTOP_TEXT
            || text.contains('\0')
            || text.contains("\x1b[201~")
        {
            self.status
                .set_text("Paste rejected: text cap, NUL or end marker");
            return;
        }
        if let Err(e) = self
            .worker
            .send(Command::Paste(self.target.borrow().clone(), text))
        {
            self.warn(&e);
        }
    }
    pub fn update(&self) {
        let mut s = self.worker.state.lock().unwrap();
        if let Some(error) = s.error.take() {
            self.warn(&error);
        }
        if let Some(frame) = s.frame.take() {
            if self
                .frame
                .borrow()
                .as_ref()
                .and_then(|v| v["revision"].as_u64())
                .is_some_and(|r| frame["revision"].as_u64().unwrap_or(0) < r)
            {
                self.status.set_text("Stale frame rejected");
                return;
            }
            let panes = frame["panes"].as_array().unwrap();
            if !panes
                .iter()
                .any(|p| p["id"].as_str() == Some(self.target.borrow().as_str()))
            {
                *self.target.borrow_mut() = frame["active_pane"].as_str().unwrap_or("").into();
            }
            if self.warning.borrow().is_none() {
                self.status.set_text(&format!(
                    "{} — Ctrl+A prefix · Shift+drag selects · PTYs persist",
                    frame["name"].as_str().unwrap_or("Terminal")
                ));
            }
            *self.frame.borrow_mut() = Some(frame);
            self.area.queue_draw();
        }
        if let Some((method, v)) = s.reply.take() {
            if method == "pane.capture" {
                self.copy
                    .buffer()
                    .set_text(v["text"].as_str().unwrap_or(""));
                self.stack.set_visible_child_name("copy");
                self.copy.grab_focus();
            } else if method == "window.list" {
                if let Some(index) = self.window_index.take() {
                    if let Some(window) = v.as_array().and_then(|a| a.get(index)) {
                        self.command("window.select", json!({"window":window["id"]}));
                    } else {
                        self.status.set_text("No window at that index");
                    }
                } else {
                    self.status.set_text(
                        &v.as_array()
                            .map(|a| {
                                a.iter()
                                    .enumerate()
                                    .map(|(i, w)| {
                                        format!("{i}: {}", w["name"].as_str().unwrap_or("Terminal"))
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" · ")
                            })
                            .unwrap_or_default(),
                    );
                }
            } else if method == "buffer.get" {
                drop(s);
                self.paste(v["text"].as_str().unwrap_or("").into());
            }
        }
    }
    pub fn focus(&self) {
        self.broker.borrow_mut().focus(Some(&self.session));
        self.area.grab_focus();
    }
    pub fn focus_at(&self, x: f64, y: f64) {
        if let Some((p, _, _)) = self.point(x, y) {
            if p != *self.target.borrow() {
                *self.selection.borrow_mut() = None;
            }
            *self.target.borrow_mut() = p.clone();
            self.command("pane.select", json!({"pane":p}));
        }
        self.focus();
    }
    fn point(&self, x: f64, y: f64) -> Option<(String, usize, usize)> {
        let frame = self.frame.borrow();
        let frame = frame.as_ref()?;
        let mut offset = 0.0;
        for p in frame["panes"].as_array()? {
            let rows = p["grid"]["rows"].as_u64()? as usize;
            let cols = p["grid"]["cols"].as_u64()? as usize;
            let bottom = offset + rows as f64 * CH;
            if y >= offset && y < bottom && x >= 0.0 && x < cols as f64 * CW {
                return Some((
                    p["id"].as_str()?.into(),
                    ((y - offset) / CH) as usize,
                    (x / CW) as usize,
                ));
            }
            offset = bottom + CH;
        }
        None
    }
    fn grid(&self) -> Option<Value> {
        self.frame
            .borrow()
            .as_ref()?
            .get("panes")?
            .as_array()?
            .iter()
            .find(|p| p["id"].as_str() == Some(self.target.borrow().as_str()))
            .map(|p| p["grid"].clone())
    }
    fn mouse_enabled(&self) -> bool {
        self.grid()
            .is_some_and(|g| g["mouse"] == true && g["mouse_sgr"] == true)
    }
    pub fn mouse(&self, button: u32, x: f64, y: f64, release: bool) {
        if !self.mouse_enabled() {
            return;
        }
        if let Some((pane, r, c)) = self.point(x, y) {
            if pane != *self.target.borrow() {
                return;
            }
            self.command("client.input",json!({"pane":pane,"kind":"mouse","text":format!("\x1b[<{button};{};{}{}",c+1,r+1,if release{'m'}else{'M'})}));
        }
    }
    pub fn click(
        self: &Rc<Self>,
        x: f64,
        y: f64,
        button: u32,
        mods: gdk::ModifierType,
        release: bool,
    ) {
        if !release {
            self.focus_at(x, y);
        }
        if self.mouse_enabled() && !mods.contains(gdk::ModifierType::SHIFT_MASK) {
            self.mouse(
                match button {
                    1 => 0,
                    2 => 1,
                    3 => 2,
                    _ => return,
                },
                x,
                y,
                release,
            );
        } else if button == 2 && !release {
            self.paste_clipboard(true);
        }
    }
    pub fn selected(&self) -> String {
        if self.stack.visible_child_name().as_deref() == Some("copy") {
            let b = self.copy.buffer();
            return b
                .selection_bounds()
                .map(|(a, z)| b.text(&a, &z, true).to_string())
                .unwrap_or_default();
        }
        let Some((mut a, mut z)) = *self.selection.borrow() else {
            return String::new();
        };
        if a > z {
            std::mem::swap(&mut a, &mut z);
        }
        let Some(g) = self.grid() else {
            return String::new();
        };
        let Some(rows) = g["cells"].as_array() else {
            return String::new();
        };
        let mut result = String::new();
        for r in a.0..=z.0 {
            let Some(cells) = rows.get(r).and_then(Value::as_array) else {
                break;
            };
            let start = if r == a.0 { a.1 } else { 0 };
            let end = if r == z.0 { z.1 + 1 } else { cells.len() };
            for cell in cells.iter().take(end).skip(start) {
                if cell[1] != true {
                    result.push_str(cell[0].as_str().unwrap_or(""));
                }
            }
            if r != z.0 {
                result.push('\n');
            }
        }
        result
    }
    pub fn copy_clipboard(&self, primary: bool) {
        if !self.broker.borrow().desktop_enabled() {
            self.status.set_text("Desktop clipboard disabled");
            return;
        }
        let text = self.selected();
        if text.is_empty() {
            self.status.set_text("Select terminal text before copying");
            return;
        }
        if text.len() > ruwac_clipboard::MAX_DESKTOP_TEXT {
            self.status.set_text("Selected text exceeds clipboard cap");
            return;
        }
        self.broker.borrow_mut().local_copy();
        let display = self.area.display();
        if primary {
            display.primary_clipboard().set_text(&text);
        } else {
            display.clipboard().set_text(&text);
        }
        self.status.set_text(if primary {
            "Selection offered to PRIMARY"
        } else {
            "Selection copied to CLIPBOARD"
        });
    }
    pub fn paste_clipboard(self: &Rc<Self>, primary: bool) {
        let display = self.area.display();
        let clip = if primary {
            display.primary_clipboard()
        } else {
            display.clipboard()
        };
        let epoch = self.broker.borrow().epoch();
        let pane = self.target.borrow().clone();
        let weak = Rc::downgrade(self);
        clipboard::read_text(
            &clip,
            self.broker.clone(),
            epoch,
            if primary {
                ruwac_clipboard::ClipScope::Primary
            } else {
                ruwac_clipboard::ClipScope::Clipboard
            },
            std::time::Duration::ZERO,
            move |result| {
                if let Some(v) = weak.upgrade() {
                    if v.detached.get() || *v.target.borrow() != pane {
                        v.status.set_text("Paste cancelled: target changed");
                        return;
                    }
                    match result {
                        Ok(text) => v.paste(text),
                        Err(e) => v.status.set_text(&e),
                    }
                }
            },
        );
    }
    fn internal_copy(&self) {
        let text = self.selected();
        if text.is_empty() {
            self.status.set_text("Select text in copy mode, then Enter");
            return;
        }
        self.command("buffer.set", json!({"session":self.session,"text":text}));
        self.stack.set_visible_child_name("terminal");
        self.area.grab_focus();
    }
    pub fn key(self: &Rc<Self>, key: gdk::Key, mods: gdk::ModifierType) {
        if self.detached.get() {
            return;
        }
        let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
        let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
        let ch = key.to_unicode();
        if ctrl && shift {
            match ch.map(|c| c.to_ascii_lowercase()) {
                Some('c') => {
                    self.copy_clipboard(false);
                    return;
                }
                Some('v') => {
                    self.paste_clipboard(false);
                    return;
                }
                _ => (),
            }
        }
        if self.prefix.replace(false) {
            match ch {Some('d')=>{self.close();},Some('c')=>self.command("window.create",json!({"session":self.session})),Some('n'|'p')=>self.command("window.cycle",json!({"session":self.session,"previous":ch==Some('p')})),Some('a')=>self.input("\x01"),Some('S')=>{if let Some(f)=self.frame.borrow().as_ref(){self.command("pane.create",json!({"window":f["window"]}));}},Some('[')=>self.command("pane.capture",json!({"pane":*self.target.borrow(),"lines":2000})),Some(']')=>self.command("buffer.get",json!({"session":self.session})),Some('?')=>self.status.set_text("c/n/p/0–9 windows · a literal ^A · d detach · [ select/search · ] internal paste · S horizontal split · Tab pane · |/: pending"),Some('"')=>self.command("window.list",json!({"session":self.session})),Some('|'|':')=>self.status.set_text("Vertical split and command prompt are not implemented"),Some(c) if c.is_ascii_digit()=>{self.window_index.set(c.to_digit(10).map(|n|n as usize));self.command("window.list",json!({"session":self.session}));},_ if key==gdk::Key::Tab=>{if let Some(f)=self.frame.borrow().as_ref(){if let Some(p)=f["panes"].as_array(){let i=p.iter().position(|p|p["id"].as_str()==Some(self.target.borrow().as_str())).unwrap_or(0);let next=p[(i+1)%p.len()]["id"].as_str().unwrap().to_string();*self.target.borrow_mut()=next.clone();self.command("pane.select",json!({"pane":next}));}}},_=>self.status.set_text("Unknown Screen prefix command")};
            return;
        }
        if ctrl && ch.is_some_and(|c| c.eq_ignore_ascii_case(&'a')) {
            self.prefix.set(true);
            return;
        }
        let mut text = match key {
            gdk::Key::Return | gdk::Key::KP_Enter => "\r".into(),
            gdk::Key::BackSpace => "\x7f".into(),
            gdk::Key::Tab => "\t".into(),
            gdk::Key::Escape => "\x1b".into(),
            gdk::Key::Up => "\x1b[A".into(),
            gdk::Key::Down => "\x1b[B".into(),
            gdk::Key::Left => "\x1b[D".into(),
            gdk::Key::Right => "\x1b[C".into(),
            gdk::Key::Home => "\x1b[H".into(),
            gdk::Key::End => "\x1b[F".into(),
            gdk::Key::Delete => "\x1b[3~".into(),
            gdk::Key::Page_Up => "\x1b[5~".into(),
            gdk::Key::Page_Down => "\x1b[6~".into(),
            _ => ch.map(|c| c.to_string()).unwrap_or_default(),
        };
        if self.grid().is_some_and(|g| g["application_cursor"] == true)
            && matches!(
                key,
                gdk::Key::Up
                    | gdk::Key::Down
                    | gdk::Key::Left
                    | gdk::Key::Right
                    | gdk::Key::Home
                    | gdk::Key::End
            )
        {
            text = text.replacen("\x1b[", "\x1bO", 1);
        }
        if ctrl {
            if let Some(c) = ch {
                if ('@'..='_').contains(&c.to_ascii_uppercase()) {
                    text = (((c as u8) & 31) as char).to_string();
                }
            }
        }
        if text.contains('\0') {
            self.status.set_text("NUL key unsupported");
            return;
        }
        if mods.contains(gdk::ModifierType::ALT_MASK) {
            text.insert(0, '\x1b');
        }
        if !text.is_empty() {
            self.input(&text);
        }
    }
    pub fn close(&self) {
        self.detached.set(true);
        self.worker.close();
        self.broker.borrow_mut().disconnect(&self.session);
        self.status.set_text("Detached; daemon PTYs preserved");
    }
    fn draw(&self, area: &gtk::DrawingArea, cr: &gtk::cairo::Context, _width: i32, _height: i32) {
        self.drawn.set(self.drawn.get() + 1);
        cr.set_source_rgb(0.04, 0.04, 0.05);
        let _ = cr.paint();
        let frame = self.frame.borrow();
        let Some(frame) = frame.as_ref() else {
            return;
        };
        let Some(panes) = frame["panes"].as_array() else {
            return;
        };
        let mut y = 0.0;
        let layout = area.create_pango_layout(None);
        let normal = gtk::pango::FontDescription::from_string("Monospace 11");
        let bold = gtk::pango::FontDescription::from_string("Monospace Bold 11");
        for p in panes {
            let g = &p["grid"];
            if let Some(rows) = g["cells"].as_array() {
                for (r, row) in rows.iter().enumerate() {
                    if let Some(cells) = row.as_array() {
                        for (c, cell) in cells.iter().enumerate() {
                            let mut bg = color(&cell[3], false);
                            if cell[5] == true {
                                bg = color(&cell[2], true);
                            }
                            if p["id"].as_str() == Some(self.target.borrow().as_str())
                                && self.selection.borrow().is_some_and(|(a, z)| {
                                    let (lo, hi) = if a <= z { (a, z) } else { (z, a) };
                                    lo <= (r, c) && (r, c) <= hi
                                })
                            {
                                bg = (0.18, 0.28, 0.48);
                            }
                            cr.set_source_rgb(bg.0, bg.1, bg.2);
                            cr.rectangle(c as f64 * CW, y + r as f64 * CH, CW, CH);
                            let _ = cr.fill();
                        }
                        for (c, cell) in cells.iter().enumerate() {
                            if cell[1] != true {
                                let fg = if cell[5] == true {
                                    color(&cell[3], false)
                                } else {
                                    color(&cell[2], true)
                                };
                                layout.set_font_description(Some(if cell[4] == true {
                                    &bold
                                } else {
                                    &normal
                                }));
                                layout.set_text(cell[0].as_str().unwrap_or(""));
                                cr.set_source_rgb(fg.0, fg.1, fg.2);
                                cr.move_to(c as f64 * CW, y + r as f64 * CH);
                                pangocairo::functions::show_layout(cr, &layout);
                            }
                        }
                    }
                }
            }
            if p["id"].as_str() == Some(self.target.borrow().as_str())
                && g["cursor"]["visible"] == true
            {
                cr.set_source_rgb(0.5, 0.7, 0.9);
                cr.rectangle(
                    g["cursor"]["col"].as_u64().unwrap_or(0) as f64 * CW,
                    y + g["cursor"]["row"].as_u64().unwrap_or(0) as f64 * CH,
                    CW,
                    CH,
                );
                let _ = cr.stroke();
            }
            y += g["rows"].as_u64().unwrap_or(0) as f64 * CH + CH;
        }
    }
    pub fn select_grid_fixture(&self) {
        self.stack.set_visible_child_name("terminal");
        *self.selection.borrow_mut() = Some(((0, 0), (0, 19)));
        self.area.queue_draw();
    }
    pub fn select_fixture(&self) {
        self.copy
            .buffer()
            .set_text("Native selection žółć 🧪\nline\t'quotes'");
        let b = self.copy.buffer();
        b.select_range(&b.start_iter(), &b.end_iter());
        self.stack.set_visible_child_name("copy");
    }
}
fn color(v: &Value, fg: bool) -> (f64, f64, f64) {
    if let Some(a) = v.as_array() {
        return (
            a[0].as_f64().unwrap_or(0.0) / 255.0,
            a[1].as_f64().unwrap_or(0.0) / 255.0,
            a[2].as_f64().unwrap_or(0.0) / 255.0,
        );
    }
    let n = v.as_u64().unwrap_or(256);
    if n >= 256 {
        return if fg {
            (0.9, 0.9, 0.9)
        } else {
            (0.04, 0.04, 0.05)
        };
    }
    let rgb = if n < 16 {
        let p = [
            0x000000, 0xcd0000, 0x00cd00, 0xcdcd00, 0x0000ee, 0xcd00cd, 0x00cdcd, 0xe5e5e5,
            0x7f7f7f, 0xff0000, 0x00ff00, 0xffff00, 0x5c5cff, 0xff00ff, 0x00ffff, 0xffffff,
        ];
        p[n as usize]
    } else if n >= 232 {
        let l = 8 + 10 * (n - 232);
        l * 0x010101
    } else {
        let n = n - 16;
        let l = [0, 95, 135, 175, 215, 255];
        l[(n / 36) as usize] * 65536 + l[((n / 6) % 6) as usize] * 256 + l[(n % 6) as usize]
    };
    (
        ((rgb >> 16) & 255) as f64 / 255.0,
        ((rgb >> 8) & 255) as f64 / 255.0,
        (rgb & 255) as f64 / 255.0,
    )
}
