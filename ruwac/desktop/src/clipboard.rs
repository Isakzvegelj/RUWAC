//! GDK transport runs asynchronously; only generated fixtures are used by tests.
use gtk::{gdk, gio, glib, prelude::*};
use ruwac_clipboard::{
    ClipScope, FocusBroker, MAX_DESKTOP_TEXT, MAX_IMAGE_BYTES, TransferDirection, TransferState,
};
use std::{cell::RefCell, io::Write, path::PathBuf, rc::Rc};
pub type Broker = Rc<RefCell<FocusBroker>>;
pub fn read_text(
    clip: &gdk::Clipboard,
    broker: Broker,
    epoch: u64,
    scope: ClipScope,
    delay: std::time::Duration,
    done: impl FnOnce(Result<String, String>) + 'static,
) {
    let Some(ticket) =
        broker
            .borrow_mut()
            .begin_transfer(scope, "text/plain", TransferDirection::DesktopToView)
    else {
        done(Err("No active clipboard view".into()));
        return;
    };
    clip.read_async(
        &["text/plain;charset=utf-8", "text/plain"],
        glib::Priority::DEFAULT,
        None::<&gio::Cancellable>,
        move |result| {
            let (stream, mime) = match result {
                Ok(v) => v,
                Err(e) => {
                    broker
                        .borrow_mut()
                        .finish_transfer(&ticket, 0, TransferState::Cancelled);
                    done(Err(e.to_string()));
                    return;
                }
            };
            glib::MainContext::default().spawn_local(async move {
                let result = async {
                    if !mime.starts_with("text/plain") {
                        return Err("unsupported text MIME".into());
                    }
                    let mut bytes = Vec::new();
                    loop {
                        if broker.borrow().epoch() != epoch
                            || !broker.borrow().transfer_valid(&ticket)
                        {
                            return Err("Clipboard read cancelled: focus/owner changed".into());
                        }
                        if !delay.is_zero() {
                            glib::timeout_future(delay).await;
                        }
                        let chunk = stream
                            .read_bytes_future(65536, glib::Priority::DEFAULT)
                            .await
                            .map_err(|e| e.to_string())?;
                        if broker.borrow().epoch() != epoch
                            || !broker.borrow().transfer_valid(&ticket)
                        {
                            return Err(
                                "Clipboard/image transfer cancelled after async read".into()
                            );
                        }
                        if chunk.is_empty() {
                            break;
                        }
                        if bytes.len() + chunk.len() > MAX_DESKTOP_TEXT {
                            return Err("Desktop text exceeds 1 MiB".into());
                        }
                        bytes.extend_from_slice(chunk.as_ref());
                    }
                    let text = String::from_utf8(bytes)
                        .map_err(|_| "Clipboard text is not UTF-8".to_string())?;
                    if text.contains('\0') || text.contains("\x1b[201~") {
                        return Err("NUL or embedded paste end marker rejected".into());
                    }
                    Ok(text)
                }
                .await;
                let _ = stream.close_future(glib::Priority::DEFAULT).await;
                broker.borrow_mut().finish_transfer(
                    &ticket,
                    result.as_ref().map(|s| s.len()).unwrap_or(0),
                    if result.is_ok() {
                        TransferState::Complete
                    } else {
                        TransferState::Cancelled
                    },
                );
                done(result);
            });
        },
    );
}
pub fn ingest_png(
    clip: &gdk::Clipboard,
    dir: PathBuf,
    broker: Broker,
    epoch: u64,
    limit: usize,
    delay: std::time::Duration,
    done: impl FnOnce(Result<ruwac_clipboard::Receipt, String>) + 'static,
) {
    let limit = match crate::staging::available(&dir) {
        Ok(available) => limit.min(available),
        Err(error) => {
            done(Err(error.to_string()));
            return;
        }
    };
    let Some(ticket) = broker.borrow_mut().begin_transfer(
        ClipScope::Clipboard,
        "image/png",
        TransferDirection::DesktopToView,
    ) else {
        done(Err("No active image view".into()));
        return;
    };
    clip.read_async(
        &["image/png"],
        glib::Priority::DEFAULT,
        None::<&gio::Cancellable>,
        move |result| {
            let (stream, mime) = match result {
                Ok(v) => v,
                Err(e) => {
                    broker
                        .borrow_mut()
                        .finish_transfer(&ticket, 0, TransferState::Cancelled);
                    done(Err(e.to_string()));
                    return;
                }
            };
            glib::MainContext::default().spawn_local(async move {
                let temp = dir.join(format!("transfer-{}.png", ruwac_protocol::id("image")));
                let result = async {
                    if mime != "image/png" {
                        return Err("Unsupported image MIME".into());
                    }
                    use std::os::unix::fs::OpenOptionsExt;
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .custom_flags(libc::O_NOFOLLOW)
                        .open(&temp)
                        .map_err(|e| e.to_string())?;
                    let mut count = 0;
                    loop {
                        if broker.borrow().epoch() != epoch
                            || !broker.borrow().transfer_valid(&ticket)
                        {
                            return Err("Image transfer cancelled on focus/owner change".into());
                        }
                        if !delay.is_zero() {
                            glib::timeout_future(delay).await;
                        }
                        let chunk = stream
                            .read_bytes_future(65536, glib::Priority::DEFAULT)
                            .await
                            .map_err(|e| e.to_string())?;
                        if broker.borrow().epoch() != epoch
                            || !broker.borrow().transfer_valid(&ticket)
                        {
                            return Err(
                                "Clipboard/image transfer cancelled after async read".into()
                            );
                        }
                        if chunk.is_empty() {
                            break;
                        }
                        count += chunk.len();
                        if count > limit.min(MAX_IMAGE_BYTES) {
                            return Err("Image transfer exceeds configured cap".into());
                        }
                        // Only a bounded 64 KiB disk write occurs per async iteration; PNG decoding is off-thread.
                        file.write_all(chunk.as_ref()).map_err(|e| e.to_string())?;
                    }
                    drop(file);
                    Ok(())
                }
                .await;
                let _ = stream.close_future(glib::Priority::DEFAULT).await;
                if let Err(e) = result {
                    let _ = std::fs::remove_file(&temp);
                    broker
                        .borrow_mut()
                        .finish_transfer(&ticket, 0, TransferState::Cancelled);
                    done(Err(e));
                    return;
                }
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                let worker_temp = temp.clone();
                std::thread::spawn(move || {
                    let receipt = ruwac_clipboard::stage_png_with_limit(&worker_temp, &dir, limit);
                    let _ = std::fs::remove_file(worker_temp);
                    let _ = tx.send(receipt);
                });
                loop {
                    match rx.try_recv() {
                        Ok(result) => {
                            if broker.borrow().epoch() != epoch
                                || !broker.borrow().transfer_valid(&ticket)
                            {
                                if let Ok(r) = result {
                                    let _ = std::fs::remove_file(r.path);
                                }
                                done(Err("Image completion cancelled".into()));
                            } else {
                                broker.borrow_mut().finish_transfer(
                                    &ticket,
                                    result.as_ref().map(|r| r.bytes).unwrap_or(0),
                                    if result.is_ok() {
                                        TransferState::Complete
                                    } else {
                                        TransferState::Cancelled
                                    },
                                );
                                done(result);
                            }
                            break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            glib::timeout_future(std::time::Duration::from_millis(10)).await
                        }
                        Err(_) => {
                            done(Err("Image worker stopped".into()));
                            break;
                        }
                    }
                }
            });
        },
    );
}

