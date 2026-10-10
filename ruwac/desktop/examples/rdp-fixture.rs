//! Test-only localhost RDP server. Run against a new private fixture directory.
#[path = "../src/cliprdr.rs"]
mod cliprdr;
use cliprdr::{Backend, Bridge, Event, Format};
use ironrdp_cliprdr::{
    backend::{ClipboardMessage, CliprdrBackendFactory},
    pdu::FormatDataResponse,
};
use ironrdp_server::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    num::{NonZeroU16, NonZeroUsize},
    os::unix::fs::MetadataExt,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
struct Display;
struct Updates;
#[async_trait::async_trait]
impl RdpServerDisplay for Display {
    async fn size(&mut self) -> DesktopSize {
        DesktopSize {
            width: 320,
            height: 180,
        }
    }
    async fn updates(&mut self) -> anyhow::Result<Box<dyn RdpServerDisplayUpdates>> {
        Ok(Box::new(Updates))
    }
}
#[async_trait::async_trait]
impl RdpServerDisplayUpdates for Updates {
    async fn next_update(&mut self) -> anyhow::Result<Option<DisplayUpdate>> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut data = vec![0; 320 * 180 * 4];
        for (y, row) in data.chunks_mut(1280).enumerate() {
            for (x, p) in row.chunks_mut(4).enumerate() {
                p.copy_from_slice(&[x as u8, y as u8, 90, 255]);
            }
        }
        Ok(Some(DisplayUpdate::Bitmap(BitmapUpdate {
            x: 0,
            y: 0,
            width: NonZeroU16::new(320).unwrap(),
            height: NonZeroU16::new(180).unwrap(),
            format: PixelFormat::BgrX32,
            data: bytes::Bytes::from(data),
            stride: NonZeroUsize::new(1280).unwrap(),
        })))
    }
}
struct Factory(Bridge);
impl CliprdrBackendFactory for Factory {
    fn build_cliprdr_backend(&self) -> Box<dyn ironrdp_cliprdr::backend::CliprdrBackend> {
        Box::new(Backend::new(self.0.clone()))
    }
}
impl ServerEventSender for Factory {
    fn set_sender(&mut self, _: tokio::sync::mpsc::UnboundedSender<ServerEvent>) {}
}
impl CliprdrServerFactory for Factory {}
fn main() -> anyhow::Result<()> {
    let dir = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("Private fixture directory required"))?,
    );
    let m = std::fs::symlink_metadata(&dir)?;
    anyhow::ensure!(
        m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0,
        "Fixture must be owned, private and not a symlink"
    );
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(tokio::task::LocalSet::new().run_until(run(dir)))
}
async fn run(dir: PathBuf) -> anyhow::Result<()> {
    let mut ports = Vec::new();
    for tab in 0..3 {
        let identity =
            TlsIdentityCtx::init_from_paths(&dir.join("cert.pem"), &dir.join("key.pem"))?;
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(identity.certs, identity.priv_key)?;
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        ports.push(port);
        let bridge = Bridge::default();
        let mut server = RdpServer::builder()
            .with_addr(([127, 0, 0, 1], port))
            .with_hybrid(acceptor, identity.pub_key)
            .with_no_input()
            .with_display_handler(Display)
            .with_cliprdr_factory(Some(Box::new(Factory(bridge.clone()))))
            .build();
        server.set_credentials(Some(Credentials {
            username: "fixture".into(),
            password: "generated-local-only".into(),
            domain: None,
        }));
        let sender = server.event_sender().clone();
        let d = dir.clone();
        tokio::task::spawn_local(async move {
            loop {
                match listener.accept().await {
                    Ok((s, _)) => {
                        let _ = server.run_connection(s).await;
                    }
                    Err(_) => break,
                }
            }
        });
        tokio::task::spawn_local(async move {
            let mut seq = 0;
            let mut offered = Vec::new();
            let mut offered_kind = Format::Text;
            let mut requested = Format::Text;
            let mut received = 0;
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                if let Ok(b) = std::fs::read(d.join(format!("server-{tab}-request.json"))) {
                    if let Ok(q) = serde_json::from_slice::<serde_json::Value>(&b) {
                        if let Some(n) = q["seq"].as_u64() {
                            if n > seq {
                                seq = n;
                                let result = (|| -> Result<(), String> {
                                    if q["action"].as_str() == Some("disconnect") {
                                        sender
                                            .send(ServerEvent::Quit(
                                                "owned reconnect fixture".into(),
                                            ))
                                            .map_err(|_| "Fixture ended")?;
                                        return Ok(());
                                    }
                                    offered_kind = match q["format"].as_str() {
                                        Some("png") => Format::Png,
                                        Some("dib") => Format::Dib,
                                        Some("dibv5") => Format::DibV5,
                                        _ => Format::Text,
                                    };
                                    let name = q["file"]
                                        .as_str()
                                        .ok_or("Generated payload file required")?;
                                    if name.contains('/') || name.contains("..") {
                                        return Err("Payload must be fixture-local".into());
                                    }
                                    use std::io::Read;
                                    use std::os::unix::fs::OpenOptionsExt;
                                    let f = std::fs::OpenOptions::new()
                                        .read(true)
                                        .custom_flags(libc::O_NOFOLLOW)
                                        .open(d.join(name))
                                        .map_err(|_| "Cannot open fixture payload")?;
                                    offered.clear();
                                    f.take(64 * 1024 * 1024 + 1)
                                        .read_to_end(&mut offered)
                                        .map_err(|_| "Cannot read fixture payload")?;
                                    cliprdr::encode(offered_kind, &offered)?;
                                    let formats = if offered_kind == Format::Png {
                                        vec![ironrdp_cliprdr::pdu::ClipboardFormat::new(cliprdr::PNG_ID).with_name(ironrdp_cliprdr::pdu::ClipboardFormatName::new_static("PNG"))]
                                    } else {
                                        vec![ironrdp_cliprdr::pdu::ClipboardFormat::new(
                                            offered_kind.id(),
                                        )]
                                    };
                                    sender
                                        .send(ServerEvent::Clipboard(
                                            ClipboardMessage::SendInitiateCopy(formats),
                                        ))
                                        .map_err(|_| "Fixture server ended")?;
                                    Ok(())
                                })();
                                let _=std::fs::write(d.join(format!("server-{tab}-response.json")),json!({"seq":seq,"result":{"offered":result.is_ok(),"error":result.err()}}).to_string());
                            }
                        }
                    }
                }
                for e in bridge.drain() {
                    match e {
                        Event::Request(_) => {
                            let data = cliprdr::encode(offered_kind, &offered).ok();
                            let _ = sender.send(ServerEvent::Clipboard(
                                ClipboardMessage::SendFormatData(match data {
                                    Some(b) => FormatDataResponse::new_data(b),
                                    None => FormatDataResponse::new_error(),
                                }),
                            ));
                        }
                        Event::Offer {
                            selected: Some((id, kind)),
                            ..
                        } => {
                            requested = kind;
                            let _ = sender.send(ServerEvent::Clipboard(
                                ClipboardMessage::SendInitiatePaste(id),
                            ));
                        }
                        Event::Data(data) => {
                            received += 1;
                            let result = data.and_then(|b| cliprdr::decode(requested, &b));
                            let receipt = match result {
                                Ok(b) => {
                                    let mut r = json!({"received":received,"mime":requested.mime(),"bytes":b.len(),"sha256":format!("{:x}",Sha256::digest(&b))});
                                    if requested != Format::Text {
                                        if let Ok(mut reader) =
                                            png::Decoder::new(std::io::Cursor::new(&b)).read_info()
                                        {
                                            let mut pixels =
                                                vec![0; reader.output_buffer_size().unwrap_or(0)];
                                            if let Ok(info) = reader.next_frame(&mut pixels) {
                                                r["pixel_sha256"] = json!(format!(
                                                    "{:x}",
                                                    Sha256::digest(&pixels[..info.buffer_size()])
                                                ));
                                                r["width"] = json!(info.width);
                                                r["height"] = json!(info.height);
                                            }
                                        }
                                    }
                                    r
                                }
                                Err(e) => json!({"received":received,"error":e}),
                            };
                            let _ = std::fs::write(
                                d.join(format!("server-{tab}-receipt.json")),
                                receipt.to_string(),
                            );
                        }
                        _ => (),
                    }
                }
            }
        });
    }
    std::fs::write(dir.join("server-ready.json"),json!({"ports":ports,"server":"ironrdp-server 0.13.0","security":"TLS verified + CredSSP/NLA","generated_desktop":[320,180]}).to_string())?;
    std::future::pending::<()>().await;
    Ok(())
}
