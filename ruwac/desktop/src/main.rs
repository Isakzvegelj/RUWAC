mod clipboard;
mod cliprdr;
mod credentials;
mod files;
mod profiles;
mod rdp;
mod sessions;
mod staging;
mod tiles;
mod view;
mod worker;
mod workspace;
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
    let workspace_warning = saved
        .as_ref()
        .err()
        .map(|e| format!("Workspace restore: {e}"));
    let selected = saved.as_ref().ok().and_then(|w| w.selected.clone());
    let restored_tabs = if sessions.is_empty() {
        saved
            .as_ref()
            .ok()
            .map(|w| w.tabs.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let restored_selected = saved.as_ref().ok().and_then(|w| w.selected_tab);
    if sessions.is_empty() {
        if let Ok(w) = saved.as_ref() {
            sessions.extend(
                w.sessions
                    .iter()
                    .filter(|id| {
                        available
                            .as_array()
                            .is_some_and(|rows| rows.iter().any(|r| r["id"] == id.as_str()))
                    })
                    .cloned(),
            );
        }
        if sessions.is_empty() {
            if let Some(id) = available
                .as_array()
                .and_then(|rows| rows.first())
                .and_then(|r| r["id"].as_str())
            {
                sessions.push(id.into());
            } else {
                sessions.push(
                    rpc.call("session.create", json!({"name":"Terminal"}))?["session"]
                        .as_str()
                        .ok_or("no session ID")?
                        .into(),
                );
            }
        }
    }
    let session_names: std::collections::HashMap<String, String> = available
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((r["id"].as_str()?.into(), r["name"].as_str()?.into())))
        .collect();
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
        .flags(if fixture.is_some() {
            gtk::gio::ApplicationFlags::NON_UNIQUE
        } else {
            gtk::gio::ApplicationFlags::FLAGS_NONE
        })
        .build();
    let views: Rc<RefCell<Vec<Rc<View>>>> = Rc::new(RefCell::new(Vec::new()));
    let all = views.clone();
    app.connect_activate(move|app|{
  if let Some(window)=app.active_window(){window.present();return;}
  let window=adw::ApplicationWindow::builder().application(app).title("RUWAC").default_width(1040).default_height(780).build();
  let box_=gtk::Box::new(gtk::Orientation::Vertical,0);let header=adw::HeaderBar::new();box_.append(&header);let tabs=adw::TabView::new();tabs.set_vexpand(true);let bar=adw::TabBar::builder().view(&tabs).autohide(false).build();box_.append(&bar);
  if let Some(message)=workspace_warning.as_ref(){box_.append(&gtk::Label::new(Some(message)));}
  let broker=Rc::new(RefCell::new(ruwac_clipboard::FocusBroker::default()));
  let last_focus:Rc<RefCell<Option<String>>>=Rc::new(RefCell::new(None));
  broker.borrow_mut().set_desktop_disabled(clipboard_disabled);
  let b=broker.clone();gtk::prelude::WidgetExt::display(&window).clipboard().connect_changed(move|_|b.borrow_mut().local_copy());
  let b=broker.clone();gtk::prelude::WidgetExt::display(&window).primary_clipboard().connect_changed(move|_|{let pending=b.borrow().last_transfer.as_ref().is_some_and(|t|t.scope==ruwac_clipboard::ClipScope::Primary&&t.state==ruwac_clipboard::TransferState::Streaming);if pending{b.borrow_mut().local_copy();}});
  let rdp_views:Rc<RefCell<Vec<Rc<rdp::RdpView>>>>=Rc::new(RefCell::new(Vec::new()));
  let file_views:Rc<RefCell<Vec<(gtk::Box,profiles::Profile)>>> = Rc::new(RefCell::new(Vec::new()));
  let placeholders:Rc<RefCell<Vec<(gtk::Widget,workspace::Layout)>>> = Rc::new(RefCell::new(Vec::new()));
  let rdp_button=gtk::Button::with_label("New RDP tab");header.pack_start(&rdp_button);let rv=rdp_views.clone();let t=tabs.clone();let b=broker.clone();rdp_button.connect_clicked(move|_|{let v=rdp::RdpView::new(b.clone());let p=tiles::append(&t,&v.root);p.set_title("RDP · embedded IronRDP");rv.borrow_mut().push(v.clone());t.set_selected_page(&p);v.focus();});
  let rv=rdp_views.clone();let vv=all.clone();let b=broker.clone();tabs.connect_selected_page_notify(move|t|{if let Some(page)=t.selected_page(){for v in rv.borrow().iter(){if !tiles::contains(&page.child(),&v.root){v.inactive();}}if let Some(v)=vv.borrow().iter().find(|v|tiles::contains(&page.child(),&v.root)){v.focus();}else if let Some(v)=rv.borrow().iter().find(|v|tiles::contains(&page.child(),&v.root)){v.focus();}else{b.borrow_mut().focus(None);}}});
  let rv=rdp_views.clone();tabs.connect_close_page(move|_,page|{let mut views=rv.borrow_mut();for v in views.iter(){if tiles::contains(&page.child(),&v.root){v.close();}}views.retain(|v|!tiles::contains(&page.child(),&v.root));glib::Propagation::Proceed});
  let rv=rdp_views.clone();window.connect_close_request(move|_|{for v in rv.borrow().iter(){v.close();}glib::Propagation::Proceed});
  let owned_images:Rc<RefCell<Vec<PathBuf>>>=Rc::new(RefCell::new(Vec::new()));
  let clear=gtk::Button::with_label("Clear message");header.pack_end(&clear);let vc=all.clone();let tc=tabs.clone();clear.connect_clicked(move|_|{if let Some(page)=tc.selected_page(){for v in vc.borrow().iter(){if tiles::contains(&page.child(),&v.root){*v.warning.borrow_mut()=None;v.status.set_text("Delivery warning acknowledged");}}}});
  let image_button=gtk::Button::with_label("Stage clipboard PNG");header.pack_end(&image_button);
  let vb=all.clone();let tb=tabs.clone();let sb=stage.clone();let owned=owned_images.clone();let last=last_focus.clone();
  image_button.connect_clicked(move|_|{let Some(page)=tb.selected_page()else{return;};let Some(v)=vb.borrow().iter().find(|v|tiles::contains(&page.child(),&v.root)&&last.borrow().as_deref()==Some(v.session.as_str())).cloned()else{return;};let Some(dir)=sb.clone()else{v.status.set_text("Private staging directory is unavailable");return;};v.focus();let epoch=v.broker.borrow().epoch();let weak=Rc::downgrade(&v);let owned=owned.clone();clipboard::ingest_png(&v.area.display().clipboard(),dir,v.broker.clone(),epoch,ruwac_clipboard::MAX_IMAGE_BYTES,Duration::ZERO,move|r|if let Some(v)=weak.upgrade(){match r{Ok(r)=>{owned.borrow_mut().push(r.path);v.warn(&format!("PNG staged: {}×{}, {} bytes; no agent attachment",r.width,r.height,r.bytes));},Err(e)=>v.warn(&e)}});});
  for session in &sessions{sessions::append(&tabs,&all,&socket,&broker,session,session_names.get(session).map(String::as_str).unwrap_or("Terminal"));}
  if let Some(id)=selected.as_ref(){if let Some(v)=all.borrow().iter().find(|v|&v.session==id){if let Some(page)=tiles::page(&tabs,&v.root){tabs.set_selected_page(&page);}}}
  header.pack_start(&sessions::controls(tabs.clone(),all.clone(),socket.clone(),broker.clone()));
  header.pack_start(&tiles::controls(&tabs));
  let(rv,vv,b,last,h)=(rdp_views.clone(),all.clone(),broker.clone(),last_focus.clone(),header.clone());window.connect_focus_widget_notify(move|w|{let focus=gtk::prelude::GtkWindowExt::focus(w);let mut owner=false;for r in rv.borrow().iter(){if focus.as_ref().is_some_and(|f|r.owns_focus(f)){r.activate_clipboard();*last.borrow_mut()=b.borrow().active().map(str::to_owned);owner=true;}else{r.inactive();}}if !owner{if let Some(v)=vv.borrow().iter().find(|v|focus.as_ref().is_some_and(|f|tiles::contains(&v.root,f))){*last.borrow_mut()=Some(v.session.clone());if b.borrow().active()!=Some(v.session.as_str()){b.borrow_mut().focus(Some(&v.session));}}else{if !focus.as_ref().is_some_and(|f|tiles::contains(&h,f)){*last.borrow_mut()=None;}if b.borrow().active().is_some(){b.borrow_mut().focus(None);}}}});

  let rv=rdp_views.clone();let t=tabs.clone();let b=broker.clone();let(sock,views)=(socket.clone(),all.clone());let fv=file_views.clone();
  let (sidebar,search,restore_profile)=profiles::sidebar(move|profile,persist,lease|{if profile.protocol=="RDP"{let v=rdp::RdpView::new(b.clone());v.configure(&profile);v.set_preferences_sink(persist);lease.bind(&v.root);let page=tiles::append(&t,&v.root);page.set_title(&format!("{} / {} · {}",profile.group,profile.name,profile.protocol));rv.borrow_mut().push(v.clone());t.set_selected_page(&page);v.focus();v.connect_profile();}else{let file_panel=files::panel(profile.clone());fv.borrow_mut().push((file_panel.clone(),profile.clone()));lease.fork().bind(&file_panel);let file_page=tiles::append(&t,&file_panel);file_page.set_title(&format!("{} / {} · Files",profile.group,profile.name));let socket=sock.clone();let name=profile.name.clone();let argv=profiles::ssh_argv(&profile);let(tx,rx)=std::sync::mpsc::sync_channel(1);std::thread::spawn(move||{let result=(||->Result<String,String>{let mut client=worker::Rpc::connect(&socket).map_err(|e|e.to_string())?;let reply=client.call("session.create",json!({"name":name,"argv":argv?})).map_err(|e|e.to_string())?;reply["session"].as_str().map(str::to_owned).ok_or("SSH session creation failed".into())})();let _=tx.send(result);});let(t,b,views,socket)=(t.clone(),b.clone(),views.clone(),sock.clone());let mut pending_lease=Some(lease);glib::timeout_add_local(Duration::from_millis(10),move||{match rx.try_recv(){Ok(Ok(session))=>{let v=View::new(socket.clone(),session,b.clone());if let Some(lease)=pending_lease.take(){lease.bind(&v.root);}let page=tiles::append(&t,&v.root);page.set_title(&format!("{} / {} · {}",profile.group,profile.name,profile.protocol));views.borrow_mut().push(v.clone());t.set_selected_page(&page);v.focus();glib::ControlFlow::Break},Ok(Err(_))|Err(std::sync::mpsc::TryRecvError::Disconnected)=>glib::ControlFlow::Break,Err(std::sync::mpsc::TryRecvError::Empty)=>glib::ControlFlow::Continue}});}});
  if !restored_tabs.is_empty(){
    tiles::restore(&tabs,&restored_tabs,|layout|match layout{
      workspace::Layout::Terminal{session}=>{
        let view=all.borrow().iter().find(|v|&v.session==session).cloned();
        if let Some(v)=view{if let Some(page)=tiles::page(&tabs,&v.root){if let Ok(body)=page.child().downcast::<gtk::Box>(){body.remove(&v.root);tabs.close_page(&page);}}v.root.clone().upcast()}
        else{let label=gtk::Label::new(Some("This terminal session ended. Use New terminal to start another; the old process was not resurrected."));label.set_wrap(true);let root:gtk::Widget=label.upcast();placeholders.borrow_mut().push((root.clone(),layout.clone()));root}
      },
      workspace::Layout::Rdp{profile}=>{
        let v=rdp::RdpView::new(broker.clone());
        if let Some(p)=profile{
          if p.id.is_empty(){v.configure(p);}
          else if let Some((current,sink,lease))=restore_profile(&p.id).filter(|(p,_,_)|p.protocol=="RDP"){v.configure(&current);v.set_preferences_sink(sink);lease.bind(&v.root);}
          else{let label=gtk::Label::new(Some("This RDP profile was removed. Reopen an authorized profile from Machines."));label.set_wrap(true);let root:gtk::Widget=label.upcast();placeholders.borrow_mut().push((root.clone(),layout.clone()));return root;}
        }
        rdp_views.borrow_mut().push(v.clone());v.root.clone().upcast()
      },
      workspace::Layout::Files{profile,directory}=>{
        if let Some((current,_,lease))=restore_profile(&profile.id).filter(|(p,_,_)|p.protocol=="SSH"){let root=files::panel(current.clone());files::directory(&root,Some(directory));lease.bind(&root);file_views.borrow_mut().push((root.clone(),current));root.upcast()}
        else{let label=gtk::Label::new(Some("This file profile was removed. Reopen an authorized SSH profile from Machines."));label.set_wrap(true);let root:gtk::Widget=label.upcast();placeholders.borrow_mut().push((root.clone(),layout.clone()));root}
      },
      workspace::Layout::Split{..}=>unreachable!(),
    });
    if let Some(index)=restored_selected{let offset=tabs.n_pages()-restored_tabs.len()as i32;let i=offset+index as i32;if i>=0&&i<tabs.n_pages(){tabs.set_selected_page(&tabs.nth_page(i));}}
  }
  let(fv,ph)=(file_views.clone(),placeholders.clone());tabs.connect_close_page(move|_,p|{fv.borrow_mut().retain(|(v,_)|!tiles::contains(&p.child(),v));ph.borrow_mut().retain(|(v,_)|!tiles::contains(&p.child(),v));glib::Propagation::Proceed});
  let body=gtk::Box::new(gtk::Orientation::Horizontal,0);let reveal=gtk::Revealer::builder().child(&sidebar).reveal_child(true).build();body.append(&reveal);body.append(&tabs);tabs.set_hexpand(true);box_.append(&body);let toggle=gtk::Button::with_label("Machines");header.pack_start(&toggle);let r=reveal.clone();toggle.connect_clicked(move|_|r.set_reveal_child(!r.reveals_child()));let keys=gtk::EventControllerKey::new();keys.set_propagation_phase(gtk::PropagationPhase::Capture);keys.connect_key_pressed(move|_,key,_,mods|{if key==gdk::Key::k&&mods.contains(gdk::ModifierType::CONTROL_MASK){reveal.set_reveal_child(true);search.grab_focus();glib::Propagation::Stop}else{glib::Propagation::Proceed}});window.add_controller(keys);window.set_content(Some(&box_));

  let vv=all.clone();tabs.connect_close_page(move|_,page|{let mut views=vv.borrow_mut();for v in views.iter(){if tiles::contains(&page.child(),&v.root){v.close();}}views.retain(|v|!tiles::contains(&page.child(),&v.root));glib::Propagation::Proceed});
  let(vv,rv,fv,ph,saved_tabs,saved_socket)=(all.clone(),rdp_views.clone(),file_views.clone(),placeholders.clone(),tabs.clone(),socket.clone());
  let recorder:workspace::Recorder=Rc::new(move||{
    let open=vv.borrow();let mut ids=Vec::new();for i in 0..saved_tabs.n_pages(){let page=saved_tabs.nth_page(i);for v in open.iter().filter(|v|tiles::contains(&page.child(),&v.root)){ids.push(v.session.clone());}}for (_,layout)in ph.borrow().iter(){if let workspace::Layout::Terminal{session}=layout{if !ids.contains(session){ids.push(session.clone());}}}let selected=saved_tabs.selected_page().and_then(|p|open.iter().find(|v|tiles::contains(&p.child(),&v.root)).map(|v|v.session.clone()));let layouts=tiles::snapshot(&saved_tabs,|root|{
      if let Some(v)=open.iter().find(|v|v.root.upcast_ref::<gtk::Widget>()==root){return Some(workspace::Layout::Terminal{session:v.session.clone()});}
      if let Some(v)=rv.borrow().iter().find(|v|v.root.upcast_ref::<gtk::Widget>()==root){return Some(workspace::Layout::Rdp{profile:v.configuration()});}
      if let Some((view,profile))=fv.borrow().iter().find(|(v,_)|v.upcast_ref::<gtk::Widget>()==root){return Some(workspace::Layout::Files{profile:profile.clone(),directory:files::directory(view,None)});}
      ph.borrow().iter().find(|(w,_)|w==root).map(|(_,l)|l.clone())
    });

    let layouts=layouts?;
    let selected_tab=saved_tabs.selected_page().map(|p|saved_tabs.page_position(&p)as usize);
    Ok(workspace::Workspace{schema:2,socket:saved_socket.clone(),sessions:ids,selected,tabs:layouts,selected_tab})
  });
  workspace::watch(&window,recorder.clone());
  let(owned,vv)=(owned_images.clone(),all.clone());window.connect_close_request(move|_|{
    match recorder(){Ok(w)=>{if let Err(e)=workspace::save(&w){eprintln!("Workspace save failed: {e}");}},Err(e)=>eprintln!("Workspace save failed: {e}")};
    for v in vv.borrow().iter(){v.close();}for path in owned.borrow_mut().drain(..){let _=std::fs::remove_file(path);}glib::Propagation::Proceed
  });window.present();if let Some(page)=tabs.selected_page(){for v in all.borrow().iter(){if tiles::contains(&page.child(),&v.root){v.focus();}}}
  let vv=all.clone();let tabs=tabs.clone();let app=app.clone();let fixture=fixture.clone();let staging=stage.clone();let owned=owned_images.clone();let tick=Rc::new(Cell::new(0u64));let last=Rc::new(Cell::new(Instant::now()));let max_gap=Rc::new(Cell::new(0u128));let sequence=Rc::new(Cell::new(0u64));
  glib::timeout_add_local(Duration::from_millis(16),move||{
   tick.set(tick.get()+1);let now=Instant::now();max_gap.set(max_gap.get().max(now.duration_since(last.replace(now)).as_millis()));for v in vv.borrow().iter(){v.update();}
   if let Some(dir)=fixture.as_ref(){let request=dir.join("gui-request.json");if let Ok(bytes)=std::fs::read(&request){if let Ok(q)=serde_json::from_slice::<Value>(&bytes){let seq=q["seq"].as_u64().unwrap_or(0);if seq>sequence.get(){sequence.set(seq);let index=q["view"].as_u64().unwrap_or(0)as usize;let view=vv.borrow().get(index).cloned();let reply_dir=dir.clone();let reply=move|result:Value|{let path=reply_dir.join("gui-response.json");let temp=reply_dir.join("gui-response.tmp");let _=std::fs::write(&temp,json!({"seq":seq,"result":result}).to_string());let _=std::fs::rename(temp,path);};if let Some(v)=view{match q["action"].as_str().unwrap_or(""){
    "new-terminal"=>reply(json!({"clicked":sessions::fixture(&window,"New terminal")})),
    "sessions-open"=>{let label=q["session"].as_str().unwrap_or("Sessions…");let mut clicked=sessions::fixture(&window,label);if !clicked{for widget in gtk::Window::list_toplevels(){if let Ok(w)=widget.downcast::<gtk::Window>(){if w.title().as_deref()==Some("Persistent terminal sessions"){clicked=sessions::fixture(&w,label);}}}}reply(json!({"clicked":clicked}));},
    "close-rdp"=>{let r=rdp_views.borrow().get(q["rdp"].as_u64().unwrap_or(0)as usize).cloned();if let Some(r)=r{if let Some(page)=tiles::page(&tabs,&r.root){tabs.close_page(&page);}}reply(json!({"rdp_views":rdp_views.borrow().len()}));},
    "close-tab"=>{let i=q["tab"].as_i64().unwrap_or(0)as i32;if i>=0&&i<tabs.n_pages(){let page=tabs.nth_page(i);tabs.close_page(&page);}reply(json!({"tabs":tabs.n_pages(),"terminal_views":vv.borrow().len()}));},
    "tab-select"=>{let i=q["tab"].as_i64().unwrap_or(0)as i32;if i>=0&&i<tabs.n_pages(){tabs.set_selected_page(&tabs.nth_page(i));}reply(json!({"tabs":tabs.n_pages()}));},
    "focus-view"=>{if let Some(page)=tiles::page(&tabs,&v.root){tabs.set_selected_page(&page);}v.focus();reply(json!({"focused":v.session}));},
    "stage-png"=>{image_button.grab_focus();image_button.emit_clicked();reply(json!({"queued":true,"native_toolbar":true}));},
    "mixed-join"=>{reply(match tiles::join(&tabs,q["tab"].as_i64().unwrap_or(-1)as i32,q["vertical"]==true){Ok(())=>tiles::inspect(&tabs),Err(e)=>json!({"error":e})});},
    "mixed-separate"=>{reply(match tiles::separate(&tabs){Ok(_)=>tiles::inspect(&tabs),Err(e)=>json!({"error":e})});},
    "mixed-inspect"=>reply(tiles::inspect(&tabs)),
    "mixed-focus"=>{if q["kind"]=="rdp"{if let Some(r)=rdp_views.borrow().get(q["index"].as_u64().unwrap_or(0)as usize){r.focus();}}else if let Some(v)=vv.borrow().get(q["index"].as_u64().unwrap_or(0)as usize){v.focus();}reply(json!({"owner":broker.borrow().active()}));},
    "workspace-close"=>{reply(json!({"closing":true}));window.close();},
    "switch"=>{v.show_terminal();if let Some(page)=tiles::page(&tabs,&v.root){tabs.set_selected_page(&page);}if let Some(pane)=q["pane"].as_str(){*v.target.borrow_mut()=pane.into();v.command("pane.select",json!({"pane":pane}));}v.focus();if let Some(text)=q["text"].as_str(){for c in text.chars(){let key=unsafe{glib::translate::from_glib(gdk::ffi::gdk_unicode_to_keyval(c as u32))};v.key(key,gdk::ModifierType::empty());}}reply(json!({"queued":true}));},
    "rdp-open"=>{rdp_button.emit_clicked();if let Some(r)=rdp_views.borrow().last(){if let Some(port)=q["port"].as_u64().filter(|p|*p>0&&*p<=65535){r.fixture_connect(port as u16,&dir.join("cert.pem"));}else{r.fixture_frame();}}reply(json!({"tabs":tabs.n_pages(),"backend":"IronRDP","synthetic_frame":q["port"].is_null()}));},
    "rdp-inspect"|"rdp-switch"|"rdp-send"|"rdp-receive"|"rdp-policy"=>{if let Some(r)=rdp_views.borrow().get(q["rdp"].as_u64().unwrap_or(0)as usize){match q["action"].as_str().unwrap_or(""){"rdp-switch"=>{if let Some(page)=tiles::page(&tabs,&r.root){tabs.set_selected_page(&page);}r.focus();},"rdp-policy"=>r.fixture_policy(q["policy"].as_u64().unwrap_or(0)as u32),"rdp-send"=>r.fixture_clipboard(false),"rdp-receive"=>r.fixture_clipboard(true),_=>()}reply(r.fixture_inspect());}else{reply(json!({"error":"RDP tab missing"}));}},
    "rdp-display"=>{if let Some(v)=rdp_views.borrow().get(q["rdp"].as_u64().unwrap_or(0) as usize){if let(Some(w),Some(h))=(q["window_width"].as_i64(),q["window_height"].as_i64()){v.fixture_window(w.clamp(700,3200)as i32,h.clamp(500,2000)as i32);}
v.fixture_display(q["mode"].as_str().unwrap_or("fit"),q["zoom"].as_f64().unwrap_or(1.0),q["width"].as_u64().unwrap_or(0) as u16,q["height"].as_u64().unwrap_or(0) as u16);reply(v.fixture_inspect());}},
    "files-open"=>{match serde_json::from_value::<profiles::Profile>(q["profile"].clone()){Ok(profile) if profiles::endpoint(&profile.server,22).is_ok_and(|(host,_)|host=="127.0.0.1")=>{let panel=files::panel(profile.clone());file_views.borrow_mut().push((panel.clone(),profile));let page=tiles::append(&tabs,&panel);page.set_title("Fixture · Files");tabs.set_selected_page(&page);reply(files::fixture(&panel,q["path"].as_str(),true));},_=>reply(json!({"error":"Loopback fixture profile required"}))}},
    "files-directory"=>{if let Some((root,_))=file_views.borrow().get(q["index"].as_u64().unwrap_or(0)as usize){reply(json!({"directory":files::directory(root,q["path"].as_str())}));}else{reply(json!({"error":"Files view missing"}));}},
    "files-inspect"=>{if let Some(page)=tabs.selected_page(){if page.title().ends_with("Files"){if let Some(root)=page.child().first_child().and_then(|c|c.downcast::<gtk::Box>().ok()){reply(files::fixture(&root,q["path"].as_str(),q["refresh"]==true));}}}},
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
