//! Native process monitor for existing daemon-owned PTYs; no agent runtime or inference.
use crate::{clipboard::Broker, sessions, view::View, worker::Rpc};
use adw::prelude::*;
use gtk::{gio, glib};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    time::Duration,
};
thread_local! {static SEARCH_PROBES:RefCell<Vec<glib::WeakRef<gtk::SearchEntry>>>=const{RefCell::new(Vec::new())};}
fn retained_search_widgets() -> usize {
    SEARCH_PROBES.with(|p| p.borrow().iter().filter(|w| w.upgrade().is_some()).count())
}
fn age(v: &Value) -> String {
    v.as_u64()
        .map(|n| format!("{}s", n / 1000))
        .unwrap_or_else(|| "unknown".into())
}
fn phase(v: &Value) -> String {
    match v["phase"].as_str() {
        Some("running") => "Running".into(),
        Some("exited") => format!(
            "Exited ({})",
            v["exit_code"]
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unknown".into())
        ),
        Some("restored_history") => "Restored history · no process".into(),
        _ => "Ended · exit status pending".into(),
    }
}
pub fn control(
    app: &adw::Application,
    tabs: adw::TabView,
    views: Rc<RefCell<Vec<Rc<View>>>>,
    socket: PathBuf,
    broker: Broker,
) -> gtk::Button {
    let button = gtk::Button::with_label("Jobs…");
    button.set_tooltip_text(Some("Search processes and jump to persistent terminals"));
    let app = app.clone();
    button.connect_clicked(move|button| {
        broker.borrow_mut().focus(None);
        if let Some(window)=gtk::Window::list_toplevels().into_iter().find(|w|w.widget_name()=="jobs-dialog").and_then(|w|w.downcast::<gtk::Window>().ok()).filter(|w|w.is_visible()){window.present();return;}
        let dialog=gtk::Dialog::builder().title("Terminal jobs").default_width(740).default_height(580).build();dialog.set_widget_name("jobs-dialog");
        dialog.set_transient_for(button.root().and_then(|w|w.downcast::<gtk::Window>().ok()).as_ref());dialog.add_button("Close",gtk::ResponseType::Close);dialog.connect_response(|d,_|d.close());
        let body=dialog.content_area();body.set_spacing(6);
        let hint=gtk::Label::new(Some("Process state is observed; quiet output does not mean finished or awaiting input. Arguments and environment are excluded."));hint.set_wrap(true);body.append(&hint);
        let search=gtk::SearchEntry::new();search.set_widget_name("jobs-search");search.set_placeholder_text(Some("Session, command, host or working directory"));body.append(&search);SEARCH_PROBES.with(|p|{let mut p=p.borrow_mut();p.retain(|w|w.upgrade().is_some());if p.len()<128{p.push(search.downgrade());}});
        let notify=gtk::CheckButton::with_label("Notify on observed process exits while this monitor is open");notify.set_widget_name("jobs-notify");body.append(&notify);
        let list=gtk::Box::new(gtk::Orientation::Vertical,6);let scroll=gtk::ScrolledWindow::builder().vexpand(true).child(&list).build();body.append(&scroll);
        let pager=gtk::Box::new(gtk::Orientation::Horizontal,6);let previous=gtk::Button::from_icon_name("go-previous-symbolic");previous.set_widget_name("jobs-previous");previous.set_tooltip_text(Some("Previous jobs page"));let label=gtk::Label::new(None);label.set_widget_name("jobs-page");label.set_hexpand(true);let next=gtk::Button::from_icon_name("go-next-symbolic");next.set_widget_name("jobs-next");next.set_tooltip_text(Some("Next jobs page"));pager.append(&previous);pager.append(&label);pager.append(&next);body.append(&pager);
        let status=gtk::Label::new(Some("Loading…"));status.set_widget_name("jobs-status");status.set_wrap(true);body.append(&status);
        let rows=Rc::new(RefCell::new(Vec::<Value>::new()));let page=Rc::new(Cell::new(0usize));
        let redraw:Rc<dyn Fn()>={
            let(list,search,rows,page,label,previous,next,status,d,t,v,s,b)=(list.clone(),search.clone(),rows.clone(),page.clone(),label.clone(),previous.clone(),next.clone(),status.clone(),dialog.downgrade(),tabs.clone(),views.clone(),socket.clone(),broker.clone());
            Rc::new(move|| {
                let focused=d.upgrade().and_then(|d|gtk::prelude::GtkWindowExt::focus(&d)).filter(|w|w.widget_name()=="job-jump").and_then(|w|w.tooltip_text());
                while let Some(child)=list.first_child(){list.remove(&child);}
                let query=search.text().to_lowercase();let rows=rows.borrow();let matches:Vec<_>=rows.iter().filter(|r|query.is_empty()||["id","session_name","window_name","command","cwd","host"].iter().any(|k|r[*k].as_str().is_some_and(|s|s.to_lowercase().contains(&query)))||r["foreground"]["command"].as_str().is_some_and(|s|s.to_lowercase().contains(&query))).collect();
                let total=matches.len();page.set(page.get().min(total.saturating_sub(1)/50));let start=page.get()*50;label.set_label(&format!("{}–{} of {}",if total==0{0}else{start+1},(start+50).min(total),total));previous.set_sensitive(page.get()>0);next.set_sensitive(start+50<total);
                for row in matches.into_iter().skip(start).take(50) {
                    let item=gtk::Box::new(gtk::Orientation::Vertical,2);item.set_widget_name("job-row");
                    let pid=row["id"].as_str().unwrap_or("");let command=row["foreground"]["command"].as_str().or(row["command"].as_str()).unwrap_or("Unknown command");
                    let title=format!("{} / {} · {}",row["session_name"].as_str().unwrap_or("Terminal"),row["window_name"].as_str().unwrap_or("Window"),command);
                    let jump=gtk::Button::with_label(&title);jump.set_widget_name("job-jump");jump.set_tooltip_text(Some(pid));jump.set_hexpand(true);if let Some(label)=jump.child().and_then(|w|w.downcast::<gtk::Label>().ok()){label.set_ellipsize(gtk::pango::EllipsizeMode::End);label.set_max_width_chars(60);}
                    item.append(&jump);let summary=gtk::Label::new(Some(&format!("{} · PID {} · age {} · last output {} ago",phase(row),row["foreground"]["pid"].as_i64().or(row["pid"].as_i64()).map(|n|n.to_string()).unwrap_or_else(||"none".into()),age(&row["age_ms"]),age(&row["last_output_age_ms"]))));summary.set_widget_name("job-summary");summary.set_xalign(0.0);item.append(&summary);
                    let location=format!("{} · {}",row["host"].as_str().unwrap_or("unknown host"),row["foreground"]["cwd"].as_str().or(row["cwd"].as_str()).unwrap_or("working directory unavailable"));let location=gtk::Label::new(Some(&location));location.set_xalign(0.0);location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);location.set_selectable(true);item.append(&location);list.append(&item);if focused.as_deref()==Some(pid){jump.grab_focus();}
                    let(t,v,s,b,status,d,pane)=(t.clone(),v.clone(),s.clone(),b.clone(),status.clone(),d.clone(),pid.to_owned());
                    jump.connect_clicked(move|button| {
                        button.set_sensitive(false);let(tx,rx)=std::sync::mpsc::sync_channel(1);let socket=s.clone();let pane=pane.clone();
                        std::thread::spawn(move|| {let result=(||{let mut rpc=Rpc::connect(&socket)?;let job=rpc.call("job.query",json!({"pane":pane}))?;rpc.call("window.select",json!({"window":job["window"]}))?;rpc.call("pane.select",json!({"pane":pane}))?;Ok::<_,std::io::Error>(job)})();let _=tx.send(result);});
                        let(t,v,s,b,status,d)=(t.clone(),v.clone(),s.clone(),b.clone(),status.clone(),d.clone());
                        glib::timeout_add_local(Duration::from_millis(20),move||{let Some(d)=d.upgrade().filter(|d|d.is_visible())else{return glib::ControlFlow::Break;};match rx.try_recv(){Ok(Ok(row))=>{sessions::append(&t,&v,&s,&b,row["session"].as_str().unwrap(),row["session_name"].as_str().unwrap_or("Terminal"));d.close();if let Some(parent)=d.transient_for(){parent.present();}glib::ControlFlow::Break},Ok(Err(_))=>{status.set_text("Job target closed or unavailable. Refresh and select its stable pane ID again.");glib::ControlFlow::Break},Err(std::sync::mpsc::TryRecvError::Empty)=>glib::ControlFlow::Continue,Err(_)=>{status.set_text("Job jump failed");glib::ControlFlow::Break}}});
                    });
                }
            })
        };
        let(r,p)=(Rc::downgrade(&redraw),page.clone());search.connect_search_changed(move|_|{p.set(0);if let Some(r)=r.upgrade(){r();}});let(r,p)=(Rc::downgrade(&redraw),page.clone());previous.connect_clicked(move|_|{p.set(p.get().saturating_sub(1));if let Some(r)=r.upgrade(){r();}});let(r,p)=(Rc::downgrade(&redraw),page.clone());next.connect_clicked(move|_|{p.set(p.get()+1);if let Some(r)=r.upgrade(){r();}});
        let(tx,rx)=std::sync::mpsc::sync_channel(1);let pending=Rc::new(Cell::new(false));let mut known=HashMap::<String,String>::new();let mut tick=0usize;let socket=socket.clone();let app=app.clone();let d=dialog.downgrade();dialog.present();
        glib::timeout_add_local(Duration::from_millis(50),move||{
            let Some(_dialog)=d.upgrade().filter(|d|d.is_visible())else{return glib::ControlFlow::Break;};
            if let Ok(result)=rx.try_recv(){pending.set(false);match result {
                Ok(value)=>{let new:Vec<Value>=value;for row in &new {let id=row["id"].as_str().unwrap_or("").to_owned();let phase=row["phase"].as_str().unwrap_or("").to_owned();if notify.is_active()&&phase=="exited"&&known.get(&id).is_some_and(|old|old=="running") {let notification=gio::Notification::new("Terminal job exited");notification.set_body(Some(&format!("{} · {}",row["session_name"].as_str().unwrap_or("Terminal"),super::jobs::phase(row))));app.send_notification(Some(&format!("job-{id}")),&notification);}known.insert(id,phase);}
                    known.retain(|id,_|new.iter().any(|r|r["id"]==*id));*rows.borrow_mut()=new;status.set_text("Live daemon process metadata · no structured agent-state inference");redraw();},
                Err(_)=>{status.set_text("Job metadata unavailable from this daemon. Existing terminal sessions remain usable.");}
            }}
            if tick%10==0&&!pending.get(){pending.set(true);let(socket,tx)=(socket.clone(),tx.clone());std::thread::spawn(move||{let result=(||{let mut rpc=Rpc::connect(&socket)?;let rows=fetch_jobs(&mut rpc)?;Ok::<_,std::io::Error>(rows)})();let _=tx.send(result);});}tick=tick.wrapping_add(1);glib::ControlFlow::Continue
        });
    });
    button
}
// Read every byte-bounded page before publishing a native snapshot. Older
// daemons may omit membership; strict cursor/count/identity guards still apply.
fn fetch_jobs(rpc: &mut Rpc) -> std::io::Result<Vec<Value>> {
    let invalid = || std::io::Error::other("Job membership or cursor changed; refresh again");
    let mut rows = Vec::new();
    let mut offset = 0u64;
    let mut expected = None;
    let mut membership: Option<Value> = None;
    let mut ids = std::collections::HashSet::new();
    for _ in 0..128 {
        let page = rpc.call("job.list", json!({"limit":100,"offset":offset}))?;
        let total = page["total"]
            .as_u64()
            .filter(|n| *n <= 128)
            .ok_or_else(invalid)?;
        if expected.is_some_and(|n| n != total) || page["offset"].as_u64() != Some(offset) {
            return Err(invalid());
        }
        expected = Some(total);
        if let Some(token) = page.get("membership") {
            if membership.as_ref().is_some_and(|old| old != token) {
                return Err(invalid());
            }
            membership = Some(token.clone());
        }
        let items = page["jobs"]
            .as_array()
            .filter(|v| v.len() <= 100)
            .ok_or_else(invalid)?;
        for item in items {
            let id = item["id"]
                .as_str()
                .filter(|id| ruwac_protocol::valid_id(id, "pane"))
                .ok_or_else(invalid)?;
            if !ids.insert(id.to_owned()) {
                return Err(invalid());
            }
            rows.push(item.clone());
        }
        if rows.len() > 128 {
            return Err(invalid());
        }
        match page.get("next_offset") {
            Some(Value::Null) => {
                if rows.len() as u64 != total {
                    return Err(invalid());
                }
                return Ok(rows);
            }
            Some(next) => {
                let next = next.as_u64().ok_or_else(invalid)?;
                if next <= offset || next != offset + items.len() as u64 || next >= total {
                    return Err(invalid());
                }
                offset = next;
            }
            None => return Err(invalid()),
        }
    }
    Err(invalid())
}
pub fn fixture(q: &Value) -> Value {
    fn find(w: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
        if w.widget_name() == name {
            return Some(w.clone());
        }
        let mut c = w.first_child();
        while let Some(w) = c {
            c = w.next_sibling();
            if let Some(r) = find(&w, name) {
                return Some(r);
            }
        }
        None
    }
    let Some(dialog) = gtk::Window::list_toplevels()
        .into_iter()
        .find(|w| w.widget_name() == "jobs-dialog")
        .and_then(|w| w.downcast::<gtk::Dialog>().ok())
        .filter(|w| w.is_visible())
    else {
        return json!({"open":false,"retained_search_widgets":retained_search_widgets()});
    };
    let root = dialog.upcast_ref::<gtk::Widget>();
    if let Some(query) = q["query"].as_str() {
        if let Some(search) =
            find(root, "jobs-search").and_then(|w| w.downcast::<gtk::SearchEntry>().ok())
        {
            search.set_text(query);
        }
    }
    if let Some(enabled) = q["notify"].as_bool() {
        if let Some(check) =
            find(root, "jobs-notify").and_then(|w| w.downcast::<gtk::CheckButton>().ok())
        {
            check.set_active(enabled);
        }
    }
    fn collect(w: &gtk::Widget, out: &mut Vec<Value>, jump: Option<&str>) {
        if w.widget_name() == "job-row" {
            if let Some(button) = w
                .first_child()
                .and_then(|w| w.downcast::<gtk::Button>().ok())
            {
                let pane = button.tooltip_text().unwrap_or_default();
                let summary = button
                    .next_sibling()
                    .and_then(|w| w.downcast::<gtk::Label>().ok())
                    .map(|l| l.text().to_string());
                out.push(json!({"pane":pane.as_str(),"label":button.label().as_deref(),"summary":summary}));
                if jump == Some(pane.as_str()) {
                    button.emit_clicked();
                }
            }
        }
        let mut c = w.first_child();
        while let Some(w) = c {
            c = w.next_sibling();
            collect(&w, out, jump);
        }
    }
    if let Some(direction) = q["page_action"].as_str() {
        let name = match direction {
            "next" => "jobs-next",
            "previous" => "jobs-previous",
            _ => "",
        };
        if let Some(button) = find(root, name)
            .and_then(|w| w.downcast::<gtk::Button>().ok())
            .filter(|b| b.is_sensitive())
        {
            button.emit_clicked();
        }
    }
    let page_text = find(root, "jobs-page")
        .and_then(|w| w.downcast::<gtk::Label>().ok())
        .map(|w| w.text().to_string());
    let mut rows = Vec::new();
    collect(root, &mut rows, q["jump"].as_str());
    let status = find(root, "jobs-status")
        .and_then(|w| w.downcast::<gtk::Label>().ok())
        .map(|l| l.text().to_string());
    if q["close"] == true {
        dialog.close();
    }
    json!({"open":true,"rows":rows,"page_text":page_text,"status":status,"retained_search_widgets":retained_search_widgets()})
}

