//! Native IronRDP transport and GTK framebuffer, independent of the PTY daemon.
//! No competitor source is copied; IronRDP is MIT OR Apache-2.0.
use adw::prelude::*;
use gtk::{gdk, glib};
use ironrdp::{
    connector,
    pdu::{
        gcc::KeyboardType,
        input::{
            MousePdu,
            fast_path::{FastPathInputEvent, KeyboardFlags},
            mouse::PointerFlags,
        },
        rdp::{
            capability_sets::MajorPlatformType,
            client_info::{PerformanceFlags, TimezoneInfo},
        },
    },
    session::{ActiveStageBuilder, ActiveStageOutput, image::DecodedImage},
};
use ironrdp_tokio::FramedWrite;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use x509_cert::der::Decode;
#[derive(Clone)]
struct Profile {
    host: String,
    port: u16,
    user: String,
    reference: String,
    ca: Option<String>,
}
struct Frame {
    width: u16,
    height: u16,
    pixels: Vec<u8>,
}
#[derive(Default)]
struct State {
    frame: Option<Frame>,
    status: Option<String>,
}
struct Connection {
    tx: tokio::sync::mpsc::Sender<Vec<FastPathInputEvent>>,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
}
impl Connection {
    fn start(p: Profile) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(State::default()));
        let (halt, st) = (stop.clone(), state.clone());
        std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let output = std::process::Command::new("timeout")
                    .args([
                        "5",
                        "secret-tool",
                        "lookup",
                        "application",
                        "RUWAC",
                        "profile",
                        &p.reference,
                    ])
                    .output()
                    .map_err(|_| "Secret Service helper unavailable".to_string())?;
                if !output.status.success()
                    || output.stdout.is_empty()
                    || output.stdout.len() > 4096
                {
                    return Err("Secret reference missing, locked, or oversized".into());
                }
                let password = String::from_utf8(output.stdout)
                    .map_err(|_| "Secret is not UTF-8".to_string())?
                    .trim_end_matches('\n')
                    .to_string();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| "RDP runtime unavailable".to_string())?;
                rt.block_on(run(p, password, rx, halt, st.clone()))
            })();
            st.lock().unwrap().status =
                Some(result.err().unwrap_or_else(|| "RDP disconnected".into()));
        });
        Self { tx, stop, state }
    }
    fn send(&self, events: Vec<FastPathInputEvent>) -> Result<(), String> {
        self.tx
            .try_send(events)
            .map_err(|_| "RDP input queue full or disconnected; input not sent".into())
    }
    fn close(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
    }
}
async fn run(
    p: Profile,
    password: String,
    mut input: tokio::sync::mpsc::Receiver<Vec<FastPathInputEvent>>,
    halt: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
) -> Result<(), String> {
    let session = async {
        let tcp = tokio::net::TcpStream::connect((p.host.as_str(), p.port))
            .await
            .map_err(|_| "RDP TCP connection failed".to_string())?;
        let addr = tcp
            .local_addr()
            .map_err(|_| "RDP socket address unavailable".to_string())?;
        let config = connector::Config {
            credentials: connector::Credentials::UsernamePassword {
                username: p.user,
                password,
            },
            domain: None,
            enable_tls: false,
            enable_credssp: true,
            keyboard_type: KeyboardType::IbmEnhanced,
            keyboard_subtype: 0,
            keyboard_layout: 0x0409,
            keyboard_functional_keys_count: 12,
            ime_file_name: String::new(),
            dig_product_id: String::new(),
            desktop_size: connector::DesktopSize {
                width: 1280,
                height: 720,
            },
            bitmap: None,
            client_build: 0,
            client_name: "RUWAC".into(),
            client_dir: String::new(),
            platform: MajorPlatformType::UNIX,
            enable_server_pointer: true,
            request_data: None,
            autologon: false,
            enable_audio_playback: false,
            compression_type: None,
            pointer_software_rendering: true,
            multitransport_flags: None,
            performance_flags: PerformanceFlags::default(),
            desktop_scale_factor: 0,
            hardware_id: None,
            license_cache: None,
            timezone_info: TimezoneInfo::default(),
            alternate_shell: String::new(),
            work_dir: String::new(),
        };
        let mut connector = connector::ClientConnector::new(config, addr);
        let mut framed = ironrdp_tokio::TokioFramed::new(tcp);
        let upgrade = ironrdp_tokio::connect_begin(&mut framed, &mut connector)
            .await
            .map_err(|_| "RDP security negotiation failed".to_string())?;
        let (stream, leftover) = framed.into_inner();
        if !leftover.is_empty() {
            return Err("Unexpected pre-TLS payload".into());
        }
        let mut tls = native_tls::TlsConnector::builder();
        if let Some(path) = p.ca.as_ref() {
            let pem = std::fs::read(path).map_err(|_| "CA certificate unavailable".to_string())?;
            if pem.len() > 1024 * 1024 {
                return Err("CA certificate oversized".into());
            }
            tls.add_root_certificate(
                native_tls::Certificate::from_pem(&pem)
                    .map_err(|_| "CA certificate invalid".to_string())?,
            );
        }
        // System roots and hostname validation stay enabled. No insecure TLS switch.
        let tls = tokio_native_tls::TlsConnector::from(
            tls.build()
                .map_err(|_| "TLS initialization failed".to_string())?,
        );
        let tls = tls
            .connect(&p.host, stream)
            .await
            .map_err(|_| "TLS certificate/hostname validation failed".to_string())?;
        let der = tls
            .get_ref()
            .peer_certificate()
            .map_err(|_| "Peer certificate unavailable".to_string())?
            .ok_or("Peer certificate missing")?
            .to_der()
            .map_err(|_| "Peer certificate invalid".to_string())?;
        let cert = x509_cert::Certificate::from_der(&der)
            .map_err(|_| "Peer certificate decoding failed".to_string())?;
        let key = cert
            .tbs_certificate
            .subject_public_key_info
            .subject_public_key
            .as_bytes()
            .ok_or("Peer key unavailable")?
            .to_vec();
        let upgraded = ironrdp_tokio::mark_as_upgraded(upgrade, &mut connector);
        let mut framed = ironrdp_tokio::TokioFramed::new(tls);
        let mut network = ironrdp_tokio::reqwest::ReqwestNetworkClient::new();
        let result = ironrdp_tokio::connect_finalize(
            upgraded,
            connector,
            &mut framed,
            &mut network,
            p.host.clone().into(),
            key,
            None,
        )
        .await
        .map_err(|_| "RDP authentication/session setup failed".to_string())?;
        if result.desktop_size.width == 0
            || result.desktop_size.height == 0
            || result.desktop_size.width > 4096
            || result.desktop_size.height > 4096
        {
            return Err("Remote desktop exceeds framebuffer limits".into());
        }
        Ok::<_, String>((result, framed))
    };
    let (result, framed) = tokio::select! {r=tokio::time::timeout(Duration::from_secs(20),session)=>r.map_err(|_|"RDP connect timed out".to_string())??,_=async{while !halt.load(Ordering::Relaxed){tokio::time::sleep(Duration::from_millis(50)).await;}}=>return Ok(())};
    let mut image = DecodedImage::new(
        ironrdp::graphics::image_processing::PixelFormat::BgrX32,
        result.desktop_size.width,
        result.desktop_size.height,
    );
    let mut active = ActiveStageBuilder {
        static_channels: result.static_channels,
        user_channel_id: result.user_channel_id,
        io_channel_id: result.io_channel_id,
        message_channel_id: result.message_channel_id,
        share_id: result.share_id,
        compression_type: result.compression_type,
        enable_server_pointer: result.enable_server_pointer,
        pointer_software_rendering: result.pointer_software_rendering,
    }
    .build();
    state.lock().unwrap().status =
        Some("Connected · embedded IronRDP · clipboard redirection disabled".into());
    let (mut reader, mut writer) = ironrdp_tokio::split_tokio_framed(framed);
    loop {
        if halt.load(Ordering::Relaxed) {
            return Ok(());
        }
        let outputs = tokio::select! {
         pdu=reader.read_pdu()=>{let(action,bytes)=pdu.map_err(|_|"RDP stream disconnected".to_string())?;active.process(&mut image,action,&bytes).map_err(|_|"RDP decode failed".to_string())?},
         events=input.recv()=>{let Some(events)=events else{return Ok(());};active.process_fastpath_input(&mut image,&events).map_err(|_|"RDP input encoding failed".to_string())?},
         _=tokio::time::sleep(Duration::from_millis(50))=>continue,
        };
        for output in outputs {
            match output {
                ActiveStageOutput::ResponseFrame(bytes) => {
                    tokio::time::timeout(Duration::from_secs(2), writer.write_all(&bytes))
                        .await
                        .map_err(|_| "RDP response write timeout".to_string())?
                        .map_err(|_| "RDP response write failed".to_string())?
                }
                ActiveStageOutput::GraphicsUpdate(_) => {
                    state.lock().unwrap().frame = Some(Frame {
                        width: image.width(),
                        height: image.height(),
                        pixels: image.data().to_vec(),
                    })
                }
                ActiveStageOutput::Terminate(_) => return Ok(()),
                ActiveStageOutput::DeactivateAll => {
                    return Err(
                        "Server reactivation requires reconnect; dynamic resize not yet supported"
                            .into(),
                    );
                }
                _ => (),
            }
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum Scale {
    Fit,
    Native,
    Stretch,
}
pub fn transform(
    widget: (f64, f64),
    remote: (u16, u16),
    mode: Scale,
    x: f64,
    y: f64,
) -> Option<(u16, u16)> {
    let (w, h) = widget;
    let (rw, rh) = (remote.0 as f64, remote.1 as f64);
    if w <= 0.0 || h <= 0.0 || rw == 0.0 || rh == 0.0 {
        return None;
    }
    let (sx, sy) = match mode {
        Scale::Fit => {
            let s = (w / rw).min(h / rh);
            (s, s)
        }
        Scale::Native => (1.0, 1.0),
        Scale::Stretch => (w / rw, h / rh),
    };
    let (ox, oy) = ((w - rw * sx) / 2.0, (h - rh * sy) / 2.0);
    let (px, py) = ((x - ox) / sx, (y - oy) / sy);
    if px < 0.0 || py < 0.0 || px >= rw || py >= rh {
        return None;
    }
    Some((px as u16, py as u16))
}
pub struct RdpView {
    pub root: gtk::Box,
    area: gtk::DrawingArea,
    status: gtk::Label,
    connection: RefCell<Option<Connection>>,
    surface: RefCell<Option<gtk::cairo::ImageSurface>>,
    size: Cell<(u16, u16)>,
    mode: Cell<Scale>,
    held: RefCell<HashSet<(u8, bool)>>,
    buttons: RefCell<HashSet<u32>>,
    pointer_position: Cell<(u16, u16)>,
    broker: crate::clipboard::Broker,
    id: String,
}
impl RdpView {
    pub fn new(broker: crate::clipboard::Broker) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let host = gtk::Entry::builder()
            .placeholder_text("Authorized host:port")
            .build();
        let user = gtk::Entry::builder().placeholder_text("Username").build();
        let secret = gtk::Entry::builder()
            .placeholder_text("Secret Service profile reference")
            .build();
        let ca = gtk::Entry::builder()
            .placeholder_text("Optional CA PEM path")
            .build();
        let connect = gtk::Button::with_label("Connect / reconnect");
        let disconnect = gtk::Button::with_label("Disconnect");
        for e in [&host, &user, &secret, &ca] {
            controls.append(e);
        }
        controls.append(&connect);
        controls.append(&disconnect);
        root.append(&controls);
        let status = gtk::Label::new(Some(
            "Embedded IronRDP · enter an authorized endpoint and a RUWAC Secret Service reference",
        ));
        root.append(&status);
        let area = gtk::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .focusable(true)
            .build();
        root.append(&area);
        let v = Rc::new(Self {
            root: root.clone(),
            area,
            status,
            connection: RefCell::new(None),
            surface: RefCell::new(None),
            size: Cell::new((0, 0)),
            mode: Cell::new(Scale::Fit),
            held: RefCell::new(HashSet::new()),
            buttons: RefCell::new(HashSet::new()),
            pointer_position: Cell::new((0, 0)),
            broker,
            id: ruwac_protocol::id("rdp"),
        });
        let weak = Rc::downgrade(&v);
        connect.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                let address = host.text();
                let (h, p) = address
                    .rsplit_once(':')
                    .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(0)))
                    .unwrap_or((address.to_string(), 3389));
                if h.is_empty() || p == 0 || user.text().is_empty() || secret.text().is_empty() {
                    v.status
                        .set_text("Host, user and secret reference are required");
                    return;
                }
                v.close();
                *v.connection.borrow_mut() = Some(Connection::start(Profile {
                    host: h,
                    port: p,
                    user: user.text().into(),
                    reference: secret.text().into(),
                    ca: if ca.text().is_empty() {
                        None
                    } else {
                        Some(ca.text().into())
                    },
                }));
                v.status.set_text("Connecting with certificate validation…");
            }
        });
        let weak = Rc::downgrade(&v);
        disconnect.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                v.close();
            }
        });
        let tools = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        for (name, mode) in [
            ("Fit", Scale::Fit),
            ("Native pixels", Scale::Native),
            ("Stretch", Scale::Stretch),
        ] {
            let b = gtk::Button::with_label(name);
            tools.append(&b);
            let w = Rc::downgrade(&v);
            b.connect_clicked(move |_| {
                if let Some(v) = w.upgrade() {
                    v.mode.set(mode);
                    v.area.queue_draw();
                }
            });
        }
        let full = gtk::Button::with_label("Fullscreen");
        tools.append(&full);
        let w = Rc::downgrade(&v);
        full.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                if let Some(win) = v.root.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                    if win.is_fullscreen() {
                        win.unfullscreen();
                    } else {
                        win.fullscreen();
                    }
                }
            }
        });
        for (name, keys) in [
            (
                "Ctrl+Alt+Del",
                vec![(0x1d, false), (0x38, false), (0x53, true)],
            ),
            ("PrintScreen", vec![(0x37, true)]),
            ("Alt+Tab", vec![(0x38, false), (0x0f, false)]),
            ("Super", vec![(0x5b, true)]),
            ("AltGr", vec![(0x38, true)]),
        ] {
            let b = gtk::Button::with_label(name);
            tools.append(&b);
            let w = Rc::downgrade(&v);
            b.connect_clicked(move |_| {
                if let Some(v) = w.upgrade() {
                    let mut events = Vec::new();
                    for (k, e) in &keys {
                        events.push(FastPathInputEvent::KeyboardEvent(
                            if *e {
                                KeyboardFlags::EXTENDED
                            } else {
                                KeyboardFlags::empty()
                            },
                            *k,
                        ));
                    }
                    for (k, e) in keys.iter().rev() {
                        events.push(FastPathInputEvent::KeyboardEvent(
                            KeyboardFlags::RELEASE
                                | if *e {
                                    KeyboardFlags::EXTENDED
                                } else {
                                    KeyboardFlags::empty()
                                },
                            *k,
                        ));
                    }
                    v.send(events);
                }
            });
        }
        root.prepend(&tools);
        let w = Rc::downgrade(&v);
        v.area.set_draw_func(move |_, cr, width, height| {
            if let Some(v) = w.upgrade() {
                cr.set_source_rgb(0.03, 0.03, 0.04);
                let _ = cr.paint();
                if let Some(surface) = v.surface.borrow().as_ref() {
                    let (rw, rh) = v.size.get();
                    let (sx, sy) = match v.mode.get() {
                        Scale::Fit => {
                            let s = (width as f64 / rw as f64).min(height as f64 / rh as f64);
                            (s, s)
                        }
                        Scale::Native => (1.0, 1.0),
                        Scale::Stretch => (width as f64 / rw as f64, height as f64 / rh as f64),
                    };
                    cr.translate(
                        (width as f64 - rw as f64 * sx) / 2.0,
                        (height as f64 - rh as f64 * sy) / 2.0,
                    );
                    cr.scale(sx, sy);
                    let _ = cr.set_source_surface(surface, 0.0, 0.0);
                    let _ = cr.paint();
                }
            }
        });
        let key = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&v);
        key.connect_key_pressed(move |_, k, code, _| {
            if let Some(v) = w.upgrade() {
                v.key(k, code, false);
            }
            glib::Propagation::Stop
        });
        let w = Rc::downgrade(&v);
        key.connect_key_released(move |_, k, code, _| {
            if let Some(v) = w.upgrade() {
                v.key(k, code, true);
            }
        });
        v.area.add_controller(key);
        let focus = gtk::EventControllerFocus::new();
        let w = Rc::downgrade(&v);
        focus.connect_leave(move |_| {
            if let Some(v) = w.upgrade() {
                v.release_keys();
            }
        });
        v.area.add_controller(focus);
        let click = gtk::GestureClick::new();
        click.set_button(0);
        let w = Rc::downgrade(&v);
        click.connect_pressed(move |g, _, x, y| {
            if let Some(v) = w.upgrade() {
                v.focus();
                v.pointer(x, y, g.current_button(), false);
            }
        });
        let w = Rc::downgrade(&v);
        click.connect_released(move |g, _, x, y| {
            if let Some(v) = w.upgrade() {
                v.pointer(x, y, g.current_button(), true);
            }
        });
        v.area.add_controller(click);
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&v);
        motion.connect_motion(move |_, x, y| {
            if let Some(v) = w.upgrade() {
                v.pointer(x, y, 0, false);
            }
        });
        v.area.add_controller(motion);
        let w = Rc::downgrade(&v);
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let Some(v) = w.upgrade() else {
                return glib::ControlFlow::Break;
            };
            v.update();
            glib::ControlFlow::Continue
        });
        v
    }
    fn send(&self, e: Vec<FastPathInputEvent>) {
        if let Some(c) = self.connection.borrow().as_ref() {
            if let Err(e) = c.send(e) {
                self.status.set_text(&e);
            }
        }
    }
    fn key(&self, key: gdk::Key, code: u32, release: bool) {
        // GDK/XKB hardware codes are evdev+8, mostly set-1 for base keys.
        let special = match key {
            gdk::Key::Up => Some((0x48, true)),
            gdk::Key::Down => Some((0x50, true)),
            gdk::Key::Left => Some((0x4b, true)),
            gdk::Key::Right => Some((0x4d, true)),
            gdk::Key::Home => Some((0x47, true)),
            gdk::Key::End => Some((0x4f, true)),
            gdk::Key::Page_Up => Some((0x49, true)),
            gdk::Key::Page_Down => Some((0x51, true)),
            gdk::Key::Delete => Some((0x53, true)),
            gdk::Key::Insert => Some((0x52, true)),
            gdk::Key::Control_R => Some((0x1d, true)),
            gdk::Key::Alt_R | gdk::Key::ISO_Level3_Shift => Some((0x38, true)),
            gdk::Key::Super_L => Some((0x5b, true)),
            gdk::Key::Super_R => Some((0x5c, true)),
            gdk::Key::Print => Some((0x37, true)),
            _ => None,
        };
        if let Some((scan, extended)) = special.or_else(|| {
            if (9..=96).contains(&code) {
                Some(((code - 8) as u8, false))
            } else {
                None
            }
        }) {
            if release {
                self.held.borrow_mut().remove(&(scan, extended));
            } else {
                self.held.borrow_mut().insert((scan, extended));
            }
            self.send(vec![FastPathInputEvent::KeyboardEvent(
                (if release {
                    KeyboardFlags::RELEASE
                } else {
                    KeyboardFlags::empty()
                }) | if extended {
                    KeyboardFlags::EXTENDED
                } else {
                    KeyboardFlags::empty()
                },
                scan,
            )]);
        } else if let Some(ch) = key.to_unicode() {
            for unit in ch.to_string().encode_utf16() {
                self.send(vec![FastPathInputEvent::UnicodeKeyboardEvent(
                    if release {
                        KeyboardFlags::RELEASE
                    } else {
                        KeyboardFlags::empty()
                    },
                    unit,
                )]);
            }
        }
    }

    fn pointer(&self, x: f64, y: f64, button: u32, release: bool) {
        let Some((x, y)) = transform(
            (self.area.width() as f64, self.area.height() as f64),
            self.size.get(),
            self.mode.get(),
            x,
            y,
        ) else {
            return;
        };
        self.pointer_position.set((x, y));
        if button != 0 {
            if release {
                self.buttons.borrow_mut().remove(&button);
            } else {
                self.buttons.borrow_mut().insert(button);
            }
        }
        let mut flags = match button {
            0 => PointerFlags::MOVE,
            1 => PointerFlags::LEFT_BUTTON,
            2 => PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
            3 => PointerFlags::RIGHT_BUTTON,
            _ => return,
        };
        if button != 0 && !release {
            flags |= PointerFlags::DOWN;
        }
        self.send(vec![FastPathInputEvent::MouseEvent(MousePdu {
            flags,
            number_of_wheel_rotation_units: 0,
            x_position: x,
            y_position: y,
        })]);
    }
    pub fn focus(&self) {
        self.broker.borrow_mut().focus(Some(&self.id));
        self.area.grab_focus();
    }
    fn release_keys(&self) {
        let (x, y) = self.pointer_position.get();
        let events: Vec<_> = self
            .buttons
            .borrow_mut()
            .drain()
            .filter_map(|b| {
                let flags = match b {
                    1 => PointerFlags::LEFT_BUTTON,
                    2 => PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
                    3 => PointerFlags::RIGHT_BUTTON,
                    _ => return None,
                };
                Some(FastPathInputEvent::MouseEvent(MousePdu {
                    flags,
                    number_of_wheel_rotation_units: 0,
                    x_position: x,
                    y_position: y,
                }))
            })
            .collect();
        if !events.is_empty() {
            self.send(events);
        }
        let keys: Vec<_> = self
            .held
            .borrow_mut()
            .drain()
            .map(|(k, e)| {
                FastPathInputEvent::KeyboardEvent(
                    KeyboardFlags::RELEASE
                        | if e {
                            KeyboardFlags::EXTENDED
                        } else {
                            KeyboardFlags::empty()
                        },
                    k,
                )
            })
            .collect();
        if !keys.is_empty() {
            self.send(keys);
        }
    }
    pub fn close(&self) {
        self.release_keys();
        if let Some(c) = self.connection.borrow_mut().take() {
            c.close();
        }
        self.broker.borrow_mut().disconnect(&self.id);
        self.status.set_text("RDP disconnected");
    }
    pub fn fixture_frame(&self) {
        let mut pixels = vec![0; 320 * 180 * 4];
        for (y, row) in pixels.chunks_mut(320 * 4).enumerate() {
            for (x, p) in row.chunks_mut(4).enumerate() {
                p.copy_from_slice(&[(x % 255) as u8, (y % 255) as u8, 80, 255]);
            }
        }
        self.size.set((320, 180));
        *self.surface.borrow_mut() = gtk::cairo::ImageSurface::create_for_data(
            pixels,
            gtk::cairo::Format::Rgb24,
            320,
            180,
            320 * 4,
        )
        .ok();
        self.status
            .set_text("Synthetic framebuffer fixture · no remote connection");
        self.area.queue_draw();
    }
    fn update(&self) {
        let frame = {
            let c = self.connection.borrow();
            let Some(c) = c.as_ref() else {
                return;
            };
            let mut s = c.state.lock().unwrap();
            if let Some(status) = s.status.take() {
                self.status.set_text(&status);
            }
            s.frame.take()
        };
        if let Some(f) = frame {
            self.size.set((f.width, f.height));
            match gtk::cairo::ImageSurface::create_for_data(
                f.pixels,
                gtk::cairo::Format::Rgb24,
                f.width as i32,
                f.height as i32,
                f.width as i32 * 4,
            ) {
                Ok(surface) => {
                    *self.surface.borrow_mut() = Some(surface);
                    self.area.queue_draw();
                }
                Err(_) => self.status.set_text("Framebuffer format rejected"),
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_loopback_tls_validation() {
        use std::{
            fs,
            io::{Read, Write},
            os::unix::fs::{DirBuilderExt, PermissionsExt},
        };
        let base = std::env::temp_dir().join(ruwac_protocol::id("rdp_tls"));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let cert = base.join("cert.pem");
        let key = base.join("key.pem");
        let status = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=RUWAC-local-fixture",
                "-addext",
                "subjectAltName=IP:127.0.0.1",
                "-keyout",
            ])
            .arg(&key)
            .arg("-out")
            .arg(&cert)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
        let identity =
            native_tls::Identity::from_pkcs8(&fs::read(&cert).unwrap(), &fs::read(&key).unwrap())
                .unwrap();
        for trusted in [false, true] {
            let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let port = listener.local_addr().unwrap().port();
            let identity = identity.clone();
            let server = std::thread::spawn(move || {
                let (mut tcp, _) = listener.accept().unwrap();
                tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                tcp.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut header = [0; 4];
                tcp.read_exact(&mut header).unwrap();
                let len = u16::from_be_bytes([header[2], header[3]]) as usize;
                assert!((4..4096).contains(&len));
                let mut data = vec![0; len - 4];
                tcp.read_exact(&mut data).unwrap();
                tcp.write_all(&[3, 0, 0, 19, 14, 0xd0, 0, 0, 0, 0, 0, 2, 0, 8, 0, 2, 0, 0, 0])
                    .unwrap();
                let acceptor = native_tls::TlsAcceptor::new(identity).unwrap();
                let _ = acceptor.accept(tcp);
            });
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            let result = rt.block_on(run(
                Profile {
                    host: "127.0.0.1".into(),
                    port,
                    user: "synthetic-fixture".into(),
                    reference: String::new(),
                    ca: if trusted {
                        Some(cert.to_string_lossy().into_owned())
                    } else {
                        None
                    },
                },
                "synthetic-fixture-password".into(),
                rx,
                Arc::new(AtomicBool::new(false)),
                Arc::new(Mutex::new(State::default())),
            ));
            assert_eq!(
                result.unwrap_err(),
                if trusted {
                    "RDP authentication/session setup failed"
                } else {
                    "TLS certificate/hostname validation failed"
                }
            );
            server.join().unwrap();
        }
        fs::remove_file(cert).unwrap();
        fs::remove_file(key).unwrap();
        fs::remove_dir(base).unwrap();
    }
    #[test]
    fn transforms() {
        assert_eq!(
            transform((800.0, 600.0), (1280, 720), Scale::Fit, 400.0, 300.0),
            Some((640, 360))
        );
        assert_eq!(
            transform((800.0, 600.0), (1280, 720), Scale::Fit, 400.0, 10.0),
            None
        );
        assert_eq!(
            transform((800.0, 600.0), (1280, 720), Scale::Stretch, 400.0, 300.0),
            Some((640, 360))
        );
        assert_eq!(
            transform((1280.0, 720.0), (1280, 720), Scale::Native, 100.0, 200.0),
            Some((100, 200))
        );
    }
}
