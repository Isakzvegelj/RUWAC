//! Explicit agent launch in a persistent remote RUWAC session.
use crate::{profiles, sessions, worker::Rpc};
use gtk::prelude::*;
use serde_json::json;
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
fn argv(
    profile: &profiles::Profile,
    agent: u32,
    binary: &str,
    socket: &str,
) -> Result<Vec<String>, String> {
    if profile.protocol != "SSH"
        || (!profile.preferences.gateway.is_empty()
            && profile.preferences.ssh_jump_alias.is_empty())
    {
        return Err("Choose an SSH endpoint with a direct route or an explicit jump alias for the Linux account".into());
    }
    if binary.is_empty()
        || [binary, socket]
            .iter()
            .any(|v| v.len() > 512 || v.chars().any(char::is_control))
    {
        return Err("Invalid remote RUWAC executable or socket".into());
    }
    let (name, command) = match agent {
        0 => (
            "Codex",
            "if type cx >/dev/null 2>&1; then cx; else exec codex agents --no-alt-screen; fi",
        ),
        1 => (
            "Claude Code",
            "if type cc >/dev/null 2>&1 && alias cc >/dev/null 2>&1; then cc; else exec claude --resume; fi",
        ),
        _ => return Err("Choose Codex or Claude Code".into()),
    };
    let cli = json!([binary]).to_string();
    let socket = if socket.is_empty() {
        vec![]
    } else {
        vec!["--socket", socket]
    };
    let script = format!(
        r#"import json,os,shutil,subprocess,sys
cli=json.loads({cli:?})+json.loads({socket:?})
exe=shutil.which(cli[0])
if not exe:sys.exit('Install the RUWAC CLI on this machine before launching a persistent agent.')
cli[0]=exe
subprocess.run(cli+['start'],check=True,stdout=subprocess.DEVNULL)
params={{'name':{name:?},'argv':['bash','-lic',{command:?}]}}
reply=json.loads(subprocess.check_output(cli+['rpc','session.create','--params',json.dumps(params)],text=True))
sid=reply.get('result',{{}}).get('session')
if not isinstance(sid,str) or not sid.startswith('session_'):sys.exit('Remote RUWAC session creation failed.')
print('Persistent '+{name:?}+' session: '+sid,flush=True)
print('Closing this view detaches; the remote agent keeps running.',flush=True)
os.execv(exe,cli+['attach','--session',sid])
"#,
        socket = serde_json::to_string(&socket).unwrap()
    );
    let mut argv = profiles::ssh_argv(profile)?;
    let position = argv.iter().position(|s| s == "--").unwrap();
    argv.splice(
        position..position,
        [
            "-tt".into(),
            "-o".into(),
            "ForwardAgent=no".into(),
            "-o".into(),
            "ClearAllForwardings=yes".into(),
        ],
    );
    argv.push(format!(
        "bash -lc {}",
        quote(&format!("python3 -c {}", quote(&script)))
    ));
    Ok(argv)
}

pub fn button(
    parent: &impl IsA<gtk::Window>,
    inventory: profiles::InventoryReader,
    reader: profiles::ImageProfileReader,
    tabs: adw::TabView,
    views: Rc<RefCell<Vec<Rc<crate::view::View>>>>,
    local_socket: PathBuf,
    broker: crate::clipboard::Broker,
) -> gtk::Button {
    let button = gtk::Button::with_label("Open remote agent…");
    let parent = parent.as_ref().downgrade();
    button.connect_clicked(move |_| {
        let Some(parent) = parent.upgrade() else { return; };
        let dialog = gtk::Dialog::builder().title("Open remote agent").transient_for(&parent).modal(true).default_width(480).build();
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        let open = dialog.add_button("Open", gtk::ResponseType::Accept);
        let body = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).margin_start(16).margin_end(16).margin_top(16).margin_bottom(16).build();
        let rows = inventory().unwrap_or_default().into_iter().filter(|r| r["protocol"] == "SSH" && r["configuration_valid"] == true).collect::<Vec<_>>();
        let names = rows.iter().map(|r| format!("{} / {} · {}",r["group"].as_str().unwrap_or(""),r["name"].as_str().unwrap_or(""),r["server"].as_str().unwrap_or(""))).collect::<Vec<_>>();
        let endpoint = gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
        endpoint.set_widget_name("agent-launch-profile");
        let agent = gtk::DropDown::from_strings(&["Codex (cx)", "Claude Code (cc)"]);
        agent.set_widget_name("agent-launch-agent");
        let binary = gtk::Entry::builder().text("ruwac").build();
        binary.set_widget_name("agent-launch-binary");
        let socket = gtk::Entry::builder().placeholder_text("Leave empty for the remote user's default").build();
        socket.set_widget_name("agent-launch-socket");
        for (label, widget) in [("SSH machine / Linux account",endpoint.upcast_ref::<gtk::Widget>()),("Agent",agent.upcast_ref()),("Remote RUWAC executable",binary.upcast_ref()),("Remote daemon socket",socket.upcast_ref())] {
            let l=gtk::Label::new(Some(label));l.set_xalign(0.0);body.append(&l);body.append(widget);
        }
        let status=gtk::Label::new(Some(if rows.is_empty(){"Add an SSH endpoint first. RDP service credentials are separate from the Linux SSH account."}else{"Starts or reuses the remote RUWAC daemon and opens a persistent agent session. Requires RUWAC, Python 3 and the agent on the remote host. SSH uses your key or prompts in the terminal; no password is embedded in the command."}));status.set_wrap(true);body.append(&status);
        open.set_sensitive(!rows.is_empty());dialog.content_area().append(&body);
        let (tabs,views,local_socket,broker,reader)=(tabs.clone(),views.clone(),local_socket.clone(),broker.clone(),reader.clone());
        dialog.connect_response(move |d,response| {
            if response!=gtk::ResponseType::Accept { d.close();return; }
            let plan=rows.get(endpoint.selected() as usize).and_then(|r|r["id"].as_str()).ok_or("Choose an SSH endpoint".to_string()).and_then(|id|reader(id)).and_then(|p|argv(&p,agent.selected(),&binary.text(),&socket.text()));
            let argv=match plan {Ok(a)=>a,Err(e)=>{status.set_text(&e);return;}};
            open.set_sensitive(false);status.set_text("Opening remote agent terminal…");
            let (tx,rx)=std::sync::mpsc::sync_channel(1);let socket=local_socket.clone();
            std::thread::spawn(move || {let result=(||{let mut rpc=Rpc::connect(&socket)?;rpc.call("session.create",json!({"name":"Remote agent","argv":argv}))})();let _=tx.send(result);});
            let (tabs,views,socket,broker,dialog,status,open)=(tabs.clone(),views.clone(),local_socket.clone(),broker.clone(),d.downgrade(),status.clone(),open.clone());
            gtk::glib::timeout_add_local(Duration::from_millis(20),move ||match rx.try_recv(){
                Ok(Ok(r))=>{if let Some(id)=r["session"].as_str(){sessions::append(&tabs,&views,&socket,&broker,id,"Remote agent");}if let Some(d)=dialog.upgrade(){d.close();}gtk::glib::ControlFlow::Break},
                Ok(Err(_))|Err(std::sync::mpsc::TryRecvError::Disconnected)=>{status.set_text("Could not create the local SSH terminal");open.set_sensitive(true);gtk::glib::ControlFlow::Break},
                Err(std::sync::mpsc::TryRecvError::Empty)=>gtk::glib::ControlFlow::Continue,
            });
        });
        dialog.present();
    });
    button
}

