mod clipboard;
mod cliprdr;
mod credentials;
mod files;
mod profiles;
mod rdp;
mod staging;
mod sessions;
mod workspace;
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
    let mut rpc = worker::Rpc::connect(&socket)?;
    let available = rpc.call("session.list", json!({}))?;
    let saved = workspace::load(&socket);
    let workspace_warning = saved.as_ref().err().map(|e| format!("Workspace restore: {e}"));
    let selected = saved.as_ref().ok().and_then(|w| w.selected.clone());
    if sessions.is_empty() {
        if let Ok(w) = saved.as_ref() {
            sessions.extend(w.sessions.iter().filter(|id| available.as_array().is_some_and(|rows| rows.iter().any(|r| r["id"] == id.as_str()))).cloned());
        }
        if sessions.is_empty() {
            if let Some(id) = available.as_array().and_then(|rows| rows.first()).and_then(|r| r["id"].as_str()) {sessions.push(id.into());}
            else {sessions.push(rpc.call("session.create", json!({"name":"Terminal"}))?["session"].as_str().ok_or("no session ID")?.into());}
        }
    }
    let session_names: std::collections::HashMap<String,String> = available.as_array().into_iter().flatten().filter_map(|r|Some((r["id"].as_str()?.into(),r["name"].as_str()?.into()))).collect();
    drop(rpc);
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
    let stage_guard = stage.as_ref().map(|p| staging::Stage::new(p)).transpose()?;
    stage = stage_guard.as_ref().map(|s| s.path.clone());
    let app = adw::Application::builder()
        .application_id("si.z4.ruwac")
        .flags(if fixture.is_some() {gtk::gio::ApplicationFlags::NON_UNIQUE} else {gtk::gio::ApplicationFlags::FLAGS_NONE})
        .build();
    let views: Rc<RefCell<Vec<Rc<View>>>> = Rc::new(RefCell::new(Vec::new()));
    let all = views.clone();
    app.connect_activate(move|app|{
  if let Some(window)=app.active_window(){window.present();return;}
  let window=adw::ApplicationWindow::builder().application(app).title("RUWAC").default_width(1040).default_height(780).build();
  let box_=gtk::Box::new(gtk::Orientation::Vertical,0);let header=adw::HeaderBar::new();box_.append(&header);let tabs=adw::TabView::new();tabs.set_vexpand(true);let bar=adw::TabBar::builder().view(&tabs).autohide(false).build();box_.append(&bar);
  if let Some(message)=workspace_warning.as_ref(){box_.append(&gtk::Label::new(Some(message)));}
  let broker=Rc::new(RefCell::new(ruwac_clipboard::FocusBroker::default()));
  broker.borrow_mut().set_desktop_disabled(clipboard_disabled);
  let b=broker.clone();gtk::prelude::WidgetExt::display(&window).clipboard().connect_changed(move|_|b.borrow_mut().local_copy());
  let b=broker.clone();gtk::prelude::WidgetExt::display(&window).primary_clipboard().connect_changed(move|_|{let pending=b.borrow().last_transfer.as_ref().is_some_and(|t|t.scope==ruwac_clipboard::ClipScope::Primary&&t.state==ruwac_clipboard::TransferState::Streaming);if pending{b.borrow_mut().local_copy();}});
  let rdp_views:Rc<RefCell<Vec<Rc<rdp::RdpView>>>>=Rc::new(RefCell::new(Vec::new()));
  let rdp_button=gtk::Button::with_label("New RDP tab");header.pack_start(&rdp_button);let rv=rdp_views.clone();let t=tabs.clone();let b=broker.clone();rdp_button.connect_clicked(move|_|{let v=rdp::RdpView::new(b.clone());let p=t.append(&v.root);p.set_title("RDP · embedded IronRDP");rv.borrow_mut().push(v.clone());t.set_selected_page(&p);v.focus();});
  let rv=rdp_views.clone();let b=broker.clone();tabs.connect_selected_page_notify(move|t|{if let Some(page)=t.selected_page(){let mut selected=false;for v in rv.borrow().iter(){if page.child()==v.root{v.focus();selected=true;}else{v.inactive();}}if !selected{b.borrow_mut().focus(None);}}});
  let rv=rdp_views.clone();tabs.connect_close_page(move|_,page|{let mut views=rv.borrow_mut();for v in views.iter(){if page.child()==v.root{v.close();}}views.retain(|v|page.child()!=v.root);glib::Propagation::Proceed});
  let rv=rdp_views.clone();window.connect_close_request(move|_|{for v in rv.borrow().iter(){v.close();}glib::Propagation::Proceed});
  let owned_images:Rc<RefCell<Vec<PathBuf>>>=Rc::new(RefCell::new(Vec::new()));
  let clear=gtk::Button::with_label("Clear message");header.pack_end(&clear);let vc=all.clone();let tc=tabs.clone();clear.connect_clicked(move|_|{if let Some(page)=tc.selected_page(){for v in vc.borrow().iter(){if page.child()==v.root{*v.warning.borrow_mut()=None;v.status.set_text("Delivery warning acknowledged");}}}});
  let image_button=gtk::Button::with_label("Stage clipboard PNG");header.pack_end(&image_button);
  let vb=all.clone();let tb=tabs.clone();let sb=stage.clone();let owned=owned_images.clone();
  image_button.connect_clicked(move|_|{let Some(page)=tb.selected_page()else{return;};let Some(v)=vb.borrow().iter().find(|v|page.child()==v.root).cloned()else{return;};let Some(dir)=sb.clone()else{v.status.set_text("Private staging directory is unavailable");return;};let epoch=v.broker.borrow().epoch();let weak=Rc::downgrade(&v);let owned=owned.clone();clipboard::ingest_png(&v.area.display().clipboard(),dir,v.broker.clone(),epoch,ruwac_clipboard::MAX_IMAGE_BYTES,Duration::ZERO,move|r|if let Some(v)=weak.upgrade(){match r{Ok(r)=>{owned.borrow_mut().push(r.path);v.status.set_text(&format!("PNG staged: {}×{}, {} bytes; no agent attachment",r.width,r.height,r.bytes));},Err(e)=>v.status.set_text(&e)}});});
  for session in &sessions{sessions::append(&tabs,&all,&socket,&broker,session,session_names.get(session).map(String::as_str).unwrap_or("Terminal"));}
  if let Some(id)=selected.as_ref(){if let Some(v)=all.borrow().iter().find(|v|&v.session==id){tabs.set_selected_page(&tabs.page(&v.root));}}
  header.pack_start(&sessions::controls(tabs.clone(),all.clone(),socket.clone(),broker.clone()));

  let rv=rdp_views.clone();let t=tabs.clone();let b=broker.clone();let(sock,views)=(socket.clone(),all.clone());
  let (sidebar,search)=profiles::sidebar(move|profile,persist,lease|{if profile.protocol=="RDP"{let v=rdp::RdpView::new(b.clone());v.configure(&profile);v.set_preferences_sink(persist);lease.bind(&v.root);let page=t.append(&v.root);page.set_title(&format!("{} / {} · {}",profile.group,profile.name,profile.protocol));rv.borrow_mut().push(v.clone());t.set_selected_page(&page);v.focus();v.connect_profile();}else{let file_panel=files::panel(profile.clone());lease.fork().bind(&file_panel);let file_page=t.append(&file_panel);file_page.set_title(&format!("{} / {} · Files",profile.group,profile.name));let socket=sock.clone();let name=profile.name.clone();let argv=profiles::ssh_argv(&profile);let(tx,rx)=std::sync::mpsc::sync_channel(1);std::thread::spawn(move||{let result=(||->Result<String,String>{let mut client=worker::Rpc::connect(&socket).map_err(|e|e.to_string())?;let reply=client.call("session.create",json!({"name":name,"argv":argv?})).map_err(|e|e.to_string())?;reply["session"].as_str().map(str::to_owned).ok_or("SSH session creation failed".into())})();let _=tx.send(result);});let(t,b,views,socket)=(t.clone(),b.clone(),views.clone(),sock.clone());let mut pending_lease=Some(lease);glib::timeout_add_local(Duration::from_millis(10),move||{match rx.try_recv(){Ok(Ok(session))=>{let v=View::new(socket.clone(),session,b.clone());if let Some(lease)=pending_lease.take(){lease.bind(&v.root);}let page=t.append(&v.root);page.set_title(&format!("{} / {} · {}",profile.group,profile.name,profile.protocol));views.borrow_mut().push(v.clone());t.set_selected_page(&page);v.focus();glib::ControlFlow::Break},Ok(Err(_))|Err(std::sync::mpsc::TryRecvError::Disconnected)=>glib::ControlFlow::Break,Err(std::sync::mpsc::TryRecvError::Empty)=>glib::ControlFlow::Continue}});}});
  let body=gtk::Box::new(gtk::Orientation::Horizontal,0);let reveal=gtk::Revealer::builder().child(&sidebar).reveal_child(true).build();body.append(&reveal);body.append(&tabs);tabs.set_hexpand(true);box_.append(&body);let toggle=gtk::Button::with_label("Machines");header.pack_start(&toggle);let r=reveal.clone();toggle.connect_clicked(move|_|r.set_reveal_child(!r.reveals_child()));let keys=gtk::EventControllerKey::new();keys.set_propagation_phase(gtk::PropagationPhase::Capture);keys.connect_key_pressed(move|_,key,_,mods|{if key==gdk::Key::k&&mods.contains(gdk::ModifierType::CONTROL_MASK){reveal.set_reveal_child(true);search.grab_focus();glib::Propagation::Stop}else{glib::Propagation::Proceed}});window.add_controller(keys);window.set_content(Some(&box_));
  let vv=all.clone();tabs.connect_selected_page_notify(move|t|{if let Some(page)=t.selected_page(){for v in vv.borrow().iter(){if page.child()==v.root{v.focus();break;}}}});
  let vv=all.clone();tabs.connect_close_page(move|_,page|{let mut views=vv.borrow_mut();for v in views.iter(){if page.child()==v.root{v.close();}}views.retain(|v|page.child()!=v.root);glib::Propagation::Proceed});
  let owned=owned_images.clone();let vv=all.clone();let saved_tabs=tabs.clone();let saved_socket=socket.clone();window.connect_close_request(move|_|{let open=vv.borrow();let mut ids=Vec::new();for i in 0..saved_tabs.n_pages(){let page=saved_tabs.nth_page(i);if let Some(v)=open.iter().find(|v|page.child()==v.root){ids.push(v.session.clone());}}let selected=saved_tabs.selected_page().and_then(|p|open.iter().find(|v|p.child()==v.root).map(|v|v.session.clone()));if let Err(e)=workspace::save(&workspace::Workspace{schema:1,socket:saved_socket.clone(),sessions:ids,selected}){eprintln!("Workspace save failed: {e}");}drop(open);for v in vv.borrow().iter(){v.close();}for path in owned.borrow_mut().drain(..){let _=std::fs::remove_file(path);}glib::Propagation::Proceed});window.present();if let Some(page)=tabs.selected_page(){for v in all.borrow().iter(){if page.child()==v.root{v.focus();}}}
  let vv=all.clone();let tabs=tabs.clone();let app=app.clone();let fixture=fixture.clone();let staging=stage.clone();let owned=owned_images.clone();let tick=Rc::new(Cell::new(0u64));let last=Rc::new(Cell::new(Instant::now()));let max_gap=Rc::new(Cell::new(0u128));let sequence=Rc::new(Cell::new(0u64));
  glib::timeout_add_local(Duration::from_millis(16),move||{
   tick.set(tick.get()+1);let now=Instant::now();max_gap.set(max_gap.get().max(now.duration_since(last.replace(now)).as_millis()));for v in vv.borrow().iter(){v.update();}
   if let Some(dir)=fixture.as_ref(){let request=dir.join("gui-request.json");if let Ok(bytes)=std::fs::read(&request){if let Ok(q)=serde_json::from_slice::<Value>(&bytes){let seq=q["seq"].as_u64().unwrap_or(0);if seq>sequence.get(){sequence.set(seq);let index=q["view"].as_u64().unwrap_or(0)as usize;let view=vv.borrow().get(index).cloned();let reply_dir=dir.clone();let reply=move|result:Value|{let path=reply_dir.join("gui-response.json");let temp=reply_dir.join("gui-response.tmp");let _=std::fs::write(&temp,json!({"seq":seq,"result":result}).to_string());let _=std::fs::rename(temp,path);};if let Some(v)=view{match q["action"].as_str().unwrap_or(""){
    "new-terminal"=>reply(json!({"clicked":sessions::fixture(&window,"New terminal")})),
    "sessions-open"=>{let label=q["session"].as_str().unwrap_or("Sessions…");let mut clicked=sessions::fixture(&window,label);if !clicked{for widget in gtk::Window::list_toplevels(){if let Ok(w)=widget.downcast::<gtk::Window>(){if w.title().as_deref()==Some("Persistent terminal sessions"){clicked=sessions::fixture(&w,label);}}}}reply(json!({"clicked":clicked}));},
    "close-rdp"=>{let r=rdp_views.borrow().get(q["rdp"].as_u64().unwrap_or(0)as usize).cloned();if let Some(r)=r{tabs.close_page(&tabs.page(&r.root));}reply(json!({"rdp_views":rdp_views.borrow().len()}));},
    "close-tab"=>{let i=q["tab"].as_i64().unwrap_or(0)as i32;if i>=0&&i<tabs.n_pages(){let page=tabs.nth_page(i);tabs.close_page(&page);}reply(json!({"tabs":tabs.n_pages(),"terminal_views":vv.borrow().len()}));},
    "tab-select"=>{let i=q["tab"].as_i64().unwrap_or(0)as i32;if i>=0&&i<tabs.n_pages(){tabs.set_selected_page(&tabs.nth_page(i));}reply(json!({"tabs":tabs.n_pages()}));},
    "workspace-close"=>{reply(json!({"closing":true}));window.close();},
    "switch"=>{v.show_terminal();let page=tabs.page(&v.root);tabs.set_selected_page(&page);if let Some(pane)=q["pane"].as_str(){*v.target.borrow_mut()=pane.into();v.command("pane.select",json!({"pane":pane}));}v.focus();if let Some(text)=q["text"].as_str(){for c in text.chars(){let key=unsafe{glib::translate::from_glib(gdk::ffi::gdk_unicode_to_keyval(c as u32))};v.key(key,gdk::ModifierType::empty());}}reply(json!({"queued":true}));},
    "rdp-open"=>{rdp_button.emit_clicked();if let Some(r)=rdp_views.borrow().last(){if let Some(port)=q["port"].as_u64().filter(|p|*p>0&&*p<=65535){r.fixture_connect(port as u16,&dir.join("cert.pem"));}else{r.fixture_frame();}}reply(json!({"tabs":tabs.n_pages(),"backend":"IronRDP","synthetic_frame":q["port"].is_null()}));},
    "rdp-inspect"|"rdp-switch"|"rdp-send"|"rdp-receive"|"rdp-policy"=>{if let Some(r)=rdp_views.borrow().get(q["rdp"].as_u64().unwrap_or(0)as usize){match q["action"].as_str().unwrap_or(""){"rdp-switch"=>{tabs.set_selected_page(&tabs.page(&r.root));r.focus();},"rdp-policy"=>r.fixture_policy(q["policy"].as_u64().unwrap_or(0)as u32),"rdp-send"=>r.fixture_clipboard(false),"rdp-receive"=>r.fixture_clipboard(true),_=>()}reply(r.fixture_inspect());}else{reply(json!({"error":"RDP tab missing"}));}},
    "rdp-display"=>{if let Some(v)=rdp_views.borrow().get(q["rdp"].as_u64().unwrap_or(0) as usize){if let(Some(w),Some(h))=(q["window_width"].as_i64(),q["window_height"].as_i64()){v.fixture_window(w.clamp(700,3200)as i32,h.clamp(500,2000)as i32);}
v.fixture_display(q["mode"].as_str().unwrap_or("fit"),q["zoom"].as_f64().unwrap_or(1.0),q["width"].as_u64().unwrap_or(0) as u16,q["height"].as_u64().unwrap_or(0) as u16);reply(v.fixture_inspect());}},
    "files-open"=>{match serde_json::from_value::<profiles::Profile>(q["profile"].clone()){Ok(profile) if profiles::endpoint(&profile.server,22).is_ok_and(|(host,_)|host=="127.0.0.1")=>{let panel=files::panel(profile);let page=tabs.append(&panel);page.set_title("Fixture · Files");tabs.set_selected_page(&page);reply(files::fixture(&panel,q["path"].as_str(),true));},_=>reply(json!({"error":"Loopback fixture profile required"}))}},
    "files-inspect"=>{if let Some(page)=tabs.selected_page(){if page.title().ends_with("Files"){if let Ok(root)=page.child().downcast::<gtk::Box>(){reply(files::fixture(&root,q["path"].as_str(),q["refresh"]==true));}}}},
    "profile-editor"=>reply(profiles::fixture_editor(&sidebar,&q)),
    "profiles"=>{let path=q["file"].as_str().filter(|s|!s.contains('/')&&!s.contains("..")).map(|s|dir.join(s));reply(profiles::fixture(&sidebar,path.as_deref(),q["search"].as_str(),q["favorite"]==true,q["open"]==true));},
    "command"=>{v.run_command(q["text"].as_str().unwrap_or(""));reply(json!({"queued":true}));},
    "divider-drag"=>{reply(v.fixture_root_drag());},
    "search"=>{v.search.set_text(q["text"].as_str().unwrap_or(""));reply(json!({"queued":true}));},
    "copy-state"=>{let b=v.copy.buffer();let text=v.selected();reply(json!({"selection":text,"visible":v.copy.is_mapped(),"search":v.search.text().as_str(),"contains_ready":b.text(&b.start_iter(),&b.end_iter(),true).contains("RUWAC"),"chars":b.char_count(),"cursor":b.iter_at_mark(&b.get_insert()).offset()}));},
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
    "inspect"=>reply(json!({"display":v.area.display().type_().name(),"active":window.is_active(),"tabs":tabs.n_pages(),"layout":v.frame.borrow().as_ref().map(|f|f["layout"].clone()),"zoom":v.frame.borrow().as_ref().map(|f|f["zoom"].clone()),"target":*v.target.borrow(),"drawn":vv.borrow().iter().map(|v|v.drawn.get()).collect::<Vec<_>>(),"panes":vv.borrow().iter().map(|v|v.frame.borrow().as_ref().and_then(|f|f["panes"].as_array()).map(|a|a.len()).unwrap_or(0)).collect::<Vec<_>>(),"ticks":tick.get(),"max_tick_gap_ms":max_gap.get(),"acknowledgements":v.worker.state.lock().unwrap().acknowledgements,"status":v.status.text().to_string()})),
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
