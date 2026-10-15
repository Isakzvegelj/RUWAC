//! Compatibility response for retired hover-overlay fixture names.
pub fn fixture(_: &gtk::Window, _: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"error":"removed"})
}