#[cfg(test)]
mod paging_tests {
    use super::*;
    use std::{
        fs,
        os::unix::{fs::PermissionsExt, net::UnixListener},
    };
    fn peer(pages: Vec<Value>) -> std::io::Result<Vec<Value>> {
        let root = std::env::temp_dir().join(ruwac_protocol::id("job-paging"));
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let socket = root.join("control.sock");
        let listener = UnixListener::bind(&socket)?;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            for page in pages {
                let Ok(request) = ruwac_protocol::read_frame(&mut stream) else {
                    break;
                };
                if ruwac_protocol::write_frame(
                    &mut stream,
                    &json!({"version":0,"id":request["id"],"result":page}),
                )
                .is_err()
                {
                    break;
                }
            }
        });
        let result = fetch_jobs(&mut Rpc::connect(&socket)?);
        server.join().unwrap();
        fs::remove_file(socket)?;
        fs::remove_dir(root)?;
        result
    }
    fn page(index: usize, total: usize, next: Option<usize>, id: &str) -> Value {
        json!({"jobs":[{"id":id}],"offset":index,"total":total,"next_offset":next})
    }
    #[test]
    fn reads_more_than_two_legacy_pages() {
        let a = ruwac_protocol::id("pane");
        let b = ruwac_protocol::id("pane");
        let c = ruwac_protocol::id("pane");
        assert_eq!(
            peer(vec![
                page(0, 3, Some(1), &a),
                page(1, 3, Some(2), &b),
                page(2, 3, None, &c)
            ])
            .unwrap()
            .len(),
            3
        );
    }
    #[test]
    fn rejects_changed_membership() {
        let a = ruwac_protocol::id("pane");
        let b = ruwac_protocol::id("pane");
        let mut first = page(0, 2, Some(1), &a);
        first["membership"] = json!("original");
        let mut second = page(1, 2, None, &b);
        second["membership"] = json!("changed");
        assert!(peer(vec![first, second]).is_err());
    }
    #[test]
    fn rejects_duplicate_ids() {
        let a = ruwac_protocol::id("pane");
        assert!(peer(vec![page(0, 2, Some(1), &a), page(1, 2, None, &a)]).is_err());
    }
    #[test]
    fn rejects_stalled_cursor_and_missing_rows() {
        let a = ruwac_protocol::id("pane");
        assert!(peer(vec![page(0, 2, Some(0), &a)]).is_err());
        assert!(peer(vec![page(0, 2, None, &a)]).is_err());
    }
}
