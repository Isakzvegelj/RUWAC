//! Connection-owned staging: nothing reaches the PTY until a complete commit.
use ruwac_protocol::{Error, Result, checked_text};
use std::time::{Duration, Instant};
pub const MAX_BYTES: usize = 1024 * 1024;
pub fn size(v: &serde_json::Value, key: &str) -> Result<usize> {
    v.get(key)
        .and_then(serde_json::Value::as_u64)
        .filter(|n| *n <= MAX_BYTES as u64)
        .map(|n| n as usize)
        .ok_or_else(|| {
            Error::new(
                "INVALID_PARAMS",
                "paste size must be an integer from zero through 1 MiB",
            )
        })
}
pub struct Upload {
    pub token: String,
    pub pane: String,
    total: usize,
    text: String,
    started: Instant,
}
impl Upload {
    pub fn new(token: String, pane: String, total: usize) -> Self {
        Self {
            token,
            pane,
            total,
            text: String::new(),
            started: Instant::now(),
        }
    }
    pub fn expired(&self) -> bool {
        self.started.elapsed() >= Duration::from_secs(30)
    }
    pub fn append(&mut self, offset: usize, text: &str) -> Result<()> {
        checked_text(text)?;
        if self.expired() {
            return Err(Error::new("PASTE_EXPIRED", "paste staging expired"));
        }
        if offset != self.text.len() || text.len() > self.total - self.text.len() {
            return Err(Error::new(
                "INVALID_PARAMS",
                "paste offset or declared length mismatch",
            ));
        }
        self.text.push_str(text);
        Ok(())
    }
    pub fn finish(self, bracketed: bool) -> Result<Vec<u8>> {
        if self.expired() || self.text.len() != self.total {
            return Err(Error::new(
                "INCOMPLETE_PASTE",
                "paste incomplete or expired; no input sent",
            ));
        }
        if self.text.contains("\x1b[201~") {
            return Err(Error::new(
                "INVALID_TEXT",
                "paste contains embedded end marker",
            ));
        }
        Ok(if bracketed {
            format!("\x1b[200~{}\x1b[201~", self.text).into_bytes()
        } else {
            self.text.into_bytes()
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sizes_preserve_large_values_and_reject_nonintegers() {
        assert_eq!(
            size(&serde_json::json!({"bytes":MAX_BYTES}), "bytes").unwrap(),
            MAX_BYTES
        );
        for v in [
            serde_json::json!({"bytes":MAX_BYTES+1}),
            serde_json::json!({"bytes":-1}),
            serde_json::json!({"bytes":"1"}),
            serde_json::json!({}),
        ] {
            assert!(size(&v, "bytes").is_err());
        }
    }
    #[test]
    fn transaction_checks_lengths_boundaries_and_one_wrapper() {
        let mut u = Upload::new("generated".into(), "pane".into(), MAX_BYTES);
        for offset in (0..MAX_BYTES).step_by(32768) {
            u.append(offset, &"x".repeat(32768)).unwrap();
        }
        let b = u.finish(true).unwrap();
        assert_eq!(b.len(), MAX_BYTES + 12);
        assert!(b.starts_with(b"\x1b[200~") && b.ends_with(b"\x1b[201~"));
        let mut u = Upload::new("generated".into(), "pane".into(), 4);
        assert!(u.append(1, "x").is_err());
        assert!(u.append(0, "xxxxx").is_err());
        u.append(0, "ž").unwrap();
        assert!(u.append(1, "x").is_err());
        assert!(u.finish(false).is_err());
    }
    #[test]
    fn split_marker_and_expiration_are_rejected_before_delivery() {
        let mut u = Upload::new("generated".into(), "pane".into(), 6);
        u.append(0, "\x1b[20").unwrap();
        u.append(4, "1~").unwrap();
        assert!(u.finish(true).is_err());
        let mut u = Upload::new("generated".into(), "pane".into(), 1);
        u.started = Instant::now() - Duration::from_secs(31);
        assert!(u.append(0, "x").is_err());
        assert!(u.finish(false).is_err());
    }
}
