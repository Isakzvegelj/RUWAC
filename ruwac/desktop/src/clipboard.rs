//! GDK transport runs asynchronously; only generated fixtures are used by tests.
use gtk::{gdk, gio, glib, prelude::*};
use ruwac_clipboard::{
    ClipScope, FocusBroker, MAX_DESKTOP_TEXT, MAX_IMAGE_BYTES, TransferDirection, TransferState,
};
use std::{
    cell::{Cell, RefCell},
    io::Write,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
pub type Broker = Rc<RefCell<FocusBroker>>;
const READ_DEADLINE: Duration = Duration::from_secs(5);
const CANCEL_POLL: Duration = Duration::from_millis(25);

/// Cover both selection negotiation and a provider stalled halfway through its stream.
/// Dropping a selected-out GIO future cancels that operation's own Cancellable.
struct ReadGuard {
    cancel: gio::Cancellable,
    timer: Option<glib::SourceId>,
    timer_live: Rc<Cell<bool>>,
    valid: Rc<dyn Fn() -> bool>,
    deadline: Instant,
}
impl ReadGuard {
    fn new(valid: impl Fn() -> bool + 'static) -> Self {
        let cancel = gio::Cancellable::new();
        let timer_live = Rc::new(Cell::new(true));
        let valid: Rc<dyn Fn() -> bool> = Rc::new(valid);
        let deadline = Instant::now() + READ_DEADLINE;
        let (c, live, check) = (cancel.clone(), timer_live.clone(), valid.clone());
        let timer = glib::timeout_add_local(CANCEL_POLL, move || {
            if Instant::now() >= deadline || !check() {
                live.set(false);
                c.cancel();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        Self {
            cancel,
            timer: Some(timer),
            timer_live,
            valid,
            deadline,
        }
    }
    fn check(&self) -> Result<(), String> {
        if !(self.valid)() {
            return Err("Clipboard read cancelled: focus/owner changed".into());
        }
        if Instant::now() >= self.deadline {
            return Err("Clipboard read deadline exceeded".into());
        }
        Ok(())
    }
    async fn wait<T>(&self, future: impl std::future::Future<Output = T>) -> Result<T, String> {
        self.check()?;
        let cancellation = async {
            loop {
                if let Err(error) = self.check() {
                    return error;
                }
                glib::timeout_future(
                    CANCEL_POLL.min(self.deadline.saturating_duration_since(Instant::now())),
                )
                .await;
            }
        };
        match futures_util::future::select(Box::pin(future), Box::pin(cancellation)).await {
            futures_util::future::Either::Left((value, _)) => {
                self.check()?;
                Ok(value)
            }
            futures_util::future::Either::Right((error, pending)) => {
                drop(pending);
                Err(error)
            }
        }
    }
}
impl Drop for ReadGuard {
    fn drop(&mut self) {
        if self.timer_live.replace(false) {
            if let Some(timer) = self.timer.take() {
                timer.remove();
            }
        }
        self.cancel.cancel();
    }
}

fn transfer_guard(
    broker: &Broker,
    epoch: u64,
    ticket: &ruwac_clipboard::DesktopTransfer,
) -> ReadGuard {
    let (broker, ticket) = (broker.clone(), ticket.clone());
    ReadGuard::new(move || {
        broker.borrow().epoch() == epoch && broker.borrow().transfer_valid(&ticket)
    })
}
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
    let guard = transfer_guard(&broker, epoch, &ticket);
    let cancel = guard.cancel.clone();
    clip.read_async(
        &["text/plain;charset=utf-8", "text/plain"],
        glib::Priority::DEFAULT,
        Some(&cancel),
        move |result| {
            let (stream, mime) = match result {
                Ok(v) => v,
                Err(e) => {
                    broker
                        .borrow_mut()
                        .finish_transfer(&ticket, 0, TransferState::Cancelled);
                    done(Err(guard.check().err().unwrap_or_else(|| e.to_string())));
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
                            guard.wait(glib::timeout_future(delay)).await?;
                        }
                        let chunk = guard
                            .wait(stream.read_bytes_future(65536, glib::Priority::DEFAULT))
                            .await?
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
                let _ = guard
                    .wait(stream.close_future(glib::Priority::DEFAULT))
                    .await;
                let result = result.and_then(|value| guard.check().map(|()| value));
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
    let guard = transfer_guard(&broker, epoch, &ticket);
    let cancel = guard.cancel.clone();
    clip.read_async(
        &["image/png"],
        glib::Priority::DEFAULT,
        Some(&cancel),
        move |result| {
            let (stream, mime) = match result {
                Ok(v) => v,
                Err(e) => {
                    broker
                        .borrow_mut()
                        .finish_transfer(&ticket, 0, TransferState::Cancelled);
                    done(Err(guard.check().err().unwrap_or_else(|| e.to_string())));
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
                            guard.wait(glib::timeout_future(delay)).await?;
                        }
                        let chunk = guard
                            .wait(stream.read_bytes_future(65536, glib::Priority::DEFAULT))
                            .await?
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
                let _ = guard
                    .wait(stream.close_future(glib::Priority::DEFAULT))
                    .await;
                let result = result.and_then(|value| guard.check().map(|()| value));
                drop(guard);
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
                                broker.borrow_mut().finish_transfer(
                                    &ticket,
                                    0,
                                    TransferState::Cancelled,
                                );
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
                            broker.borrow_mut().finish_transfer(
                                &ticket,
                                0,
                                TransferState::Cancelled,
                            );
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
    let check_broker = broker.clone();
    let check_view = view.clone();
    let guard = ReadGuard::new(move || {
        check_broker.borrow().epoch() == epoch
            && check_broker.borrow().active() == check_view.as_deref()
    });
    let cancel = guard.cancel.clone();
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
                            let chunk = guard
                                .wait(stream.read_bytes_future(65536, glib::Priority::DEFAULT))
                                .await?
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
                    let _ = guard
                        .wait(stream.close_future(glib::Priority::DEFAULT))
                        .await;
                    result
                };
                let result = guard.wait(read).await.and_then(|r| r);
                done(result);
            });
        },
    );
}

#[cfg(test)]
mod stalled_stream_tests {
    use super::*;
    use std::{io::Write, os::unix::net::UnixStream, sync::Mutex};
    static CONTEXT: Mutex<()> = Mutex::new(());

    fn exercise(focus_cancel: bool) {
        let _serial = CONTEXT.lock().unwrap();
        let context = glib::MainContext::default();
        let _owner = context.acquire().unwrap();
        let (local, mut peer) = UnixStream::pair().unwrap();
        let socket = gio::Socket::from_fd(local.into()).unwrap();
        let connection = socket.connection_factory_create_connection();
        let input = connection.input_stream();
        let valid = Rc::new(Cell::new(true));
        let check = valid.clone();
        let guard = ReadGuard::new(move || check.get());
        if focus_cancel {
            glib::timeout_add_local_once(Duration::from_millis(100), move || valid.set(false));
        }
        let started = Instant::now();
        let result =
            context.block_on(guard.wait(input.read_bytes_future(65536, glib::Priority::DEFAULT)));
        assert!(result.is_err());
        assert!(
            started.elapsed()
                < if focus_cancel {
                    Duration::from_secs(1)
                } else {
                    Duration::from_secs(6)
                }
        );
        assert!(result.unwrap_err().contains(if focus_cancel {
            "focus/owner"
        } else {
            "deadline"
        }));
        drop(guard);
        // GIO's cancellation callback must release the pending-operation flag;
        // otherwise a subsequent ordinary paste fails even after our callback returns.
        let cleanup_limit = Instant::now() + Duration::from_secs(1);
        while input.has_pending() {
            assert!(Instant::now() < cleanup_limit);
            context.iteration(false);
        }
        peer.write_all(b"generated recovery bytes").unwrap();
        let recovery = ReadGuard::new(|| true);
        let bytes = context
            .block_on(recovery.wait(input.read_bytes_future(65536, glib::Priority::DEFAULT)))
            .unwrap()
            .unwrap();
        assert_eq!(bytes.as_ref(), b"generated recovery bytes");
    }
    #[test]
    fn focus_change_cancels_a_blocked_gio_read_and_allows_a_fresh_read() {
        exercise(true);
    }
    #[test]
    fn deadline_cancels_a_blocked_gio_read_and_allows_a_fresh_read() {
        exercise(false);
    }
}