/// CLIPRDR delayed rendering: bounded async GDK bytes, no decoding on GTK thread.
pub fn read_bytes(
    clip: &gdk::Clipboard,
    broker: Broker,
    epoch: u64,
    mime: &'static str,
    done: impl FnOnce(Result<Vec<u8>, String>) + 'static,
) {
    let limit = if mime == "text/plain" {
        MAX_DESKTOP_TEXT
    } else {
        MAX_IMAGE_BYTES
    };
    let types = if mime == "text/plain" {
        vec!["text/plain;charset=utf-8", "text/plain"]
    } else {
        vec!["image/png"]
    };
    let view = broker.borrow().active().map(str::to_owned);
    let started = std::time::Instant::now();
    let cancel = gio::Cancellable::new();
    let timeout_cancel = cancel.clone();
    glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || {
        timeout_cancel.cancel()
    });
    clip.read_async(
        &types,
        glib::Priority::DEFAULT,
        Some(&cancel),
        move |result| {
            glib::MainContext::default().spawn_local(async move {
                let read = async {
                    let (stream, _) =
                        result.map_err(|_| "Desktop clipboard read failed".to_string())?;
                    let result = async {
                        let mut bytes = Vec::new();
                        loop {
                            if broker.borrow().epoch() != epoch
                                || broker.borrow().active() != view.as_deref()
                            {
                                return Err("Clipboard owner/focus changed".into());
                            }
                            let chunk = stream
                                .read_bytes_future(65536, glib::Priority::DEFAULT)
                                .await
                                .map_err(|_| "Clipboard stream failed".to_string())?;
                            if broker.borrow().epoch() != epoch
                                || broker.borrow().active() != view.as_deref()
                            {
                                return Err("Clipboard owner/focus changed".into());
                            }
                            if bytes.len() + chunk.len() > limit {
                                return Err("Clipboard transfer exceeds cap".into());
                            }
                            if chunk.is_empty() {
                                return Ok(bytes);
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                    }
                    .await;
                    let _ = stream.close_future(glib::Priority::DEFAULT).await;
                    result
                };
                let result = match futures_util::future::select(
                    Box::pin(read),
                    Box::pin(glib::timeout_future(
                        std::time::Duration::from_secs(5).saturating_sub(started.elapsed()),
                    )),
                )
                .await
                {
                    futures_util::future::Either::Left((r, _)) => r,
                    futures_util::future::Either::Right(_) => {
                        Err("Clipboard read deadline exceeded".into())
                    }
                };
                done(result);
            });
        },
    );
}
