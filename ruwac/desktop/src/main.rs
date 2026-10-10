mod clipboard;
mod view;
mod worker;
use adw::prelude::*;
use gtk::{gdk, glib};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
use view::View;
fn main() {
    if let Err(e) = run() {
        eprintln!("ruwac-desktop: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut socket = None;
    let mut sessions = Vec::new();
    let mut fixture = None;
    let mut stage = None;
    let mut clipboard_disabled = false;
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" => {
                i += 1;
                socket = Some(PathBuf::from(args.get(i).ok_or("missing socket")?));
            }
            "--session" => {
                i += 1;
                sessions.push(args.get(i).ok_or("missing session")?.clone());
            }
            "--fixture-dir" => {
                i += 1;
                fixture = Some(PathBuf::from(
                    args.get(i).ok_or("missing fixture directory")?,
                ));
            }
            "--staging-dir" => {
                i += 1;
                stage = Some(PathBuf::from(
                    args.get(i).ok_or("missing staging directory")?,
                ));
            }
            "--clipboard-disabled" => clipboard_disabled = true,
            "--version" => {
                println!("RUWAC desktop {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" => {
                println!(
                    "ruwac-desktop --socket PRIVATE_SOCKET [--session SESSION]... [--staging-dir PRIVATE_DIRECTORY]\nCtrl+A prefix; Ctrl+Shift+C/V desktop selection; Shift+drag overrides application mouse.\nOptional independent GTK client; daemon owns PTYs. Image staging is not agent attachment."
                );
                return Ok(());
            }
            _ => return Err("unknown option".into()),
        }
        i += 1;
    }
    let socket = socket.ok_or("--socket required")?;
    // Session creation precedes the GTK main loop; no synchronous RPC runs in UI callbacks.
    if sessions.is_empty() {
        let mut rpc = worker::Rpc::connect(&socket)?;
        for name in ["Terminal 1", "Terminal 2"] {
            sessions.push(
                rpc.call("session.create", json!({"name":name}))?["session"]
                    .as_str()
                    .ok_or("no session ID")?
                    .into(),
            );
        }
    }
    for id in &sessions {
        if !ruwac_protocol::valid_id(id, "session") {
            return Err("invalid session ID".into());
        }
    }
    if let Some(dir) = fixture.as_ref() {
        use std::os::unix::fs::MetadataExt;
        let m = std::fs::symlink_metadata(dir)?;
        if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
            return Err("fixture directory must be owned and 0700".into());
        }
    }
    if stage.is_none() {
        if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
            stage = Some(PathBuf::from(runtime).join("ruwac-image-stage"));
        }
    }
    if let Some(dir) = stage.as_ref() {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        if !dir.exists() {
            std::fs::DirBuilder::new().mode(0o700).create(dir)?;
        }
        let m = std::fs::symlink_metadata(dir)?;
        if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
            return Err("staging directory must be owned and 0700".into());
        }
    }
    let app = adw::Application::builder()
        .application_id("si.z4.ruwac.desktop")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let views: Rc<RefCell<Vec<Rc<View>>>> = Rc::new(RefCell::new(Vec::new()));
    let all = views.clone();
    app.connect_activate(move|app|{
  let window=adw::ApplicationWindow::builder().application(app).title("RUWAC — terminal workspace candidate").default_width(1040).default_height(780).build();
  let box_=gtk::Box::new(gtk::Orientation::Vertical,0);let header=adw::HeaderBar::new();box_.append(&header);let tabs=adw::TabView::new();tabs.set_vexpand(true);let bar=adw::TabBar::builder().view(&tabs).autohide(false).build();box_.append(&bar);
  let broker=Rc::new(RefCell::new(ruwac_clipboard::FocusBroker::default()));
  broker.borrow_mut().set_desktop_disabled(clipboard_disabled);
  let b=broker.clone();gtk::prelude::WidgetExt::display(&window).clipboard().connect_changed(move|_|b.borrow_mut().local_copy());
  let b=broker.clone();gtk::prelude::WidgetExt::display(&window).primary_clipboard().connect_changed(move|_|b.borrow_mut().local_copy());
  let owned_images:Rc<RefCell<Vec<PathBuf>>>=Rc::new(RefCell::new(Vec::new()));
  let clear=gtk::Button::with_label("Clear message");header.pack_end(&clear);let vc=all.clone();let tc=tabs.clone();clear.connect_clicked(move|_|{if let Some(page)=tc.selected_page(){for v in vc.borrow().iter(){if page.child()==v.root{*v.warning.borrow_mut()=None;v.status.set_text("Delivery warning acknowledged");}}}});
  let image_button=gtk::Button::with_label("Stage clipboard PNG");header.pack_end(&image_button);
  let vb=all.clone();let tb=tabs.clone();let sb=stage.clone();let owned=owned_images.clone();
  image_button.connect_clicked(move|_|{let Some(page)=tb.selected_page()else{return;};let Some(v)=vb.borrow().iter().find(|v|page.child()==v.root).cloned()else{return;};let Some(dir)=sb.clone()else{v.status.set_text("Private staging directory is unavailable");return;};let epoch=v.broker.borrow().epoch();let weak=Rc::downgrade(&v);let owned=owned.clone();clipboard::ingest_png(&v.area.display().clipboard(),dir,v.broker.clone(),epoch,ruwac_clipboard::MAX_IMAGE_BYTES,Duration::ZERO,move|r|if let Some(v)=weak.upgrade(){match r{Ok(r)=>{owned.borrow_mut().push(r.path);v.status.set_text(&format!("PNG staged: {}×{}, {} bytes; no agent attachment",r.width,r.height,r.bytes));},Err(e)=>v.status.set_text(&e)}});});
  for (i,session) in sessions.iter().enumerate(){let v=View::new(socket.clone(),session.clone(),broker.clone());let page=tabs.append(&v.root);page.set_title(&format!("Terminal {} · {}",i+1,&session[..session.len().min(16)]));all.borrow_mut().push(v);}
  box_.append(&tabs);window.set_content(Some(&box_));
  let vv=all.clone();tabs.connect_selected_page_notify(move|t|{if let Some(page)=t.selected_page(){for v in vv.borrow().iter(){if page.child()==v.root{v.focus();break;}}}});
  let vv=all.clone();tabs.connect_close_page(move|_,page|{for v in vv.borrow().iter(){if page.child()==v.root{v.close();}}glib::Propagation::Proceed});
  let owned=owned_images.clone();let vv=all.clone();window.connect_close_request(move|_|{for v in vv.borrow().iter(){v.close();}for path in owned.borrow_mut().drain(..){let _=std::fs::remove_file(path);}glib::Propagation::Proceed});window.present();if let Some(v)=all.borrow().first(){v.focus();}
  let vv=all.clone();let tabs=tabs.clone();let app=app.clone();let fixture=fixture.clone();let staging=stage.clone();let owned=owned_images.clone();let tick=Rc::new(Cell::new(0u64));let last=Rc::new(Cell::new(Instant::now()));let max_gap=Rc::new(Cell::new(0u128));let sequence=Rc::new(Cell::new(0u64));
  glib::timeout_add_local(Duration::from_millis(16),move||{
   tick.set(tick.get()+1);let now=Instant::now();max_gap.set(max_gap.get().max(now.duration_since(last.replace(now)).as_millis()));for v in vv.borrow().iter(){v.update();}
   if let Some(dir)=fixture.as_ref(){let request=dir.join("gui-request.json");if let Ok(bytes)=std::fs::read(&request){if let Ok(q)=serde_json::from_slice::<Value>(&bytes){let seq=q["seq"].as_u64().unwrap_or(0);if seq>sequence.get(){sequence.set(seq);let index=q["view"].as_u64().unwrap_or(0)as usize;let view=vv.borrow().get(index).cloned();let reply_dir=dir.clone();let reply=move|result:Value|{let path=reply_dir.join("gui-response.json");let temp=reply_dir.join("gui-response.tmp");let _=std::fs::write(&temp,json!({"seq":seq,"result":result}).to_string());let _=std::fs::rename(temp,path);};if let Some(v)=view{match q["action"].as_str().unwrap_or(""){
    "switch"=>{let page=tabs.page(&v.root);tabs.set_selected_page(&page);if let Some(pane)=q["pane"].as_str(){*v.target.borrow_mut()=pane.into();v.command("pane.select",json!({"pane":pane}));}v.focus();if let Some(text)=q["text"].as_str(){for c in text.chars(){let key=unsafe{glib::translate::from_glib(gdk::ffi::gdk_unicode_to_keyval(c as u32))};v.key(key,gdk::ModifierType::empty());}}reply(json!({"queued":true}));},
    "key"=>{if let Some(key)=q["key"].as_str().and_then(gdk::Key::from_name){let mods=gdk::ModifierType::from_bits_truncate(q["mods"].as_u64().unwrap_or(0)as u32);v.key(key,mods);}reply(json!({"handler":"synthetic GTK callback"}));},
    "copy"=>{v.select_fixture();v.copy_clipboard(q["primary"]==true);reply(json!({"offered":true}));},
    "paste"=>{v.paste_clipboard(q["primary"]==true);reply(json!({"async":true}));},
    "read-text"=>{let epoch=v.broker.borrow().epoch();clipboard::read_text(&v.area.display().clipboard(),v.broker.clone(),epoch,ruwac_clipboard::ClipScope::Clipboard,Duration::from_millis(q["delay_ms"].as_u64().unwrap_or(0).min(100)),move|r|{use sha2::{Digest,Sha256};reply(match r{Ok(t)=>json!({"bytes":t.len(),"sha256":format!("{:x}",Sha256::digest(t.as_bytes()))}),Err(e)=>json!({"error":e})});});},
    "read-image"=>{if let Some(stage)=staging.clone(){let epoch=v.broker.borrow().epoch();let owned=owned.clone();clipboard::ingest_png(&v.area.display().clipboard(),stage,v.broker.clone(),epoch,q["limit"].as_u64().unwrap_or(ruwac_clipboard::MAX_IMAGE_BYTES as u64)as usize,Duration::from_millis(q["delay_ms"].as_u64().unwrap_or(0).min(100)),move|r|reply(match r{Ok(r)=>{owned.borrow_mut().push(r.path.clone());json!({"path":r.path,"sha256":r.sha256,"bytes":r.bytes,"width":r.width,"height":r.height})},Err(e)=>json!({"error":e})}));}else{reply(json!({"error":"staging directory required"}));}},
    "mouse"=>{v.mouse(q["button"].as_u64().unwrap_or(0)as u32,q["x"].as_f64().unwrap_or(1.0),q["y"].as_f64().unwrap_or(1.0),q["release"]==true);reply(json!({"handler":"synthetic mouse callback"}));},
    "delay-queue"=>{v.command("pane.wait",json!({"pane":*v.target.borrow(),"contains":"not-present-delayed-fixture","timeout_ms":1000}));for _ in 0..160{v.command("pane.select",json!({"pane":*v.target.borrow()}));}reply(json!({"requested":160}));},
    "policy"=>{v.broker.borrow_mut().set_desktop_disabled(q["disabled"]==true);reply(json!({"disabled":q["disabled"]==true}));},
    "grid-copy"=>{v.select_grid_fixture();v.copy_clipboard(false);reply(json!({"offered":true}));},
    "cancel"=>{v.broker.borrow_mut().local_copy();reply(json!({"epoch":v.broker.borrow().epoch()}));},
    "snapshot"=>{let paint=gtk::WidgetPaintable::new(Some(&window));let snap=gtk::Snapshot::new();paint.snapshot(&snap,window.width()as f64,window.height()as f64);if let(Some(node),Some(renderer))=(snap.to_node(),window.renderer()){let texture=renderer.render_texture(&node,None);let _=texture.save_to_png(dir.join("native-rust.png"));}reply(json!({"image":"native-rust.png"}));},
    "inspect"=>reply(json!({"display":v.area.display().type_().name(),"active":window.is_active(),"tabs":tabs.n_pages(),"drawn":vv.borrow().iter().map(|v|v.drawn.get()).collect::<Vec<_>>(),"panes":vv.borrow().iter().map(|v|v.frame.borrow().as_ref().and_then(|f|f["panes"].as_array()).map(|a|a.len()).unwrap_or(0)).collect::<Vec<_>>(),"ticks":tick.get(),"max_tick_gap_ms":max_gap.get(),"acknowledgements":v.worker.state.lock().unwrap().acknowledgements,"status":v.status.text().to_string()})),
    "quit"=>{for v in vv.borrow().iter(){v.close();}for path in owned.borrow_mut().drain(..){let _=std::fs::remove_file(path);}reply(json!({"detached":true}));app.quit();return glib::ControlFlow::Break;},
    _=>reply(json!({"error":"unknown fixture action"}))
   }}else{reply(json!({"error":"view not found"}));}}}}}
   glib::ControlFlow::Continue
  });
 });
    app.run_with_args::<&str>(&[]);
    for view in views.borrow().iter() {
        view.close();
    }
    Ok(())
}
