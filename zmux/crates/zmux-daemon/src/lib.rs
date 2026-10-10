use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::{Notify, Semaphore},
};
use zmux_core::{Handle, Message};
use zmux_protocol::{Error, Request, id, read_async, request, response, write_async};
struct Guard {
    socket: PathBuf,
    dev: u64,
    ino: u64,
    _lock: File,
}
impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(m) = fs::symlink_metadata(&self.socket) {
            if m.dev() == self.dev && m.ino() == self.ino {
                let _ = fs::remove_file(&self.socket);
            }
        }
    }
}
pub async fn serve(socket: &Path) -> std::io::Result<()> {
    let uid = unsafe { libc::geteuid() };
    let parent = socket
        .parent()
        .ok_or_else(|| std::io::Error::other("socket needs parent directory"))?;
    if !parent.exists() {
        fs::DirBuilder::new().mode(0o700).create(parent)?;
    }
    let m = fs::symlink_metadata(parent)?;
    if !m.is_dir() || m.uid() != uid || m.mode() & 0o077 != 0 {
        return Err(std::io::Error::other(
            "socket parent must be an owned private directory (0700)",
        ));
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(socket.with_extension("lock"))?;
    let m = lock.metadata()?;
    if !m.is_file() || m.uid() != uid || m.mode() & 0o077 != 0 || m.nlink() != 1 {
        return Err(std::io::Error::other("unsafe lock file"));
    }
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(std::io::Error::other("daemon already owns start lock"));
    }
    if let Ok(m) = fs::symlink_metadata(socket) {
        if !m.file_type().is_socket() || m.uid() != uid {
            return Err(std::io::Error::other(
                "refusing to replace non-owned socket",
            ));
        }
        if UnixStream::connect(socket).await.is_ok() {
            return Err(std::io::Error::other("socket already active"));
        }
        fs::remove_file(socket)?;
    }
    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
    let m = fs::symlink_metadata(socket)?;
    let _guard = Guard {
        socket: socket.into(),
        dev: m.dev(),
        ino: m.ino(),
        _lock: lock,
    };
    let handle = zmux_core::start();
    let stop = Arc::new(Notify::new());
    let limit = Arc::new(Semaphore::new(64));
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {_ = stop.notified()=>break,_=tokio::signal::ctrl_c()=>break,_=term.recv()=>break, accepted=listener.accept()=>{let(stream,_)=accepted?;if stream.peer_cred()?.uid()!=uid{continue}let Ok(permit)=limit.clone().try_acquire_owned()else{continue};let h=handle.clone();let stop=stop.clone();tokio::spawn(async move{let _permit=permit;let client=id("client");let _=connection(stream,&h,&client,stop).await;let _=h.tx.send(Message::Disconnect(client)).await;});}}
    }
    let _ = handle
        .call(request("daemon.stop", json!({})), "shutdown")
        .await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    Ok(())
}
use std::os::unix::fs::DirBuilderExt;
async fn write(stream: &mut UnixStream, v: &Value) -> std::io::Result<()> {
    tokio::time::timeout(Duration::from_secs(10), write_async(stream, v))
        .await
        .map_err(|_| std::io::Error::other("write timeout"))?
}
async fn connection(
    mut stream: UnixStream,
    h: &Handle,
    c: &str,
    stop: Arc<Notify>,
) -> std::io::Result<()> {
    loop {
        let b = match tokio::time::timeout(Duration::from_secs(60), read_async(&mut stream)).await {
            Ok(Ok(b)) => b,
            _ => return Ok(()),
        };
        let r: Request = match serde_json::from_slice(&b) {
            Ok(r) => r,
            Err(e) => {
                write(
                    &mut stream,
                    &response(
                        &Value::Null,
                        Err(Error::new("INVALID_REQUEST", e.to_string())),
                    ),
                )
                .await?;
                continue;
            }
        };
        if r.method == "events.subscribe" {
            if r.version != 0 {
                write(
                    &mut stream,
                    &response(
                        &r.id,
                        Err(Error::new("VERSION", "supported protocol version is 0")),
                    ),
                )
                .await?;
                continue;
            }
            write(
                &mut stream,
                &response(
                    &r.id,
                    Ok(json!({"subscribed":true,"revision":*h.revision.borrow()})),
                ),
            )
            .await?;
            let mut rev = h.revision.clone();
            loop {
                if rev.changed().await.is_err() {
                    return Ok(());
                }
                let n = *rev.borrow_and_update();
                write(
                    &mut stream,
                    &json!({"version":0,"event":"state.changed","revision":n}),
                )
                .await?;
            }
        }
        if r.method == "pane.wait" {
            let timeout = r
                .params
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or(20000);
            if timeout > 60000 || r.params.get("contains").and_then(Value::as_str).is_none() {
                write(
                    &mut stream,
                    &response(
                        &r.id,
                        Err(Error::new(
                            "INVALID_PARAMS",
                            "contains required; timeout_ms max 60000",
                        )),
                    ),
                )
                .await?;
                continue;
            }
            let mut rev = h.revision.clone();
            let result = tokio::time::timeout(Duration::from_millis(timeout), async {
                loop {
                    let out = h.call(r.clone(), c).await?;
                    if out["matched"] == true {
                        return Ok(out);
                    }
                    if out["ended"] == true {
                        return Err(Error::new("PANE_EXITED", "pane exited before match"));
                    }
                    rev.changed()
                        .await
                        .map_err(|_| Error::new("UNAVAILABLE", "daemon stopped"))?;
                }
            })
            .await
            .unwrap_or_else(|_| Err(Error::new("TIMEOUT", "text not observed before deadline")));
            write(&mut stream, &response(&r.id, result)).await?;
            continue;
        }
        let result = tokio::time::timeout(Duration::from_secs(10), h.call(r.clone(), c))
            .await
            .unwrap_or_else(|_| {
                Err(Error::new(
                    "TIMEOUT",
                    "request timed out; mutation outcome may be unknown",
                ))
            });
        let stopping = r.method == "daemon.stop" && result.is_ok();
        let written = write(&mut stream, &response(&r.id, result)).await;
        if stopping {
            stop.notify_one();
        }
        written?;
        if stopping {
            return Ok(());
        }
    }
}
