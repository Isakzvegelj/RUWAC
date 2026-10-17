//! SSH_ASKPASS helper for endpoints that log in with a saved password.
//! ssh runs this binary with the prompt as its only argument; the answer is
//! written to stdout. The secret comes from Secret Service, never argv.
use gtk::{glib, prelude::*};
use std::{cell::RefCell, rc::Rc};

/// Names the endpoint's Secret Service credential reference.
pub const REFERENCE: &str = "RUWAC_ASKPASS_REF";
/// Background image delivery may use saved credentials but never opens a dialog.
pub const ONLY_SAVED: &str = "RUWAC_ASKPASS_ONLY_SAVED";
pub const KEY_REFERENCE: &str = "RUWAC_ASKPASS_KEY_REF";

/// Some(exit status) when ssh started this process as its askpass program.
pub fn run_if_requested() -> Option<i32> {
    let reference = std::env::var(REFERENCE).ok()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() > 1 || args.first().is_some_and(|a| a.starts_with("--")) {
        return None;
    }
    let prompt = args.into_iter().next().unwrap_or_default();
    Some(answer(&reference, &prompt))
}

fn password_prompt(prompt: &str) -> bool {
    let prompt = prompt.to_lowercase();
    prompt.contains("password") && !prompt.contains("passphrase")
}
fn confirmation_prompt(prompt: &str) -> bool {
    prompt.contains("(yes/no")
}

/// The saved password is offered once per ssh process; a rejected password
/// is then asked of the person instead of being repeated.
fn first_attempt() -> bool {
    let Some(dir) =
        std::env::var_os("XDG_RUNTIME_DIR").map(|d| std::path::PathBuf::from(d).join("ruwac"))
    else {
        return true;
    };
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let stale = entry.file_name().to_string_lossy().starts_with("askpass-")
                && entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .is_ok_and(|t| t.elapsed().is_ok_and(|age| age.as_secs() > 3600));
            if stale {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let marker = dir.join(format!("askpass-{}", unsafe { libc::getppid() }));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)
        .is_ok()
        || !dir.is_dir()
}

fn answer(reference: &str, prompt: &str) -> i32 {
    if std::env::var(ONLY_SAVED).as_deref() == Ok("1") {
        let selected = if password_prompt(prompt) {
            Some(reference.to_owned())
        } else if prompt.to_lowercase().contains("passphrase for key") {
            std::env::var(KEY_REFERENCE).ok().filter(|r| !r.is_empty())
        } else {
            None
        };
        return selected
            .and_then(|r| crate::credentials::lookup(&r).ok())
            .map(|secret| {
                println!("{secret}");
                0
            })
            .unwrap_or(1);
    }
    if password_prompt(prompt) && first_attempt() {
        if let Ok(secret) = crate::credentials::lookup(reference) {
            println!("{secret}");
            return 0;
        }
    }
    dialog(prompt)
}

fn dialog(prompt: &str) -> i32 {
    if gtk::init().is_err() {
        return 1;
    }
    let confirm = confirmation_prompt(prompt);
    let window = gtk::Window::builder()
        .title("SSH")
        .default_width(440)
        .build();
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    let label = gtk::Label::new(Some(prompt.trim()));
    label.set_wrap(true);
    label.set_selectable(true);
    label.set_xalign(0.0);
    body.append(&label);
    let entry = gtk::PasswordEntry::builder().show_peek_icon(true).build();
    if !confirm {
        body.append(&entry);
    }
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label(if confirm { "No" } else { "Cancel" });
    let ok = gtk::Button::with_label(if confirm {
        "Yes, trust this host"
    } else {
        "OK"
    });
    ok.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&ok);
    body.append(&buttons);
    window.set_child(Some(&body));
    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let main = glib::MainLoop::new(None, false);
    let accept = {
        let (result, main, entry) = (result.clone(), main.clone(), entry.clone());
        Rc::new(move || {
            *result.borrow_mut() = Some(if confirm {
                "yes".into()
            } else {
                entry.text().to_string()
            });
            main.quit();
        })
    };
    let a = accept.clone();
    ok.connect_clicked(move |_| a());
    let a = accept.clone();
    entry.connect_activate(move |_| a());
    let m = main.clone();
    cancel.connect_clicked(move |_| m.quit());
    let m = main.clone();
    window.connect_close_request(move |_| {
        m.quit();
        glib::Propagation::Proceed
    });
    window.present();
    if !confirm {
        entry.grab_focus();
    }
    main.run();
    window.close();
    match result.take() {
        Some(text) => {
            println!("{text}");
            0
        }
        None => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_ssh_prompts() {
        assert!(password_prompt("user@host's password: "));
        assert!(password_prompt("Password: "));
        assert!(!password_prompt(
            "Enter passphrase for key '~/.ssh/id_ed25519': "
        ));
        assert!(confirmation_prompt(
            "Are you sure you want to continue connecting (yes/no/[fingerprint])? "
        ));
        assert!(!confirmation_prompt("Password: "));
    }
}
