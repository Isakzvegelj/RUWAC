//! GTK-owned mixed view trees. Reparenting changes presentation, never connections.
use adw::prelude::*;
use gtk::{Orientation, glib};

pub fn contains(root: &impl IsA<gtk::Widget>, child: &impl IsA<gtk::Widget>) -> bool {
    root.as_ref() == child.as_ref() || child.as_ref().is_ancestor(root)
}

pub fn page(tabs: &adw::TabView, child: &impl IsA<gtk::Widget>) -> Option<adw::TabPage> {
    (0..tabs.n_pages())
        .map(|i| tabs.nth_page(i))
        .find(|p| contains(&p.child(), child))
}

pub fn append(tabs: &adw::TabView, child: &impl IsA<gtk::Widget>) -> adw::TabPage {
    let body = gtk::Box::new(Orientation::Vertical, 0);
    body.set_hexpand(true);
    body.set_vexpand(true);
    body.append(child);
    let page = tabs.append(&body);
    let weak = body.downgrade();
    page.connect_title_notify(move |p| {
        if let Some(w) = weak.upgrade().and_then(|b| b.first_child()) {
            if !w.is::<gtk::Paned>() {
                w.set_tooltip_text(Some(&p.title()));
            }
        }
    });
    page
}

pub fn join(tabs: &adw::TabView, other: i32, vertical: bool) -> Result<(), String> {
    let current = tabs.selected_page().ok_or("Select a workspace tab")?;
    if other < 0 || other >= tabs.n_pages() {
        return Err("Workspace tab is missing".into());
    }
    let other = tabs.nth_page(other);
    if current == other {
        return Err("Choose a different tab to split with".into());
    }
    let a = current
        .child()
        .downcast::<gtk::Box>()
        .map_err(|_| "Invalid workspace container")?;
    let b = other
        .child()
        .downcast::<gtk::Box>()
        .map_err(|_| "Invalid workspace container")?;
    let first = a.first_child().ok_or("Empty workspace")?;
    let second = b.first_child().ok_or("Empty workspace")?;
    let focus = a
        .root()
        .and_then(|w| w.downcast::<gtk::Window>().ok())
        .and_then(|w| gtk::prelude::GtkWindowExt::focus(&w));
    // Both trees stay alive. Emptying the donor before close avoids resource-close callbacks.
    a.remove(&first);
    b.remove(&second);
    let split = gtk::Paned::new(if vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    });
    split.set_hexpand(true);
    split.set_vexpand(true);
    split.set_resize_start_child(true);
    split.set_resize_end_child(true);
    split.set_shrink_start_child(false);
    split.set_shrink_end_child(false);
    split.set_wide_handle(true);
    split.set_start_child(Some(&first));
    split.set_end_child(Some(&second));
    // Choose a usable midpoint after allocation; later GTK divider drags remain independent.
    split.connect_map(|p| {
        let weak = p.downgrade();
        glib::idle_add_local_once(move || {
            if let Some(p) = weak.upgrade() {
                let size = if p.orientation() == Orientation::Vertical {
                    p.height()
                } else {
                    p.width()
                };
                if p.position() == 0 && size > 0 {
                    p.set_position(size / 2);
                }
            }
        });
    });
    a.append(&split);
    current.set_title(&format!("{} + {}", current.title(), other.title()));
    tabs.close_page(&other);
    tabs.set_selected_page(&current);
    if let Some(focus) = focus {
        focus.grab_focus();
    }
    Ok(())
}

fn leaves(node: gtk::Widget, out: &mut Vec<gtk::Widget>) {
    if let Some(p) = node.downcast_ref::<gtk::Paned>() {
        if let Some(a) = p.start_child() {
            leaves(a, out);
        }
        if let Some(b) = p.end_child() {
            leaves(b, out);
        }
    } else {
        out.push(node);
    }
}

