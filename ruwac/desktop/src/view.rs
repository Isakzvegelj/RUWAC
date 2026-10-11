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
    pressed: Cell<Option<u32>>,
    mouse_position: Cell<(f64, f64)>,
    pub copy: gtk::TextView,
    copy_anchor: Cell<Option<i32>>,
    pub search: gtk::SearchEntry,
    early: RefCell<Vec<String>>,
    navigation: Cell<bool>,
    expected_window: RefCell<Option<String>>,
    divider: RefCell<Option<(String, bool, f64, f64, f64)>>,
    remaps: std::collections::HashMap<String, String>,
    shortcuts: std::collections::HashMap<String, String>,
    wheel_delta: Cell<(f64, f64)>,
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
            .content_width(200)
            .content_height(80)
            .build();
        let stack = gtk::Stack::builder().hexpand(true).vexpand(true).build();
        stack.set_transition_type(gtk::StackTransitionType::None);
        stack.add_named(&area, Some("terminal"));
        let copy = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::None)
            .build();
        // Internal copy selection must not claim compositor PRIMARY implicitly.
        copy.connect_realize(|view| {
            view.buffer()
                .remove_selection_clipboard(&view.display().primary_clipboard());
        });
        let scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&copy)
            .build();
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search canonical history (bounded 8 MiB copy view)"));
        let cb = copy.buffer();
        search.connect_changed(move |entry| {
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
            pressed: Cell::new(None),
            mouse_position: Cell::new((0.0, 0.0)),
            copy,
            search,
            copy_anchor: Cell::new(None),
            early: RefCell::new(Vec::new()),
            navigation: Cell::new(false),
            expected_window: RefCell::new(None),
            divider: RefCell::new(None),
            shortcuts: std::env::var("RUWAC_SHORTCUTS")
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            wheel_delta: Cell::new((0.0, 0.0)),
            remaps: std::env::var("RUWAC_KEYMAP")
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
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
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let w = Rc::downgrade(&v);
        key.connect_key_pressed(move |_, key, _, mods| {
            let Some(v) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if v.copy_key(key, mods) {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
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
                if let Some(divider) = v.find_divider(x, y) {
                    *v.divider.borrow_mut() = Some(divider);
                    return;
                }
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
                if v.divider.borrow().is_some() {
                    return;
                }
                if v.mouse_enabled()
                    && !g
                        .current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    // Motion reports are emitted once by the motion controller.
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
        drag.connect_drag_end(move |g, dx, dy| {
            if let Some(v) = w.upgrade() {
                if let Some((id, vertical, start, size, initial)) = v.divider.borrow_mut().take() {
                    let movement = if vertical { dx } else { dy };
                    let ratio =
                        ((initial + movement - start) / size * 1000.0).clamp(100.0, 900.0) as u16;
                    if let Some(f) = v.frame.borrow().as_ref() {
                        v.command(
                            "pane.layout",
                            json!({"window":f["window"],"split":id,"ratio":ratio}),
                        );
                    }
                    return;
                }
                if !v.mouse_enabled()
                    || g.current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    v.copy_clipboard(true);
                }
            }
        });
        v.area.add_controller(drag);
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&v);
        motion.connect_motion(move |g, x, y| {
            if let Some(v) = w.upgrade() {
                let mods = g.current_event_state();
                if mods.contains(gdk::ModifierType::SHIFT_MASK) {
                    return;
                }
                if let Some(grid) = v.grid() {
                    if grid["mouse_motion"] == true
                        || (grid["mouse_drag"] == true && v.pressed.get().is_some())
                    {
                        v.mouse_with_mods(32 + v.pressed.get().unwrap_or(3), x, y, false, mods);
                    }
                }
            }
        });
        v.area.add_controller(motion);
        let focus = gtk::EventControllerFocus::new();
        let w = Rc::downgrade(&v);
        focus.connect_leave(move |_| {
            if let Some(v) = w.upgrade() {
                v.release_mouse();
                if v.prefix.replace(false) {
                    v.input("\x01");
                }
            }
        });
        v.area.add_controller(focus);
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        let w = Rc::downgrade(&v);
        wheel.connect_scroll(move |g, dx, dy| {
            if let Some(v) = w.upgrade() {
                if v.mouse_enabled()
                    && !g
                        .current_event_state()
                        .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    if let Some((x, y)) = g.current_event().and_then(|e| e.position()) {
                        let (ax, ay) = v.wheel_delta.get();
                        let (mut ax, mut ay) = (ax + dx.clamp(-8.0, 8.0), ay + dy.clamp(-8.0, 8.0));
                        while ay.abs() >= 1.0 {
                            v.mouse_with_mods(
                                if ay < 0.0 { 64 } else { 65 },
                                x,
                                y,
                                false,
                                g.current_event_state(),
                            );
                            ay -= ay.signum();
                        }
                        while ax.abs() >= 1.0 {
                            v.mouse_with_mods(
                                if ax < 0.0 { 66 } else { 67 },
                                x,
                                y,
                                false,
                                g.current_event_state(),
                            );
                            ax -= ax.signum();
                        }
                        v.wheel_delta.set((ax, ay));
                    }
                } else {
                    v.open_history();
                }
            }
            glib::Propagation::Stop
        });
        v.area.add_controller(wheel);
        let w = Rc::downgrade(&v);
        v.area.connect_resize(move|_,width,height|if let Some(v)=w.upgrade(){v.command("client.resize",json!({"rows":((height as f64/CH)as i32-2).clamp(4,100),"cols":((width as f64/CW)as i32).clamp(10,180)}));});
        v
    }
    pub fn show_terminal(&self) {
        self.stack.set_visible_child_name("terminal");
        self.area.grab_focus();
    }
    pub fn open_history(&self) {
        if let Err(e) = self
            .worker
            .send(Command::History(self.target.borrow().clone()))
        {
            self.warn(&e);
        }
        self.status
            .set_text("Loading bounded canonical scrollback…");
    }
    fn navigate(&self, method: &str, params: Value) {
        self.release_mouse();
        self.navigation.set(true);
        *self.selection.borrow_mut() = None;
        self.command(method, params);
    }
    fn split(&self, vertical: bool) {
        self.release_mouse();
        if let Some(f) = self.frame.borrow().as_ref() {
            self.navigation.set(true);
            self.command("pane.create",json!({"window":f["window"],"pane":*self.target.borrow(),"orientation":if vertical{"vertical"}else{"horizontal"}}));
        }
    }
    fn zoom(&self) {
        if let Some(f) = self.frame.borrow().as_ref() {
            self.command(
                "pane.layout",
                json!({"window":f["window"],"zoom":f["zoom"]!=true}),
            );
        }
    }
    fn binding(&self, key: gdk::Key, mods: gdk::ModifierType, name: &str, default: &str) -> bool {
        let text = self
            .shortcuts
            .get(name)
            .map(String::as_str)
            .unwrap_or(default);
        let Some((wanted, flags)) = gtk::accelerator_parse(text) else {
            return false;
        };
        let mask = gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SHIFT_MASK
            | gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK;
        (key == wanted
            || key
                .to_unicode()
                .zip(wanted.to_unicode())
                .is_some_and(|(a, b)| a.to_lowercase().to_string() == b.to_lowercase().to_string()))
            && (mods & mask) == (flags & mask)
    }
    fn show_help(&self) {
        let text = format!(
            "Prefix: {}\nDesktop copy: {}\nDesktop paste: {}\n\nPrefix commands: double prefix previous window · c new · n/p windows · 0–9 select · \" chooser · S horizontal / | vertical split · Tab pane · z zoom/restore · [ copy/search · ] internal paste · : command · d detach · a literal prefix.\n\nCopy mode: arrows/hjkl, Page Up/Down, g/G, Space mark, Enter save internally, / search, Escape exit. Drag dividers to resize. Shift+drag overrides application mouse.\n\nCustom prefix command mappings: {}",
            self.shortcuts
                .get("prefix")
                .map(String::as_str)
                .unwrap_or("Ctrl+A"),
            self.shortcuts
                .get("copy")
                .map(String::as_str)
                .unwrap_or("Ctrl+Shift+C"),
            self.shortcuts
                .get("paste")
                .map(String::as_str)
                .unwrap_or("Ctrl+Shift+V"),
            self.remaps
                .iter()
                .map(|(k, v)| format!("{k} → {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let label = gtk::Label::new(Some(&text));
        label.set_wrap(true);
        label.set_selectable(true);
        label.set_margin_start(20);
        label.set_margin_end(20);
        label.set_margin_top(20);
        label.set_margin_bottom(20);
        let window = gtk::Window::builder()
            .title("RUWAC — current shortcuts")
            .default_width(600)
            .default_height(360)
            .child(&label)
            .build();
        if let Some(parent) = self.parent() {
            window.set_transient_for(Some(&parent));
        }
        window.present();
    }
    fn parent(&self) -> Option<gtk::Window> {
        self.root
            .root()
            .and_then(|r| r.downcast::<gtk::Window>().ok())
    }
    fn show_chooser(self: &Rc<Self>, windows: &Value) {
        let dialog = gtk::Dialog::builder()
            .title("Windows — choose a persistent window")
            .modal(true)
            .default_width(480)
            .build();
        if let Some(parent) = self.parent() {
            dialog.set_transient_for(Some(&parent));
        }
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        let mut ids = Vec::new();
        if let Some(windows) = windows.as_array() {
            for (i, w) in windows.iter().enumerate() {
                ids.push(w["id"].as_str().unwrap_or("").to_string());
                let label = gtk::Label::new(Some(&format!(
                    "{i}    {}",
                    w["name"].as_str().unwrap_or("Shell")
                )));
                label.set_xalign(0.0);
                label.set_margin_top(8);
                label.set_margin_bottom(8);
                list.append(&label);
            }
        }
        dialog.content_area().append(&list);
        let weak = Rc::downgrade(self);
        let d = dialog.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(v) = weak.upgrade() {
                if let Some(id) = ids.get(row.index() as usize) {
                    v.navigate("window.select", json!({"window":id}));
                }
            }
            d.close();
        });
        dialog.connect_response(|d, _| d.close());
        dialog.present();
        list.grab_focus();
        if let Some(row) = list.row_at_index(0) {
            list.select_row(Some(&row));
            row.grab_focus();
        }
    }
    fn show_prompt(self: &Rc<Self>) {
        let dialog = gtk::Dialog::builder()
            .title("RUWAC command")
            .modal(true)
            .default_width(520)
            .build();
        if let Some(p) = self.parent() {
            dialog.set_transient_for(Some(&p));
        }
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some("new · next · prev · select N · split horizontal/vertical · zoom · copy · paste · detach · help"));
        dialog.content_area().append(&entry);
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Run", gtk::ResponseType::Accept);
        let weak = Rc::downgrade(self);
        let e = entry.clone();
        dialog.connect_response(move |d, r| {
            if r == gtk::ResponseType::Accept {
                if let Some(v) = weak.upgrade() {
                    v.run_command(e.text().as_str());
                }
            }
            d.close();
        });
        let d = dialog.clone();
        entry.connect_activate(move |_| d.response(gtk::ResponseType::Accept));
        dialog.present();
        entry.grab_focus();
    }
    pub fn run_command(self: &Rc<Self>, command: &str) {
        let words: Vec<_> = command.split_whitespace().collect();
        match words.as_slice(){
            ["new"]=>self.navigate("window.create",json!({"session":self.session})),
            ["next"]|["prev"]=>self.navigate("window.cycle",json!({"session":self.session,"previous":words[0]=="prev"})),
            ["select",n]=>match n.parse::<usize>(){Ok(n)=>{self.window_index.set(Some(n));self.command("window.list",json!({"session":self.session}));},Err(_)=>self.status.set_text("select requires a nonnegative window index")},
            ["split"]|["split","horizontal"]=>self.split(false),["split","vertical"]=>self.split(true),
            ["zoom"]=>self.zoom(),["copy"]=>self.open_history(),["paste"]=>self.command("buffer.get",json!({"session":self.session})),
            ["detach"]=>self.close(),["help"]=>self.show_help(),
            _=>self.warn("Unknown command. Use: new, next, prev, select N, split horizontal/vertical, zoom, copy, paste, detach, help"),
        }
    }
    pub fn copy_key(self: &Rc<Self>, key: gdk::Key, mods: gdk::ModifierType) -> bool {
        if key == gdk::Key::Escape {
            self.show_terminal();
            return true;
        }
        if key == gdk::Key::Return {
            self.internal_copy();
            return true;
        }
        if self.binding(key, mods, "copy", "<Control><Shift>c") {
            self.copy_clipboard(false);
            return true;
        }
        if self.binding(key, mods, "paste", "<Control><Shift>v") {
            self.paste_clipboard(false);
            return true;
        }
        if key == gdk::Key::slash {
            self.search.grab_focus();
            return true;
        }
        let b = self.copy.buffer();
        let mut pos = b.iter_at_mark(&b.get_insert());
        if key == gdk::Key::space {
            if self.copy_anchor.get().is_some() {
                self.internal_copy();
            } else {
                self.copy_anchor.set(Some(pos.offset()));
                self.status
                    .set_text("Copy mark set; navigate then Enter to save internal buffer");
            }
            return true;
        }
        let moved = match key {
            gdk::Key::Left | gdk::Key::h => pos.backward_cursor_position(),
            gdk::Key::Right | gdk::Key::l => pos.forward_cursor_position(),
            gdk::Key::Up | gdk::Key::k => pos.backward_line(),
            gdk::Key::Down | gdk::Key::j => pos.forward_line(),
            gdk::Key::Page_Up => pos.backward_lines(20),
            gdk::Key::Page_Down => pos.forward_lines(20),
            gdk::Key::Home => {
                pos.set_line_offset(0);
                true
            }
            gdk::Key::End => pos.forward_to_line_end(),
            gdk::Key::g => {
                pos = b.start_iter();
                true
            }
            gdk::Key::G => {
                pos = b.end_iter();
                true
            }
            _ => return false,
        };
        if moved {
            if let Some(anchor) = self.copy_anchor.get() {
                b.select_range(&pos, &b.iter_at_offset(anchor));
            } else {
                b.place_cursor(&pos);
            }
            self.copy.scroll_to_iter(&mut pos, 0.1, false, 0.0, 0.0);
        }
        true
    }
    fn find_divider(&self, x: f64, y: f64) -> Option<(String, bool, f64, f64, f64)> {
        fn walk(
            node: &Value,
            r: (f64, f64, f64, f64),
            x: f64,
            y: f64,
        ) -> Option<(String, bool, f64, f64, f64)> {
            let id = node["id"].as_str()?;
            let vertical = node["orientation"] == "vertical";
            let ratio = node["ratio"].as_f64().unwrap_or(500.0) / 1000.0;
            let (rx, ry, w, h) = r;
            let size = if vertical { w } else { h };
            let start = if vertical { rx } else { ry };
            let initial = start
                + ((size / (if vertical { CW } else { CH }) * ratio).floor())
                    * (if vertical { CW } else { CH });
            if x >= rx
                && x < rx + w
                && y >= ry
                && y < ry + h
                && (if vertical {
                    (x - initial).abs() < 4.0
                } else {
                    (y - initial).abs() < 4.0
                })
            {
                return Some((id.into(), vertical, start, size, initial));
            }
            let (a, b) = if vertical {
                (
                    (rx, ry, initial - rx, h),
                    (initial, ry, rx + w - initial, h),
                )
            } else {
                (
                    (rx, ry, w, initial - ry),
                    (rx, initial, w, ry + h - initial),
                )
            };
            walk(&node["first"], a, x, y).or_else(|| walk(&node["second"], b, x, y))
        }
        let f = self.frame.borrow();
        let f = f.as_ref()?;
        if f["zoom"] == true {
            return None;
        }
        let panes = f["panes"].as_array()?;
        let width = panes
            .iter()
            .map(|p| {
                p["rect"]["x"].as_f64().unwrap_or(0.0) + p["rect"]["cols"].as_f64().unwrap_or(0.0)
            })
            .fold(0.0, f64::max)
            * CW;
        let height = panes
            .iter()
            .map(|p| {
                p["rect"]["y"].as_f64().unwrap_or(0.0) + p["rect"]["rows"].as_f64().unwrap_or(0.0)
            })
            .fold(0.0, f64::max)
            * CH;
        walk(&f["layout"], (0.0, 0.0, width, height), x, y)
    }

    // Test-only caller reaches the same compiled GTK gesture callbacks as pointer drags.
    pub fn fixture_root_drag(&self) -> Value {
        let (x, y, dx, dy) = {
            let frame = self.frame.borrow();
            let Some(f) = frame.as_ref() else {
                return json!({"error":"no frame"});
            };
            let Some(panes) = f["panes"].as_array() else {
                return json!({"error":"no panes"});
            };
            let width = panes
                .iter()
                .map(|p| {
                    p["rect"]["x"].as_f64().unwrap_or(0.) + p["rect"]["cols"].as_f64().unwrap_or(0.)
                })
                .fold(0., f64::max)
                * CW;
            let height = panes
                .iter()
                .map(|p| {
                    p["rect"]["y"].as_f64().unwrap_or(0.) + p["rect"]["rows"].as_f64().unwrap_or(0.)
                })
                .fold(0., f64::max)
                * CH;
            let ratio = f["layout"]["ratio"].as_f64().unwrap_or(500.) / 1000.;
            if f["layout"]["orientation"] == "vertical" {
                (
                    (width / CW * ratio).floor() * CW,
                    height / 2.,
                    width * 0.1,
                    0.,
                )
            } else {
                (
                    width / 2.,
                    (height / CH * ratio).floor() * CH,
                    0.,
                    height * 0.1,
                )
            }
        };
        let found = self.find_divider(x, y);
        let controllers = self.area.observe_controllers();
        for i in 0..controllers.n_items() {
            if let Some(gesture) = controllers
                .item(i)
                .and_then(|o| o.downcast::<gtk::GestureDrag>().ok())
            {
                gesture.emit_by_name::<()>("drag-begin", &[&x, &y]);
                gesture.emit_by_name::<()>("drag-end", &[&dx, &dy]);
            }
        }
        json!({"coordinates":[x,y,dx,dy],"divider":found,"controllers":controllers.n_items()})
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
        if self.target.borrow().is_empty() || self.navigation.get() {
            let mut pending = self.early.borrow_mut();
            if pending.iter().map(String::len).sum::<usize>() + text.len() > 256 * 1024 {
                self.warn("Pending input budget reached; input not queued");
            } else {
                pending.push(text.into());
            }
            return;
        }
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
    pub fn update(self: &Rc<Self>) {
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
                let active = panes.iter().find(|p| p["id"] == frame["active_pane"]);
                let state = match active {
                    Some(p) if p["restored"] == true => {
                        "Restored history · process ended · New terminal starts a session"
                    }
                    Some(p) if p["ended"] == true => "Process ended · history remains available",
                    _ => "Ctrl+A prefix · Shift+drag selects · PTYs persist",
                };
                self.status.set_text(&format!(
                    "{} — {state}",
                    frame["name"].as_str().unwrap_or("Terminal")
                ));
            }
            if self.navigation.get() {
                if self.expected_window.borrow().as_deref() == frame["window"].as_str() {
                    *self.target.borrow_mut() = frame["active_pane"].as_str().unwrap_or("").into();
                    self.navigation.set(false);
                    *self.expected_window.borrow_mut() = None;
                }
            }
            *self.frame.borrow_mut() = Some(frame);
            if !self.navigation.get() && !self.target.borrow().is_empty() {
                let pending = std::mem::take(&mut *self.early.borrow_mut());
                for text in pending {
                    self.input(&text);
                }
            }
            self.area.queue_draw();
        }
        if let Some((method, v)) = s.reply.pop_front() {
            if method == "pane.create" {
                if let Some(pane) = v["pane"].as_str() {
                    *self.target.borrow_mut() = pane.into();
                    self.navigation.set(false);
                    let pending = std::mem::take(&mut *self.early.borrow_mut());
                    for text in pending {
                        self.input(&text);
                    }
                }
            }
            if method == "window.create"
                || method == "window.select"
                || method == "window.cycle"
                || method == "window.previous"
            {
                *self.expected_window.borrow_mut() = v["window"]
                    .as_str()
                    .or(v["selected"].as_str())
                    .map(str::to_string);
                if let Some(frame) = self.frame.borrow().as_ref() {
                    if self.expected_window.borrow().as_deref() == frame["window"].as_str() {
                        *self.target.borrow_mut() =
                            frame["active_pane"].as_str().unwrap_or("").into();
                        self.navigation.set(false);
                        *self.expected_window.borrow_mut() = None;
                        let pending = std::mem::take(&mut *self.early.borrow_mut());
                        for text in pending {
                            self.input(&text);
                        }
                    }
                }
            }
            if method == "pane.history" || method == "pane.capture" {
                if v["truncated"] == true {
                    self.warn("Copy view reached its 8 MiB limit; only the loaded history can be selected");
                }
                self.copy
                    .buffer()
                    .set_text(v["text"].as_str().unwrap_or(""));
                self.stack.set_visible_child_name("copy");
                self.copy_anchor.set(None);
                self.copy
                    .buffer()
                    .place_cursor(&self.copy.buffer().end_iter());
                self.copy.set_cursor_visible(true);
                self.copy.grab_focus();
            } else if method == "window.list" {
                if let Some(index) = self.window_index.take() {
                    if let Some(window) = v.as_array().and_then(|a| a.get(index)) {
                        self.navigate("window.select", json!({"window":window["id"]}));
                    } else {
                        self.status.set_text("No window at that index");
                    }
                } else {
                    self.show_chooser(&v);
                }
            } else if method == "buffer.get" {
                drop(s);
                self.paste(v["text"].as_str().unwrap_or("").into());
            }
        }
    }
    pub fn focus(&self) {
        if self.stack.visible_child_name().as_deref() == Some("copy") {
            self.copy.grab_focus();
        } else {
            self.area.grab_focus();
        }
        let foreground = self
            .root
            .root()
            .and_then(|r| r.downcast::<gtk::Window>().ok())
            .is_some_and(|w| w.is_active());
        if foreground {
            self.broker.borrow_mut().focus(Some(&self.session));
        } else if self.broker.borrow().active() == Some(self.session.as_str()) {
            self.broker.borrow_mut().focus(None);
        }
    }
    pub fn focus_at(&self, x: f64, y: f64) {
        if let Some((p, _, _)) = self.point(x, y) {
            if p != *self.target.borrow() {
                self.release_mouse();
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
        for p in frame["panes"].as_array()? {
            let rows = p["grid"]["rows"].as_u64()? as usize;
            let cols = p["grid"]["cols"].as_u64()? as usize;
            let px = p["rect"]["x"].as_f64().unwrap_or(0.0) * CW;
            let py = p["rect"]["y"].as_f64().unwrap_or(0.0) * CH;
            if y >= py && y < py + rows as f64 * CH && x >= px && x < px + cols as f64 * CW {
                return Some((
                    p["id"].as_str()?.into(),
                    ((y - py) / CH) as usize,
                    ((x - px) / CW) as usize,
                ));
            }
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
        self.grid().is_some_and(|g| g["mouse"] == true)
    }
    pub fn mouse(&self, button: u32, x: f64, y: f64, release: bool) {
        self.mouse_with_mods(button, x, y, release, gdk::ModifierType::empty());
    }
    pub fn mouse_with_mods(
        &self,
        button: u32,
        x: f64,
        y: f64,
        release: bool,
        mods: gdk::ModifierType,
    ) {
        if !self.mouse_enabled() {
            return;
        }
        self.mouse_position.set((x, y));
        if let Some((pane, r, c)) = self.point(x, y) {
            if pane != *self.target.borrow() {
                return;
            }
            let button = button
                + if mods.contains(gdk::ModifierType::SHIFT_MASK) {
                    4
                } else {
                    0
                }
                + if mods.contains(gdk::ModifierType::ALT_MASK) {
                    8
                } else {
                    0
                }
                + if mods.contains(gdk::ModifierType::CONTROL_MASK) {
                    16
                } else {
                    0
                };
            self.command("client.input",json!({"pane":pane,"kind":"mouse","text":"","mouse_event":{"button":button,"x":c+1,"y":r+1,"release":release}}));
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
            self.pressed.set(if release {
                None
            } else {
                Some(button.saturating_sub(1))
            });
            self.mouse_with_mods(
                match button {
                    1 => 0,
                    2 => 1,
                    3 => 2,
                    _ => return,
                },
                x,
                y,
                release,
                mods,
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
        if self.binding(key, mods, "copy", "<Control><Shift>c") {
            self.copy_clipboard(false);
            return;
        }
        if self.binding(key, mods, "paste", "<Control><Shift>v") {
            self.paste_clipboard(false);
            return;
        }
        if !self.prefix.get() && self.binding(key, mods, "prefix", "<Control>a") {
            self.prefix.set(true);
            self.status
                .set_text("Prefix command pending · ? help · a literal prefix");
            return;
        }
        if self.stack.visible_child_name().as_deref() == Some("copy") {
            self.copy_key(key, mods);
            return;
        }
        if matches!(
            key,
            gdk::Key::Shift_L
                | gdk::Key::Shift_R
                | gdk::Key::Control_L
                | gdk::Key::Control_R
                | gdk::Key::Alt_L
                | gdk::Key::Alt_R
                | gdk::Key::Super_L
                | gdk::Key::Super_R
                | gdk::Key::ISO_Level3_Shift
        ) {
            return;
        }
        let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
        let ch = key.to_unicode();
        if self.prefix.replace(false) {
            if self.binding(key, mods, "prefix", "<Control>a") {
                if self.worker.state.lock().unwrap().supports_previous_window {
                    self.navigate("window.previous", json!({"session":self.session}));
                } else {
                    self.warn("Previous-window navigation requires a newer daemon; the active session is preserved");
                }
                return;
            }
            let binding = if key == gdk::Key::Tab {
                "Tab".into()
            } else {
                ch.map(|c| c.to_string()).unwrap_or_default()
            };
            let binding = self.remaps.get(&binding).cloned().unwrap_or(binding);
            match binding.as_str() {
                "d" => self.close(),
                "c" => self.navigate("window.create", json!({"session":self.session})),
                "n" | "p" => self.navigate(
                    "window.cycle",
                    json!({"session":self.session,"previous":binding=="p"}),
                ),
                "a" => self.input("\x01"),
                "S" | "|" => self.split(binding == "|"),
                "[" => self.open_history(),
                "]" => self.command("buffer.get", json!({"session":self.session})),
                "?" => self.show_help(),
                "\"" => self.command("window.list", json!({"session":self.session})),
                ":" => self.show_prompt(),
                "z" => self.zoom(),
                "Tab" => {
                    self.release_mouse();
                    if let Some(f) = self.frame.borrow().as_ref() {
                        if let Some(p) = f["panes"].as_array() {
                            let i = p
                                .iter()
                                .position(|p| {
                                    p["id"].as_str() == Some(self.target.borrow().as_str())
                                })
                                .unwrap_or(0);
                            let next = p[(i + 1) % p.len()]["id"].as_str().unwrap().to_string();
                            *self.target.borrow_mut() = next.clone();
                            self.command("pane.select", json!({"pane":next}));
                        }
                    }
                }
                digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
                    self.window_index.set(digit.parse().ok());
                    self.command("window.list", json!({"session":self.session}));
                }
                _ => {
                    self.status
                        .set_text("Unknown prefix command forwarded to application");
                    self.input("\x01");
                    self.key(key, mods);
                }
            }
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
    fn release_mouse(&self) {
        if let Some(button) = self.pressed.take() {
            let (x, y) = self.mouse_position.get();
            self.mouse(button, x, y, true);
        }
    }
    pub fn close(&self) {
        self.release_mouse();
        if self.prefix.replace(false) {
            self.input("\x01");
        }
        *self.selection.borrow_mut() = None;
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
        let layout = area.create_pango_layout(None);
        let normal = gtk::pango::FontDescription::from_string("Monospace 11");
        let bold = gtk::pango::FontDescription::from_string("Monospace Bold 11");
        for p in panes {
            let _ = cr.save();
            cr.translate(
                p["rect"]["x"].as_f64().unwrap_or(0.0) * CW,
                p["rect"]["y"].as_f64().unwrap_or(0.0) * CH,
            );
            let y = 0.0;
            let g = &p["grid"];
            cr.set_source_rgb(0.30, 0.38, 0.48);
            cr.rectangle(
                0.0,
                0.0,
                g["cols"].as_f64().unwrap_or(0.0) * CW,
                g["rows"].as_f64().unwrap_or(0.0) * CH,
            );
            let _ = cr.stroke();
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
            let _ = cr.restore();
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
