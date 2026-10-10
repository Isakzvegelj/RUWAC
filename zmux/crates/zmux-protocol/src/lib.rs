use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, Read, Write};
use uuid::Uuid;
pub const MAX_FRAME: usize = 1024 * 1024;
pub const MAX_TEXT: usize = 256 * 1024;
pub fn id(kind: &str) -> String {
    format!("{kind}_{}", Uuid::new_v4().simple())
}
pub fn valid_id(s: &str, kind: &str) -> bool {
    s.strip_prefix(&format!("{kind}_"))
        .is_some_and(|x| x.len() == 32 && x.bytes().all(|b| b.is_ascii_hexdigit()))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u8,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}
pub fn request(method: &str, params: Value) -> Request {
    Request {
        version: 0,
        id: json!(id("request")),
        method: method.into(),
        params,
        idempotency_key: None,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn response(id: &Value, result: Result<Value>) -> Value {
    match result {
        Ok(v) => json!({"version":0,"id":id,"result":v}),
        Err(e) => json!({"version":0,"id":id,"error":e}),
    }
}
pub fn write_frame(w: &mut impl Write, v: &impl Serialize) -> io::Result<()> {
    let b = serde_json::to_vec(v)?;
    if b.len() > MAX_FRAME {
        return Err(io::Error::other("oversized frame"));
    }
    w.write_all(&(b.len() as u32).to_be_bytes())?;
    w.write_all(&b)?;
    w.flush()
}
pub fn read_frame(r: &mut impl Read) -> io::Result<Value> {
    let mut h = [0; 4];
    r.read_exact(&mut h)?;
    let n = u32::from_be_bytes(h) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(io::Error::other("invalid frame size"));
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b)?;
    Ok(serde_json::from_slice(&b)?)
}
pub async fn read_async(r: &mut (impl tokio::io::AsyncRead + Unpin)) -> io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let n = r.read_u32().await? as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(io::Error::other("invalid frame size"));
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b).await?;
    Ok(b)
}
pub async fn write_async(
    w: &mut (impl tokio::io::AsyncWrite + Unpin),
    v: &Value,
) -> io::Result<()> {
    use tokio::io::AsyncWriteExt;
    let b = serde_json::to_vec(v)?;
    if b.len() > MAX_FRAME {
        return Err(io::Error::other("oversized frame"));
    }
    w.write_u32(b.len() as u32).await?;
    w.write_all(&b).await?;
    w.flush().await
}
pub fn text(v: &Value, k: &str) -> Result<String> {
    v.get(k)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::new("INVALID_PARAMS", format!("{k} must be a string")))
}
pub fn checked_text(s: &str) -> Result<()> {
    if s.contains('\0') {
        return Err(Error::new("INVALID_TEXT", "NUL is not accepted"));
    }
    if s.len() > MAX_TEXT {
        return Err(Error::new("LIMIT", "text exceeds 256 KiB"));
    }
    Ok(())
}
pub fn target(v: &Value, k: &str, kind: &str) -> Result<String> {
    let s = text(v, k)?;
    if !valid_id(&s, kind) {
        return Err(Error::new(
            "INVALID_ID",
            format!("expected {kind}_ followed by 32 hex digits"),
        ));
    }
    Ok(s)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_bounds() {
        let mut b = Vec::new();
        write_frame(&mut b, &json!({"a":"ž"})).unwrap();
        assert_eq!(read_frame(&mut b.as_slice()).unwrap()["a"], "ž");
        assert!(read_frame(&mut (MAX_FRAME as u32 + 1).to_be_bytes().as_slice()).is_err());
    }
    #[test]
    fn target_and_text() {
        assert!(valid_id(&id("pane"), "pane"));
        assert!(!valid_id(&id("window"), "pane"));
        assert!(checked_text("\0").is_err());
        assert!(checked_text(&"x".repeat(MAX_TEXT + 1)).is_err());
    }
}