pub fn separate(tabs: &adw::TabView) -> Result<usize, String> {
    let current = tabs.selected_page().ok_or("Select a workspace tab")?;
    let body = current
        .child()
        .downcast::<gtk::Box>()
        .map_err(|_| "Invalid workspace container")?;
    let tree = body.first_child().ok_or("Empty workspace")?;
    if !tree.is::<gtk::Paned>() {
        return Ok(1);
    }
    let mut views = Vec::new();
    leaves(tree.clone(), &mut views);
    let focus = body
        .root()
        .and_then(|w| w.downcast::<gtk::Window>().ok())
        .and_then(|w| gtk::prelude::GtkWindowExt::focus(&w));
    body.remove(&tree);
    for leaf in &views {
        if let Some(parent) = leaf.parent().and_then(|p| p.downcast::<gtk::Paned>().ok()) {
            if parent.start_child().as_ref() == Some(leaf) {
                parent.set_start_child(None::<&gtk::Widget>);
            } else {
                parent.set_end_child(None::<&gtk::Widget>);
            }
        }
    }
    for (i, leaf) in views.iter().enumerate() {
        if i == 0 {
            body.append(leaf);
            current.set_title(leaf.tooltip_text().as_deref().unwrap_or("Workspace view"));
        } else {
            let page = append(tabs, leaf);
            page.set_title(leaf.tooltip_text().as_deref().unwrap_or("Workspace view"));
        }
    }
    tabs.set_selected_page(&current);
    if let Some(focus) = focus {
        focus.grab_focus();
    }
    Ok(views.len())
}

pub fn controls(tabs: &adw::TabView) -> gtk::Box {
    let root = gtk::Box::new(Orientation::Horizontal, 4);
    let button = gtk::Button::with_label("Split tabs…");
    let separate_button = gtk::Button::with_label("Separate views");
    separate_button.set_tooltip_text(Some(
        "Move every mixed pane into its own tab; connections keep running",
    ));
    root.append(&button);
    root.append(&separate_button);
    let t = tabs.clone();
    button.connect_clicked(move |button| {
        let d = gtk::Dialog::builder()
            .title("Combine tabs into a mixed workspace")
            .modal(true)
            .default_width(500)
            .build();
        if let Some(w) = button.root().and_then(|w| w.downcast::<gtk::Window>().ok()) {
            d.set_transient_for(Some(&w));
        }
        d.add_button("Cancel", gtk::ResponseType::Cancel);
        d.connect_response(|d, _| d.close());
        let orientation = gtk::DropDown::from_strings(&["Side by side", "Top and bottom"]);
        d.content_area().append(&orientation);
        let status = gtk::Label::new(Some(
            "Choose another live tab. Its views keep their connections.",
        ));
        status.set_wrap(true);
        d.content_area().append(&status);
        let selected = t.selected_page();
        for index in 0..t.n_pages() {
            let page = t.nth_page(index);
            if Some(&page) == selected.as_ref() {
                continue;
            }
            let add = gtk::Button::with_label(&page.title());
            let (tabs, dialog, o, status) = (
                t.clone(),
                d.downgrade(),
                orientation.clone(),
                status.clone(),
            );
            add.connect_clicked(move |_| {
                if crate::tiles::page(&tabs, &page.child()).is_none() {
                    status.set_text("The selected tab closed");
                    return;
                }
                match join(&tabs, tabs.page_position(&page), o.selected() == 1) {
                    Ok(()) => {
                        if let Some(d) = dialog.upgrade() {
                            d.close();
                        }
                    }
                    Err(e) => status.set_text(&e),
                }
            });
            d.content_area().append(&add);
        }
        d.present();
    });
    let t = tabs.clone();
    separate_button.connect_clicked(move |button| {
        if let Err(e) = separate(&t) {
            button.set_tooltip_text(Some(&e));
        }
    });
    root
}

