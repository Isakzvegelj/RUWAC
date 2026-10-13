//! Exercise production GDK readers with a provider that deliberately never finishes.
#[path = "../src/clipboard.rs"]
mod clipboard;
#[path = "../src/staging.rs"]
mod staging;
use glib::subclass::prelude::*;
use gtk::{gdk, gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

mod imp {
    use super::*;
    use gdk::subclass::prelude::*;
    #[derive(Default)]
    pub struct Stalled {
        pub mime: RefCell<String>,
        pub invoked: Cell<bool>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Stalled {
        const NAME: &'static str = "RuwacGeneratedStalledProvider";
        type Type = super::Stalled;
        type ParentType = gdk::ContentProvider;
    }
    impl ObjectImpl for Stalled {}
    impl ContentProviderImpl for Stalled {
        fn formats(&self) -> gdk::ContentFormats {
            gdk::ContentFormats::new(&[self.mime.borrow().as_str()])
        }
        fn storable_formats(&self) -> gdk::ContentFormats {
            self.formats()
        }
        fn write_mime_type_future(
            &self,
            _: &str,
            stream: &gio::OutputStream,
            priority: glib::Priority,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), glib::Error>> + 'static>>
        {
            self.invoked.set(true);
            let stream = stream.clone();
            Box::pin(async move {
                let _ = stream
                    .write_all_future(b"generated incomplete bytes".to_vec(), priority)
                    .await;
                std::future::pending::<Result<(), glib::Error>>().await
            })
        }
    }
}
glib::wrapper! { pub struct Stalled(ObjectSubclass<imp::Stalled>) @extends gdk::ContentProvider; }

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let kind = args.get(1).expect("reader required").as_str();
    let focus_cancel = args.get(2).is_some_and(|v| v == "focus");
    gtk::init().expect("owned graphical fixture required");
    let display = gdk::Display::default().unwrap();
    let clip = display.clipboard();
    let provider: Stalled = glib::Object::new();
    *provider.imp().mime.borrow_mut() = if kind == "png" {
        "image/png"
    } else {
        "text/plain;charset=utf-8"
    }
    .into();
    clip.set_content(Some(&provider)).unwrap();
    let broker = Rc::new(RefCell::new(ruwac_clipboard::FocusBroker::default()));
    broker.borrow_mut().focus(Some("generated-terminal"));
    let epoch = broker.borrow().epoch();
    let stage = staging::Stage::new(std::path::Path::new(
        args.get(3).expect("staging root required"),
    ))
    .unwrap();
    let completed = Rc::new(Cell::new(false));
    let success = Rc::new(Cell::new(false));
    let loop_ = glib::MainLoop::new(None, false);
    let start = Instant::now();
    let tick_count = Rc::new(Cell::new(0usize));
    let tick = tick_count.clone();
    let timer = glib::timeout_add_local(Duration::from_millis(10), move || {
        tick.set(tick.get() + 1);
        glib::ControlFlow::Continue
    });
    if focus_cancel {
        let broker = broker.clone();
        glib::timeout_add_local_once(Duration::from_millis(200), move || {
            broker.borrow_mut().focus(None)
        });
    }
    let done = {
        let (completed, success, loop_) = (completed.clone(), success.clone(), loop_.clone());
        move |error: Option<String>| {
            completed.set(true);
            success.set(error.is_some());
            println!(
                "{}",
                serde_json::json!({"completed":true,"error":error,"elapsed_ms":start.elapsed().as_millis()})
            );
            loop_.quit();
        }
    };
    match kind {
        "text" => clipboard::read_text(
            &clip,
            broker.clone(),
            epoch,
            ruwac_clipboard::ClipScope::Clipboard,
            Duration::ZERO,
            move |r| done(r.err()),
        ),
        "png" => clipboard::ingest_png(
            &clip,
            stage.path.clone(),
            broker.clone(),
            epoch,
            ruwac_clipboard::MAX_IMAGE_BYTES,
            Duration::ZERO,
            move |r| done(r.err()),
        ),
        "rdp" => clipboard::read_bytes(&clip, broker.clone(), epoch, "text/plain", move |r| {
            done(r.err())
        }),
        _ => panic!("unknown reader"),
    }
    let watch = loop_.clone();
    let watch_fired = Rc::new(Cell::new(false));
    let fired = watch_fired.clone();
    let watchdog = glib::timeout_add_local_once(Duration::from_secs(7), move || {
        fired.set(true);
        watch.quit();
    });
    loop_.run();
    timer.remove();
    if !watch_fired.get() {
        watchdog.remove();
    }
    let partials = std::fs::read_dir(&stage.path)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|v| v.file_name().to_string_lossy().starts_with("transfer-"))
        .count();
    let elapsed = start.elapsed().as_millis();
    println!(
        "{}",
        serde_json::json!({"completed":completed.get(),"provider_invoked":provider.imp().invoked.get(),"ticks":tick_count.get(),"partial_files":partials,"elapsed_ms":elapsed,"scope":"Actual GDK on Wayland; same-process deliberately stalled provider; no physical input or external-host claim"})
    );
    let limit = if focus_cancel { 1000 } else { 6000 };
    std::process::exit(
        if completed.get()
            && success.get()
            && provider.imp().invoked.get()
            && elapsed < limit
            && partials == 0
            && tick_count.get() > 10
        {
            0
        } else {
            1
        },
    );
}
