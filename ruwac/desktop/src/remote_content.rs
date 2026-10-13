//! Keep a received selection's provider alive across copies of the same MIME.
//! Existing compositor offers can then request the current payload. Each read
//! snapshots immutable bytes so an in-flight transfer never mixes generations.
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
mod imp {
    use super::*;
    use gdk::subclass::prelude::*;
    use std::{cell::RefCell, future::Future, pin::Pin};
    #[derive(Default)]
    pub struct RemoteContent {
        pub payload: RefCell<Option<(&'static str, glib::Bytes)>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for RemoteContent {
        const NAME: &'static str = "RuwacRemoteContent";
        type Type = super::RemoteContent;
        type ParentType = gdk::ContentProvider;
    }
    impl ObjectImpl for RemoteContent {}
    impl ContentProviderImpl for RemoteContent {
        fn formats(&self) -> gdk::ContentFormats {
            let p = self.payload.borrow();
            gdk::ContentFormats::new(&p.as_ref().map(|(m, _)| vec![*m]).unwrap_or_default())
        }
        fn storable_formats(&self) -> gdk::ContentFormats {
            self.formats()
        }
        fn write_mime_type_future(
            &self,
            mime: &str,
            stream: &gio::OutputStream,
            priority: glib::Priority,
        ) -> Pin<Box<dyn Future<Output = Result<(), glib::Error>> + 'static>> {
            let bytes = self
                .payload
                .borrow()
                .as_ref()
                .filter(|(m, _)| *m == mime)
                .map(|(_, b)| b.clone());
            let stream = stream.clone();
            Box::pin(async move {
                let bytes = bytes.ok_or_else(|| {
                    glib::Error::new(
                        gio::IOErrorEnum::NotSupported,
                        "Clipboard format unavailable",
                    )
                })?;
                match stream.write_all_future(bytes, priority).await {
                    Ok((_, _, None)) => Ok(()),
                    Ok((_, _, Some(e))) | Err((_, e)) => Err(e),
                }
            })
        }
    }
}
glib::wrapper! { pub struct RemoteContent(ObjectSubclass<imp::RemoteContent>) @extends gdk::ContentProvider; }
impl RemoteContent {
    pub fn new(mime: &'static str, bytes: glib::Bytes) -> Self {
        let p: Self = glib::Object::new();
        *p.imp().payload.borrow_mut() = Some((mime, bytes));
        p
    }
    pub fn mime(&self) -> Option<&'static str> {
        self.imp().payload.borrow().as_ref().map(|(m, _)| *m)
    }
    pub fn replace_bytes(&self, bytes: glib::Bytes) {
        if let Some((_, current)) = self.imp().payload.borrow_mut().as_mut() {
            *current = bytes;
        }
        // MIME formats are unchanged. Keep the existing selection source and
        // serve the new immutable snapshot on its next read, without claiming
        // a new compositor selection from an older input serial.
    }
}
