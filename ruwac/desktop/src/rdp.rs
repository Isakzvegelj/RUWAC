//! Native IronRDP transport and GTK framebuffer, independent of the PTY daemon.
//! No competitor source is copied; IronRDP is MIT OR Apache-2.0.
use crate::cliprdr::{Backend, Bridge, Event, Format};
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
use ironrdp_cliprdr::{
    CliprdrClient,
    pdu::{ClipboardFormat, ClipboardFormatId, FormatDataResponse},
};
use ironrdp_tokio::FramedWrite;
use ruwac_clipboard::{ClipPolicy, ClipScope, OfferDecision, RemoteOffer};
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
enum Command {
    Input(Vec<FastPathInputEvent>),
    Copy(Vec<ClipboardFormat>),
    Request(ClipboardFormatId),
    Reply(Option<Vec<u8>>),
}
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
    bridge: Bridge,
    status: Option<String>,
}
struct Connection {
    tx: tokio::sync::mpsc::Sender<Command>,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
}
impl Connection {
    fn start(p: Profile) -> Self {
        Self::start_with_secret(p, None)
    }
    fn start_with_secret(p: Profile, fixture_secret: Option<String>) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(State::default()));
        let (halt, st) = (stop.clone(), state.clone());
        std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let password = if let Some(secret) = fixture_secret {
                    secret
                } else {
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
                    password
                };
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
            .try_send(Command::Input(events))
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
    mut input: tokio::sync::mpsc::Receiver<Command>,
    halt: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
) -> Result<(), String> {
    for attempt in 0..=3 {
        if halt.load(Ordering::Relaxed) {
            return Ok(());
        }
        if attempt > 0 {
            {
                let mut st = state.lock().unwrap();
                st.frame = None;
                st.bridge.reset();
                st.status = Some(format!(
                    "Reconnecting · attempt {attempt}/3 · bounded backoff"
                ));
            }
            while input.try_recv().is_ok() {}
            for _ in 0..(5 * (1 << (attempt - 1))) {
                if halt.load(Ordering::Relaxed) {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        let result = run_session(
            p.clone(),
            password.clone(),
            &mut input,
            halt.clone(),
            state.clone(),
        )
        .await;
        let retry = result.as_ref().err().is_some_and(|e| {
            matches!(
                e.as_str(),
                "RDP TCP connection failed"
                    | "RDP stream disconnected"
                    | "RDP connect timed out"
                    | "RDP response write timeout"
                    | "RDP response write failed"
            )
        });
        if !retry || attempt == 3 {
            return result;
        }
    }
    unreachable!()
}
async fn run_session(
    p: Profile,
    password: String,
    input: &mut tokio::sync::mpsc::Receiver<Command>,
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
        let bridge = state.lock().unwrap().bridge.clone();
        let mut connector = connector::ClientConnector::new(config, addr)
            .with_static_channel(CliprdrClient::new(Box::new(Backend::new(bridge))));
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
    let clip_channel = result
        .static_channels
        .get_channel_id_by_type::<CliprdrClient>();
    let mut fragments = crate::cliprdr::FragmentGuard::default();
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
        Some("Connected · embedded IronRDP · CLIPRDR text and images".into());
    let (mut reader, mut writer) = ironrdp_tokio::split_tokio_framed(framed);
    loop {
        if state
            .lock()
            .unwrap()
            .bridge
            .overflow
            .load(Ordering::Relaxed)
        {
            return Err("CLIPRDR queue exceeded safety bounds".into());
        }
        if halt.load(Ordering::Relaxed) {
            // Give queued modifier/button releases a bounded flush before closing the socket.
            let _ = tokio::time::timeout(Duration::from_millis(250), async {
                for _ in 0..128 {
                    match input.try_recv() {
                        Ok(Command::Input(events)) => {
                            if let Ok(outputs) = active.process_fastpath_input(&mut image, &events)
                            {
                                for output in outputs {
                                    if let ActiveStageOutput::ResponseFrame(bytes) = output {
                                        if writer.write_all(&bytes).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                        Ok(_) => (),
                        Err(_) => break,
                    }
                }
            })
            .await;
            return Ok(());
        }
        let outputs = tokio::select! {
         pdu=reader.read_pdu()=>{let(action,bytes)=pdu.map_err(|_|"RDP stream disconnected".to_string())?;if action==ironrdp::pdu::Action::X224{if let Ok(ctx)=ironrdp::pdu::mcs::decode_send_data_indication(&bytes){if Some(ctx.channel_id)==clip_channel{fragments.check(ctx.user_data)?;}}}active.process(&mut image,action,&bytes).map_err(|_|"RDP decode failed".to_string())?},
         command=input.recv()=>{let Some(command)=command else{return Ok(());};match command {
            Command::Input(events)=>active.process_fastpath_input(&mut image,&events).map_err(|_|"RDP input encoding failed".to_string())?,
            command=>{let clip=active.get_svc_processor_mut::<CliprdrClient>().ok_or("CLIPRDR not negotiated")?;
            let messages=match command{Command::Copy(f)=>clip.initiate_copy(&f),Command::Request(f)=>clip.initiate_paste(f),Command::Reply(data)=>clip.submit_format_data(match data{Some(b)=>FormatDataResponse::new_data(b),None=>FormatDataResponse::new_error()}),_=>unreachable!()}.map_err(|_|"CLIPRDR command failed".to_string())?;
            let bytes=active.process_svc_processor_messages(messages).map_err(|_|"CLIPRDR encoding failed".to_string())?;vec![ActiveStageOutput::ResponseFrame(bytes)]}
         }},
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
    policy: Cell<ClipPolicy>,
    serial: Cell<u64>,
    bridge: RefCell<Bridge>,
    advertised: Cell<u64>,
    received_generation: Cell<u64>,
    remote: Cell<Option<(ClipboardFormatId, Format, u64, u64)>>,
    pending: RefCell<Option<(Format, u64, u64, std::time::Instant)>>,
    replying: Cell<bool>,
    decoding: Cell<bool>,
    pending_paste: Cell<bool>,
    provider: RefCell<Option<gdk::ContentProvider>>,
    entries: [gtk::Entry; 4],
}
impl RdpView {
    pub fn new(broker: crate::clipboard::Broker) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let controls = gtk::Grid::builder()
            .column_spacing(4)
            .row_spacing(4)
            .build();
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
        for (i, e) in [&host, &user, &secret, &ca].iter().enumerate() {
            e.set_hexpand(true);
            controls.attach(*e, (i % 2) as i32, (i / 2) as i32, 1, 1);
        }
        controls.attach(&connect, 0, 2, 1, 1);
        controls.attach(&disconnect, 1, 2, 1, 1);
        root.append(&controls);
        let status = gtk::Label::new(Some(
            "Embedded IronRDP · enter an authorized endpoint and a RUWAC Secret Service reference",
        ));
        status.set_wrap(true);
        status.set_max_width_chars(80);
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
            policy: Cell::new(ClipPolicy::Bidirectional),
            serial: Cell::new(0),
            bridge: RefCell::new(Bridge::default()),
            advertised: Cell::new(0),
            received_generation: Cell::new(0),
            remote: Cell::new(None),
            pending: RefCell::new(None),
            replying: Cell::new(false),
            decoding: Cell::new(false),
            pending_paste: Cell::new(false),
            provider: RefCell::new(None),
            entries: [host.clone(), user.clone(), secret.clone(), ca.clone()],
        });
        let weak = Rc::downgrade(&v);
        connect.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                let address = host.text();
                let (h, p) = match crate::profiles::endpoint(&address, 3389) {
                    Ok(endpoint) => endpoint,
                    Err(e) => {
                        v.status.set_text(&e);
                        return;
                    }
                };
                if h.is_empty() || p == 0 || user.text().is_empty() || secret.text().is_empty() {
                    v.status
                        .set_text("Host, user and secret reference are required");
                    return;
                }
                v.close();
                let c = Connection::start(Profile {
                    host: h,
                    port: p,
                    user: user.text().into(),
                    reference: secret.text().into(),
                    ca: if ca.text().is_empty() {
                        None
                    } else {
                        Some(ca.text().into())
                    },
                });
                *v.bridge.borrow_mut() = c.state.lock().unwrap().bridge.clone();
                *v.connection.borrow_mut() = Some(c);
                v.focus();
                v.status.set_text("Connecting with certificate validation…");
            }
        });
        let weak = Rc::downgrade(&v);
        disconnect.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                v.close();
            }
        });
        let tools = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .min_children_per_line(1)
            .max_children_per_line(8)
            .column_spacing(4)
            .row_spacing(4)
            .build();
        for (name, mode) in [
            ("Fit", Scale::Fit),
            ("Native pixels", Scale::Native),
            ("Stretch", Scale::Stretch),
        ] {
            let b = gtk::Button::with_label(name);
            tools.insert(&b, -1);
            let w = Rc::downgrade(&v);
            b.connect_clicked(move |_| {
                if let Some(v) = w.upgrade() {
                    v.mode.set(mode);
                    v.area.queue_draw();
                }
            });
        }
        let full = gtk::Button::with_label("Fullscreen");
        tools.insert(&full, -1);
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
            tools.insert(&b, -1);
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
        let policy = gtk::DropDown::from_strings(&[
            "Clipboard: bidirectional",
            "Local → remote",
            "Remote → local",
            "Manual",
            "Off",
        ]);
        tools.insert(&policy, -1);
        let w = Rc::downgrade(&v);
        policy.connect_selected_notify(move |p| {
            if let Some(v) = w.upgrade() {
                v.policy.set(match p.selected() {
                    0 => ClipPolicy::Bidirectional,
                    1 => ClipPolicy::LocalToRemote,
                    2 => ClipPolicy::RemoteToLocal,
                    3 => ClipPolicy::Manual,
                    _ => ClipPolicy::Disabled,
                });
                v.broker.borrow_mut().local_copy();
                v.advertised.set(0);
                v.advertise(false);
            }
        });
        for (label, receive) in [("Send clipboard", false), ("Receive clipboard", true)] {
            let b = gtk::Button::with_label(label);
            tools.insert(&b, -1);
            let w = Rc::downgrade(&v);
            b.connect_clicked(move |_| {
                if let Some(v) = w.upgrade() {
                    if receive {
                        v.receive(true);
                    } else {
                        v.advertise(true);
                    }
                }
            });
        }
        let w = Rc::downgrade(&v);
        v.area.display().clipboard().connect_changed(move |clip| {
            if let Some(v) = w.upgrade() {
                if v.provider
                    .borrow()
                    .as_ref()
                    .is_some_and(|p| clip.content().as_ref() == Some(p))
                {
                    return;
                }
                v.advertise(false);
            }
        });
        let paste = gtk::Button::with_label("Paste clipboard into remote");
        tools.insert(&paste, -1);
        let w = Rc::downgrade(&v);
        paste.connect_clicked(move |_| {
            if let Some(v) = w.upgrade() {
                v.paste();
            }
        });
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
        key.connect_key_pressed(move |_, k, code, modifiers| {
            if let Some(v) = w.upgrade() {
                if k == gdk::Key::v && modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
                    v.paste();
                } else {
                    v.key(k, code, false);
                }
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
    pub fn fixture_connect(&self, port: u16, ca: &std::path::Path) {
        self.close();
        let c = Connection::start_with_secret(
            Profile {
                host: "127.0.0.1".into(),
                port,
                user: "fixture".into(),
                reference: String::new(),
                ca: Some(ca.to_string_lossy().into_owned()),
            },
            Some("generated-local-only".into()),
        );
        *self.bridge.borrow_mut() = c.state.lock().unwrap().bridge.clone();
        *self.connection.borrow_mut() = Some(c);
        self.focus();
    }
    pub fn fixture_inspect(&self) -> serde_json::Value {
        serde_json::json!({"status":self.status.text().as_str(),"size":self.size.get(),"viewport":[self.area.width(),self.area.height()],"window_width":self.root.root().and_then(|r|r.downcast::<gtk::Window>().ok()).map(|w|w.width()),"active":self.broker.borrow().active()==Some(self.id.as_str()),"epoch":self.broker.borrow().epoch(),"pending":self.pending.borrow().is_some(),"received_generation":self.received_generation.get(),"policy":format!("{:?}",self.policy.get())})
    }
    pub fn fixture_policy(&self, index: u32) {
        fn find(w: &gtk::Widget) -> Option<gtk::DropDown> {
            if let Some(d) = w.downcast_ref::<gtk::DropDown>() {
                return Some(d.clone());
            }
            let mut c = w.first_child();
            while let Some(child) = c {
                c = child.next_sibling();
                if let Some(d) = find(&child) {
                    return Some(d);
                }
            }
            None
        }
        if let Some(drop) = find(self.root.upcast_ref()) {
            drop.set_selected(index.min(4));
        }
    }
    pub fn fixture_clipboard(&self, receive: bool) {
        if receive {
            self.receive(true);
        } else {
            self.advertise(true);
        }
    }
    pub fn set_profile(&self, host: &str, user: &str, reference: &str, ca: &str) {
        for (e, value) in self.entries.iter().zip([host, user, reference, ca]) {
            e.set_text(value);
        }
    }
    pub fn inactive(&self) {
        self.bridge.borrow().epoch.store(0, Ordering::Release);
        self.release_keys();
    }
    fn command(&self, c: Command) {
        if let Some(conn) = self.connection.borrow().as_ref() {
            if conn.tx.try_send(c).is_err() {
                self.status
                    .set_text("CLIPRDR queue full; transfer not sent");
                conn.close();
            }
        }
    }
    fn allowed(&self, receive: bool, manual: bool) -> bool {
        self.broker.borrow().desktop_enabled()
            && self.broker.borrow().active() == Some(self.id.as_str())
            && match self.policy.get() {
                ClipPolicy::Disabled => false,
                ClipPolicy::Manual => manual,
                ClipPolicy::LocalToRemote => !receive,
                ClipPolicy::RemoteToLocal => receive,
                ClipPolicy::Bidirectional => true,
            }
    }
    fn advertise(&self, manual: bool) {
        if !self.allowed(false, manual) {
            return;
        }
        let clip = self.area.display().clipboard();
        if self
            .provider
            .borrow()
            .as_ref()
            .is_some_and(|p| clip.content().as_ref() == Some(p))
        {
            return;
        }
        let f = clip.formats();
        let formats = crate::cliprdr::formats(
            f.contain_mime_type("text/plain;charset=utf-8") || f.contain_mime_type("text/plain"),
            f.contain_mime_type("image/png"),
        );
        self.advertised.set(self.broker.borrow().epoch());
        self.command(Command::Copy(formats));
    }
    fn paste(&self) {
        if self.allowed(false, true) {
            self.release_keys();
            self.pending_paste.set(true);
            self.advertise(true);
        } else {
            self.status
                .set_text("Clipboard policy prevents local paste");
        }
    }
    fn receive(&self, manual: bool) {
        if !self.allowed(true, manual) || self.pending.borrow().is_some() {
            return;
        }
        let Some((id, kind, generation, offered_epoch)) = self.remote.get() else {
            return;
        };
        let epoch = self.broker.borrow().epoch();
        if !manual && offered_epoch != epoch {
            return;
        }
        *self.pending.borrow_mut() = Some((kind, epoch, generation, std::time::Instant::now()));
        self.command(Command::Request(id));
    }
    fn clip_event(self: &Rc<Self>, e: Event) {
        match e {
            Event::Reset => {
                self.serial.set(self.serial.get().saturating_add(1));
                let active = self.broker.borrow().active() == Some(self.id.as_str());
                self.broker.borrow_mut().disconnect(&self.id);
                self.pending.borrow_mut().take();
                self.remote.set(None);
                self.advertised.set(0);
                self.replying.set(false);
                self.decoding.set(false);
                self.held.borrow_mut().clear();
                self.buttons.borrow_mut().clear();
                self.surface.borrow_mut().take();
                self.size.set((0, 0));
                if active {
                    self.focus();
                }
                self.area.queue_draw();
            }
            Event::Initialize => self.command(Command::Copy(Vec::new())),
            Event::Ready => self.advertise(false),
            Event::Accepted(ok) => {
                if self.pending_paste.replace(false) && ok && self.allowed(false, true) {
                    self.send(vec![
                        FastPathInputEvent::KeyboardEvent(KeyboardFlags::empty(), 0x1d),
                        FastPathInputEvent::KeyboardEvent(KeyboardFlags::empty(), 0x2f),
                        FastPathInputEvent::KeyboardEvent(KeyboardFlags::RELEASE, 0x2f),
                        FastPathInputEvent::KeyboardEvent(KeyboardFlags::RELEASE, 0x1d),
                    ]);
                }
                if !ok {
                    self.status
                        .set_text("Remote clipboard rejected format list");
                }
            }
            Event::Offer {
                generation,
                epoch,
                selected,
            } => {
                self.remote
                    .set(selected.map(|(id, k)| (id, k, generation, epoch)));
                if let Some((_, kind)) = selected {
                    let decision = self.broker.borrow_mut().offer(
                        RemoteOffer {
                            view: self.id.clone(),
                            mime: kind.mime().into(),
                            generation,
                            owner_focus_epoch: epoch,
                            size: 0,
                            scope: ClipScope::Clipboard,
                        },
                        self.policy.get(),
                    );
                    if decision == OfferDecision::Eligible {
                        self.receive(false);
                    }
                }
            }
            Event::Request(id) => {
                let kind = if id == Format::Text.id() {
                    Some(Format::Text)
                } else if id == Format::Png.id() {
                    Some(Format::Png)
                } else if id == Format::Dib.id() {
                    Some(Format::Dib)
                } else if id == Format::DibV5.id() {
                    Some(Format::DibV5)
                } else {
                    None
                };
                let epoch = self.broker.borrow().epoch();
                if kind.is_none()
                    || !self.allowed(false, true)
                    || epoch != self.advertised.get()
                    || self.replying.replace(true)
                {
                    self.command(Command::Reply(None));
                    return;
                }
                let kind = kind.unwrap();
                let serial = self.serial.get();
                let w = Rc::downgrade(self);
                let clip = self.area.display().clipboard();
                let broker = self.broker.clone();
                crate::clipboard::read_bytes(&clip, broker, epoch, kind.mime(), move |result| {
                    let Some(v) = w.upgrade() else {
                        return;
                    };
                    if v.serial.get() != serial {
                        return;
                    }
                    let w = Rc::downgrade(&v);
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);
                    std::thread::spawn(move || {
                        let _ = tx.send(result.and_then(|b| crate::cliprdr::encode(kind, &b)));
                    });
                    glib::timeout_add_local(Duration::from_millis(10), move || {
                        match rx.try_recv() {
                            Ok(r) => {
                                if let Some(v) = w.upgrade() {
                                    if v.serial.get() != serial {
                                        return glib::ControlFlow::Break;
                                    }
                                    v.replying.set(false);
                                    let valid = v.allowed(false, true)
                                        && v.broker.borrow().epoch() == epoch
                                        && v.advertised.get() == epoch;
                                    v.command(Command::Reply(if valid { r.ok() } else { None }));
                                }
                                glib::ControlFlow::Break
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => {
                                glib::ControlFlow::Continue
                            }
                            Err(_) => glib::ControlFlow::Break,
                        }
                    });
                });
            }
            Event::Data(data) => {
                let Some((kind, epoch, generation, _)) = *self.pending.borrow() else {
                    return;
                };
                if self.decoding.replace(true) {
                    return;
                }
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                std::thread::spawn(move || {
                    let _ = tx.send(data.and_then(|b| crate::cliprdr::decode(kind, &b)));
                });
                let w = Rc::downgrade(self);
                glib::timeout_add_local(Duration::from_millis(10), move || match rx.try_recv() {
                    Ok(r) => {
                        if let Some(v) = w.upgrade() {
                            if v.pending
                                .borrow()
                                .as_ref()
                                .is_some_and(|p| p.1 == epoch && p.2 == generation)
                            {
                                v.pending.borrow_mut().take();
                                v.decoding.set(false);
                            }
                            if v.allowed(true, true)
                                && v.broker.borrow().epoch() == epoch
                                && v.remote.get().is_some_and(|o| o.2 == generation)
                            {
                                match r {
                                    Ok(b) => {
                                        let provider = gdk::ContentProvider::for_bytes(
                                            if kind == Format::Text {
                                                "text/plain;charset=utf-8"
                                            } else {
                                                kind.mime()
                                            },
                                            &glib::Bytes::from_owned(b),
                                        );
                                        *v.provider.borrow_mut() = Some(provider.clone());
                                        if v.area
                                            .display()
                                            .clipboard()
                                            .set_content(Some(&provider))
                                            .is_ok()
                                        {
                                            v.received_generation.set(generation);
                                            v.status.set_text(
                                                "Remote clipboard received through CLIPRDR",
                                            );
                                        }
                                    }
                                    Err(e) => v.status.set_text(&e),
                                }
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(_) => glib::ControlFlow::Break,
                });
            }
        }
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
        if self.broker.borrow().active() != Some(self.id.as_str()) {
            self.broker.borrow_mut().focus(Some(&self.id));
        }
        self.bridge
            .borrow()
            .epoch
            .store(self.broker.borrow().epoch(), Ordering::Release);
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
        self.serial.set(self.serial.get().saturating_add(1));
        self.release_keys();
        if let Some(c) = self.connection.borrow_mut().take() {
            c.close();
        }
        self.bridge.borrow().epoch.store(0, Ordering::Release);
        self.pending.borrow_mut().take();
        self.remote.set(None);
        self.advertised.set(0);
        self.replying.set(false);
        self.decoding.set(false);
        self.pending_paste.set(false);
        self.surface.borrow_mut().take();
        self.size.set((0, 0));
        self.area.queue_draw();
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
    fn update(self: &Rc<Self>) {
        let epoch = if self.broker.borrow().active() == Some(self.id.as_str()) {
            self.broker.borrow().epoch()
        } else {
            0
        };
        self.bridge.borrow().epoch.store(epoch, Ordering::Release);
        if self
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|p| p.3.elapsed() > Duration::from_secs(5))
        {
            self.close();
            self.status.set_text(
                "CLIPRDR response timed out; disconnected to prevent stale response reuse",
            );
            return;
        }
        let events = self.bridge.borrow().drain();
        for e in events {
            self.clip_event(e);
        }

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
#[cfg(test)]
#[path = "rdp_fixture.rs"]
mod authenticated_fixture;
