//! Endpoint-bound certificate approval; provisional TLS never authorizes credentials.
use sha2::{Digest, Sha256};
use std::time::SystemTime;
use x509_cert::der::Decode;

const PREFIX: &str = "pin-sha256:";
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trust {
    System,
    CaFile(String),
    Leaf([u8; 32]),
}
#[derive(Clone, Debug)]
pub struct Review {
    pub host: String,
    pub port: u16,
    pub digest: [u8; 32],
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: String,
    pub valid_now: bool,
    pub changed: bool,
    pub previous: Option<String>,
}
fn host_identity(host: &str) -> String {
    host.parse::<std::net::IpAddr>()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| host.trim_end_matches('.').to_ascii_lowercase())
}
fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn digest_from_hex(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid saved RDP certificate fingerprint".into());
    }
    let mut out = [0; 32];
    for (i, item) in out.iter_mut().enumerate() {
        *item = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)
            .map_err(|_| "Invalid saved RDP certificate fingerprint")?;
    }
    Ok(out)
}
impl Trust {
    pub fn parse(value: Option<&str>, host: &str, port: u16) -> Result<Self, String> {
        let value = value.unwrap_or("");
        if value.is_empty() {
            return Ok(Self::System);
        }
        if let Some(pin) = value.strip_prefix(PREFIX) {
            let (digest, endpoint) = pin
                .split_once('@')
                .ok_or("Invalid saved RDP certificate trust")?;
            let (bound_port, bound_host) = endpoint
                .split_once(':')
                .ok_or("Invalid saved RDP certificate endpoint")?;
            if bound_port.parse::<u16>().ok() != Some(port)
                || bound_host != host_identity(host)
                || port == 0
            {
                return Err(
                    "Saved RDP certificate belongs to another endpoint; review trust again".into(),
                );
            }
            return Ok(Self::Leaf(digest_from_hex(digest)?));
        }
        if value.starts_with("pin-") {
            return Err("Unsupported saved RDP certificate trust".into());
        }
        Ok(Self::CaFile(value.into()))
    }
    pub fn is_pinned(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }
    pub fn verify_leaf(&self, der: &[u8], host: &str, port: u16) -> Result<Review, String> {
        let review = Review::from_der(der, host, port, self)?;
        if !review.valid_now {
            return Err("RDP certificate is expired or not yet valid".into());
        }
        if matches!(self, Self::Leaf(expected) if expected != &review.digest) {
            return Err(
                "RDP certificate changed; review its fingerprint before reconnecting".into(),
            );
        }
        Ok(review)
    }
}
impl Review {
    pub fn from_der(der: &[u8], host: &str, port: u16, previous: &Trust) -> Result<Self, String> {
        if der.len() > 64 * 1024 || host.len() > 255 || port == 0 {
            return Err("RDP certificate or endpoint exceeds limits".into());
        }
        let cert = x509_cert::Certificate::from_der(der)
            .map_err(|_| "Peer certificate decoding failed")?;
        let digest: [u8; 32] = Sha256::digest(der).into();
        let validity = cert.tbs_certificate.validity;
        let now = SystemTime::now();
        let clean = |value: String| {
            value
                .chars()
                .filter(|c| !c.is_control())
                .take(256)
                .collect::<String>()
        };
        Ok(Self {
            host: host.into(),
            port,
            digest,
            subject: clean(cert.tbs_certificate.subject.to_string()),
            issuer: clean(cert.tbs_certificate.issuer.to_string()),
            valid_from: validity.not_before.to_string(),
            valid_until: validity.not_after.to_string(),
            valid_now: now >= validity.not_before.to_system_time()
                && now <= validity.not_after.to_system_time(),
            changed: matches!(previous, Trust::Leaf(old) if old != &digest),
            previous: match previous {
                Trust::Leaf(old) => Some(hex(old)),
                _ => None,
            },
        })
    }
    pub fn fingerprint(&self) -> String {
        self.digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }
    pub fn token(&self) -> String {
        format!(
            "{PREFIX}{}@{}:{}",
            hex(&self.digest),
            self.port,
            host_identity(&self.host)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_certificate_matches_pin_and_rejects_changed_or_expired_leaf() {
        use std::process::Command;
        let root = std::env::temp_dir().join(ruwac_protocol::id("certificate_policy"));
        std::fs::create_dir(&root).unwrap();
        let cert = root.join("cert.pem");
        let key = root.join("key.pem");
        let output = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=Generated fixture",
            ])
            .arg("-keyout")
            .arg(&key)
            .arg("-out")
            .arg(&cert)
            .output()
            .unwrap();
        assert!(output.status.success());
        let der = Command::new("openssl")
            .arg("x509")
            .arg("-in")
            .arg(&cert)
            .args(["-outform", "DER"])
            .output()
            .unwrap();
        assert!(der.status.success());
        let review = Review::from_der(&der.stdout, "127.0.0.1", 3389, &Trust::System).unwrap();
        assert!(review.valid_now);
        let pin = Trust::parse(Some(&review.token()), "127.0.0.1", 3389).unwrap();
        assert!(pin.verify_leaf(&der.stdout, "127.0.0.1", 3389).is_ok());
        assert!(
            Trust::Leaf([0; 32])
                .verify_leaf(&der.stdout, "127.0.0.1", 3389)
                .unwrap_err()
                .contains("changed")
        );
        let mut expired = x509_cert::Certificate::from_der(&der.stdout).unwrap();
        expired.tbs_certificate.validity.not_after = x509_cert::time::Time::try_from(
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(86400),
        )
        .unwrap();
        let expired = x509_cert::der::Encode::to_der(&expired).unwrap();
        let expired_review = Review::from_der(&expired, "127.0.0.1", 3389, &Trust::System).unwrap();
        assert!(!expired_review.valid_now);
        // Pinning intentionally attests exact DER; expiry is an independent gate.
        let expired_pin = Trust::parse(Some(&expired_review.token()), "127.0.0.1", 3389).unwrap();
        assert!(
            expired_pin
                .verify_leaf(&expired, "127.0.0.1", 3389)
                .unwrap_err()
                .contains("expired")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn saved_fingerprint_is_bound_to_host_and_port() {
        let value = format!("{PREFIX}{}@3389:fedora.example.test", "a1".repeat(32));
        assert!(matches!(
            Trust::parse(Some(&value), "fedora.example.test.", 3389),
            Ok(Trust::Leaf(_))
        ));
        assert!(Trust::parse(Some(&value), "windows.example.test", 3389).is_err());
        assert!(Trust::parse(Some(&value), "fedora.example.test", 3390).is_err());
        assert_eq!(
            Trust::parse(None, "fedora.example.test", 3389).unwrap(),
            Trust::System
        );
        assert!(matches!(
            Trust::parse(Some("root.pem"), "fedora.example.test", 3389),
            Ok(Trust::CaFile(_))
        ));
    }
    #[test]
    fn malformed_pins_never_fall_back_to_unchecked_tls() {
        for value in [
            "pin-sha256:bad",
            "pin-sha256:AA@3389:fedora.example.test",
            "pin-unknown:value",
        ] {
            assert!(Trust::parse(Some(value), "fedora.example.test", 3389).is_err());
        }
        let value = format!("{PREFIX}{}@0:fedora.example.test", "a1".repeat(32));
        assert!(Trust::parse(Some(&value), "fedora.example.test", 0).is_err());
    }
    #[test]
    fn ipv6_pin_identity_is_canonical_and_certificate_input_is_bounded() {
        let value = format!("{PREFIX}{}@3389:::1", "a1".repeat(32));
        assert!(Trust::parse(Some(&value), "0:0:0:0:0:0:0:1", 3389).is_ok());
        assert!(
            Review::from_der(&vec![0; 65537], "fedora.example.test", 3389, &Trust::System).is_err()
        );
        assert!(
            Review::from_der(
                b"not a certificate",
                "fedora.example.test",
                3389,
                &Trust::System
            )
            .is_err()
        );
    }
}