pub fn inspect(tabs: &adw::TabView) -> serde_json::Value {
    fn node(w: gtk::Widget) -> serde_json::Value {
        if let Some(p) = w.downcast_ref::<gtk::Paned>() {
            serde_json::json!({"split":if p.orientation()==Orientation::Vertical{"vertical"}else{"horizontal"},"position":p.position(),"width":p.width(),"height":p.height(),"first":p.start_child().map(node),"second":p.end_child().map(node)})
        } else {
            serde_json::json!({"view":w.type_().name(),"mapped":w.is_mapped(),"width":w.width(),"height":w.height()})
        }
    }
    serde_json::json!({"tabs":(0..tabs.n_pages()).map(|i|{let p=tabs.nth_page(i);serde_json::json!({"title":p.title().as_str(),"layout":p.child().first_child().map(node)})}).collect::<Vec<_>>()})
}

pub fn snapshot(
    tabs: &adw::TabView,
    describe: impl Fn(&gtk::Widget) -> Option<crate::workspace::Layout>,
) -> Result<Vec<crate::workspace::Tab>, String> {
    fn walk(
        w: &gtk::Widget,
        describe: &impl Fn(&gtk::Widget) -> Option<crate::workspace::Layout>,
    ) -> Result<crate::workspace::Layout, String> {
        if let Some(p) = w.downcast_ref::<gtk::Paned>() {
            let size = if p.orientation() == Orientation::Vertical {
                p.height()
            } else {
                p.width()
            };
            let ratio = if size > 0 {
                (p.position() as f64 / size as f64).clamp(0.05, 0.95)
            } else {
                0.5
            };
            Ok(crate::workspace::Layout::Split {
                vertical: p.orientation() == Orientation::Vertical,
                ratio,
                first: Box::new(walk(&p.start_child().ok_or("Empty split")?, describe)?),
                second: Box::new(walk(&p.end_child().ok_or("Empty split")?, describe)?),
            })
        } else {
            describe(w)
                .ok_or_else(|| "Unrecognized workspace view; existing metadata preserved".into())
        }
    }
    (0..tabs.n_pages())
        .map(|i| {
            let p = tabs.nth_page(i);
            Ok(crate::workspace::Tab {
                title: p.title().to_string(),
                layout: walk(&p.child().first_child().ok_or("Empty tab")?, &describe)?,
            })
        })
        .collect()
}

pub fn restore(
    tabs: &adw::TabView,
    saved: &[crate::workspace::Tab],
    mut create: impl FnMut(&crate::workspace::Layout) -> gtk::Widget,
) {
    fn build(
        layout: &crate::workspace::Layout,
        create: &mut impl FnMut(&crate::workspace::Layout) -> gtk::Widget,
    ) -> gtk::Widget {
        if let crate::workspace::Layout::Split {
            vertical,
            ratio,
            first,
            second,
        } = layout
        {
            let p = gtk::Paned::new(if *vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            });
            p.set_hexpand(true);
            p.set_vexpand(true);
            p.set_wide_handle(true);
            p.set_resize_start_child(true);
            p.set_resize_end_child(true);
            p.set_shrink_start_child(false);
            p.set_shrink_end_child(false);
            p.set_start_child(Some(&build(first, create)));
            p.set_end_child(Some(&build(second, create)));
            let ratio = *ratio;
            p.connect_map(move |p| {
                let weak = p.downgrade();
                glib::idle_add_local_once(move || {
                    if let Some(p) = weak.upgrade() {
                        let size = if p.orientation() == Orientation::Vertical {
                            p.height()
                        } else {
                            p.width()
                        };
                        if size > 0 {
                            p.set_position((size as f64 * ratio).round() as i32);
                        }
                    }
                });
            });
            p.upcast()
        } else {
            create(layout)
        }
    }
    for tab in saved {
        let tree = build(&tab.layout, &mut create);
        let page = append(tabs, &tree);
        page.set_title(&tab.title);
    }
}
