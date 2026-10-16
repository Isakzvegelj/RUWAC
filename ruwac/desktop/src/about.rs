//! Product information and optional support; no network access until a link is opened.
use adw::prelude::*;
pub const DONATION_URL: &str = "https://idz.si/bs/ruwac-donate.html";
pub const WEBSITE: &str = "https://github.com/w3fans/RUWAC";
pub fn present(parent: &adw::ApplicationWindow) -> gtk::Window {
    let window = gtk::Window::builder()
        .title("About RUWAC")
        .transient_for(parent)
        .modal(true)
        .default_width(460)
        .default_height(590)
        .build();
    window.set_widget_name("ruwac-about");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    let title = gtk::Label::new(Some("RUWAC"));
    title.add_css_class("title-1");
    content.append(&title);
    let description = gtk::Label::new(Some("Remote Unified Workspace & Agent Cockpit"));
    description.set_wrap(true);
    content.append(&description);
    let info = adw::PreferencesGroup::new();
    for (name, value) in [
        ("Installed version", env!("CARGO_PKG_VERSION")),
        ("Author", "Denis Zvegelj"),
        ("License", "Apache-2.0"),
    ] {
        info.add(
            &adw::ActionRow::builder()
                .title(name)
                .subtitle(value)
                .build(),
        );
    }
    content.append(&info);
    content.append(&gtk::LinkButton::with_label(
        WEBSITE,
        "Source code and issue tracker",
    ));
    let support = gtk::Label::new(Some("Support further development"));
    support.add_css_class("heading");
    content.append(&support);
    let qr = gtk::DrawingArea::builder()
        .content_width(196)
        .content_height(196)
        .halign(gtk::Align::Center)
        .build();
    qr.set_widget_name("ruwac-donation-qr");
    qr.set_tooltip_text(Some(DONATION_URL));
    qr.update_property(&[gtk::accessible::Property::Label(
        "QR code for the RUWAC donation page",
    )]);
    qr.set_draw_func(|_, c, w, h| {
        c.set_antialias(gtk::cairo::Antialias::None);
        c.set_source_rgb(1.0, 1.0, 1.0);
        let _ = c.paint();
        let step = (w.min(h) as f64 / QR.len() as f64).floor().max(1.0);
        let ox = (w as f64 - step * QR.len() as f64) / 2.0;
        let oy = (h as f64 - step * QR.len() as f64) / 2.0;
        c.set_source_rgb(0.0, 0.0, 0.0);
        for (y, row) in QR.iter().enumerate() {
            for (x, bit) in row.bytes().enumerate() {
                if bit == b'1' {
                    c.rectangle(ox + x as f64 * step, oy + y as f64 * step, step, step);
                }
            }
        }
        let _ = c.fill();
    });
    content.append(&qr);
    content.append(&gtk::LinkButton::with_label(
        DONATION_URL,
        "Open donation page",
    ));
    let close = gtk::Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    let weak = window.downgrade();
    close.connect_clicked(move |_| {
        if let Some(w) = weak.upgrade() {
            w.close();
        }
    });
    content.append(&close);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&content)
        .build();
    window.set_child(Some(&scroll));
    window.present();
    window
}
// Generated with qrcode 8.2, error correction M, four-module quiet zone.
// Fixed public URL; no runtime dependency or remotely loaded image.
pub const QR: &[&str] = &[
    "0000000000000000000000000000000000000",
    "0000000000000000000000000000000000000",
    "0000000000000000000000000000000000000",
    "0000000000000000000000000000000000000",
    "0000111111101000011101100011111110000",
    "0000100000101010111101110010000010000",
    "0000101110101101011000010010111010000",
    "0000101110100111101111101010111010000",
    "0000101110101100110100110010111010000",
    "0000100000100001111011001010000010000",
    "0000111111101010101010101011111110000",
    "0000000000000100001010000000000000000",
    "0000100111111111111011011100101110000",
    "0000101011011000001100110101101100000",
    "0000101010101001001111000101001000000",
    "0000111101010111101001101100010010000",
    "0000110101101111111010010011000010000",
    "0000001000001010100111100010111110000",
    "0000001110101010001111010100001010000",
    "0000010101010110111000011110001010000",
    "0000111010101000000110000100010000000",
    "0000110111011100100100111000101100000",
    "0000111001101101011110010001110010000",
    "0000110010011001110000100111111000000",
    "0000111111101110110011001111111100000",
    "0000000000001011110100101000110000000",
    "0000111111101110110101111010110000000",
    "0000100000101101110101001000100100000",
    "0000101110101000110010011111110110000",
    "0000101110101100001100000100000010000",
    "0000101110100011011100010101101110000",
    "0000100000100101000000110011111010000",
    "0000111111101101101100011111000000000",
    "0000000000000000000000000000000000000",
    "0000000000000000000000000000000000000",
    "0000000000000000000000000000000000000",
    "0000000000000000000000000000000000000",
];