pub fn fixture(q: &serde_json::Value) -> serde_json::Value {
    let mut visible = false;
    let mut profiles = 0;
    for w in gtk::Window::list_toplevels() {
        if let Some(d) = w.downcast_ref::<gtk::Dialog>() {
            if d.title().as_deref() != Some("Open remote agent") {
                continue;
            }
            visible = d.is_visible();
            let mut child = d.content_area().first_child().and_then(|w| w.first_child());
            while let Some(w) = child {
                child = w.next_sibling();
                if w.widget_name() == "agent-launch-profile" {
                    profiles = w
                        .downcast_ref::<gtk::DropDown>()
                        .and_then(|d| d.model())
                        .map(|m| m.n_items())
                        .unwrap_or(0);
                }
            }
            if q["operation"] == "cancel" {
                d.response(gtk::ResponseType::Cancel);
            }
        }
    }
    json!({"dialog_visible":visible,"SSH_profiles":profiles,"agents":["Codex (cx)","Claude Code (cc)"],"starts_agent_on_cancel":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    #[test]
    fn bootstrap_survives_shell_quoting_and_detaches_without_agent_input() {
        let root = std::env::temp_dir().join(ruwac_protocol::id("launch"));
        fs::create_dir(&root).unwrap();
        let binary = root.join("remote ' executable");
        let receipt = root.join("receipt.json");
        // A disposable remote CLI process verifies the real shell/Python
        // boundary. It never starts Codex, Claude or an owner daemon.
        fs::write(&binary,format!("#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\na=sys.argv\nif 'session.create' in a:\n Path({:?}).write_text(a[a.index('--params')+1])\n print(json.dumps({{'result':{{'session':'session_generated'}}}}))\n",receipt.to_str().unwrap())).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let p = profiles::Profile {
            id: "endpoint_generated".into(),
            machine_id: "machine_generated".into(),
            name: "Generated".into(),
            group: String::new(),
            protocol: "SSH".into(),
            server: "127.0.0.1:22".into(),
            username: "fixture".into(),
            secret_reference: "private-reference-never-in-argv".into(),
            favorite: false,
            last_used: 0,
            tags: vec![],
            preferences: Default::default(),
            inherit: false,
            inherit_credential: false,
            inherit_connection: false,
        };
        for agent in [0, 1] {
            let args = argv(
                &p,
                agent,
                binary.to_str().unwrap(),
                "/generated/socket ' quote",
            )
            .unwrap();
            assert!(!args.join(" ").contains(&p.secret_reference));
            let output = Command::new("bash")
                .arg("-c")
                .arg(args.last().unwrap())
                .output()
                .unwrap();
            assert!(output.status.success(), "bootstrap process failed");
            let params: serde_json::Value =
                serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
            assert_eq!(params["argv"][0], "bash");
            assert_eq!(params["argv"][1], "-lic");
            assert_eq!(
                params["name"],
                if agent == 0 { "Codex" } else { "Claude Code" }
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("Closing this view detaches"));
        }
        let mut rdp = p.clone();
        rdp.protocol = "RDP".into();
        assert!(argv(&rdp, 0, "ruwac", "").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
