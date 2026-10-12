//! Same-endpoint GNOME redirection and RDSTLS opaque credentials (MS-RDPBCGR 2.2.13/2.2.17).
//! No credential or routing field implements Debug, Serialize, or profile persistence.
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub struct Plan {
    pub session: u32,
    route: Vec<u8>,
    user: Vec<u8>,
    domain: Vec<u8>,
    password: Vec<u8>,
    guid: Vec<u8>,
}
struct SensitivePacket(Vec<u8>);
impl Drop for SensitivePacket {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}
impl Drop for Plan {
    fn drop(&mut self) {
        for data in [
            &mut self.route,
            &mut self.user,
            &mut self.domain,
            &mut self.password,
            &mut self.guid,
        ] {
            data.fill(0);
        }
    }
}
impl Plan {
    pub fn routing_token(&self) -> Result<String, String> {
        let value = std::str::from_utf8(&self.route).map_err(|_| "Invalid GNOME routing token")?;
        // FreeRDP may supply the complete X.224 cookie line as opaque LoadBalanceInfo.
        let value = value.strip_suffix('\0').unwrap_or(value);
        let value = value.strip_prefix("Cookie: msts=").unwrap_or(value);
        let value = value.strip_suffix("\r\n").unwrap_or(value);
        if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Unsupported GNOME routing token format".into());
        }
        Ok(value.to_owned())
    }
    pub fn username(&self) -> Result<String, String> {
        text(&self.user)
    }
    fn request(&self) -> Result<SensitivePacket, String> {
        let mut data = vec![1, 0, 2, 0, 1, 0];
        for field in [&self.guid, &self.user, &self.domain, &self.password] {
            let len = u16::try_from(field.len())
                .map_err(|_| "RDSTLS credential length exceeds bounds")?;
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(field);
        }
        Ok(SensitivePacket(data))
    }
}
fn text(data: &[u8]) -> Result<String, String> {
    if data.len() < 2 || data.len() % 2 != 0 || data[data.len() - 2..] != [0, 0] {
        return Err("Invalid handover string".into());
    }
    let mut units: Vec<u16> = data[..data.len() - 2]
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect();
    let value = String::from_utf16(&units).map_err(|_| "Invalid handover Unicode");
    units.fill(0);
    value.map_err(str::to_owned)
}
pub fn parse(data: &[u8]) -> Result<Option<Plan>, String> {
    if data.len() < 4 || u16::from_le_bytes([data[2], data[3]]) & 15 != 10 {
        return Ok(None);
    }
    if !matches!(u16::from_le_bytes([data[2], data[3]]), 0x0a | 0x1a) {
        return Err("Unsupported redirection share header".into());
    }
    if data.len() < 20 || data.len() > 65536 {
        return Err("Invalid GNOME redirection size".into());
    }
    let total = u16::from_le_bytes([data[0], data[1]]) as usize;
    if total != data.len() {
        return Err("Truncated GNOME redirection header".into());
    }
    let packet = &data[8..];
    if u16::from_le_bytes([packet[0], packet[1]]) != 0x400 {
        return Err("Invalid redirection packet marker".into());
    }
    let length = u16::from_le_bytes([packet[2], packet[3]]) as usize;
    if length < 12 || length > packet.len() || packet.len() - length > 8 {
        return Err("Invalid redirection packet length".into());
    }
    let flags = u32::from_le_bytes(packet[8..12].try_into().unwrap());
    // This narrowly supports GNOME's encrypted, same-endpoint handover. Never trust a supplied destination/certificate.
    let permitted = 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x4000 | 0x8000 | 0x10000;
    if flags & !permitted != 0
        || flags & (0x2 | 0x4 | 0x10 | 0x4000 | 0x8000) != (0x2 | 0x4 | 0x10 | 0x4000 | 0x8000)
    {
        return Err("Unsupported redirection target or credential mode".into());
    }
    let mut cursor = 12usize;
    let mut plan = Plan {
        session: u32::from_le_bytes(packet[4..8].try_into().unwrap()),
        route: vec![],
        user: vec![],
        domain: vec![0, 0],
        password: vec![],
        guid: vec![],
    };
    for bit in [0x2, 0x4, 0x8, 0x10, 0x8000, 0x10000] {
        if flags & bit == 0 {
            continue;
        }
        if cursor + 4 > length {
            return Err("Truncated redirection field".into());
        }
        let size = u32::from_le_bytes(packet[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        if size > 16384 || size > length.saturating_sub(cursor) {
            return Err("Redirection field exceeds bounds".into());
        }
        let value = &packet[cursor..cursor + size];
        cursor += size;
        match bit {
            0x2 => plan.route = value.to_vec(),
            0x4 => plan.user = value.to_vec(),
            0x8 => plan.domain = value.to_vec(),
            0x10 => plan.password = value.to_vec(),
            0x8000 => plan.guid = value.to_vec(),
            _ => (),
        }
    }
    if length - cursor > 8 || plan.password.is_empty() {
        return Err("Invalid redirection payload".into());
    }
    plan.routing_token()?;
    plan.username()?;
    text(&plan.domain)?;
    text(&plan.guid)?;
    Ok(Some(plan))
}
pub async fn authenticate<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    plan: &Plan,
) -> Result<(), String> {
    let mut caps = [0u8; 8];
    stream
        .read_exact(&mut caps)
        .await
        .map_err(|_| "RDSTLS capabilities read failed")?;
    if caps[..6] != [1, 0, 1, 0, 1, 0] || u16::from_le_bytes([caps[6], caps[7]]) & 1 == 0 {
        return Err("Unsupported RDSTLS capabilities".into());
    }
    let request = plan.request()?;
    let result = stream.write_all(&request.0).await;
    result.map_err(|_| "RDSTLS authentication write failed")?;
    let mut response = [0u8; 10];
    stream
        .read_exact(&mut response)
        .await
        .map_err(|_| "RDSTLS authentication response failed")?;
    if response[..6] != [1, 0, 4, 0, 1, 0] || response[6..] != [0, 0, 0, 0] {
        return Err("RDSTLS handover authentication rejected".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unicode(s: &str) -> Vec<u8> {
        s.encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect()
    }
    fn packet() -> Vec<u8> {
        let flags = 0x2u32 | 0x4 | 0x10 | 0x4000 | 0x8000;
        let mut p = vec![0, 4, 0, 0];
        p.extend_from_slice(&42u32.to_le_bytes());
        p.extend_from_slice(&flags.to_le_bytes());
        for value in [
            b"12345".to_vec(),
            unicode("fixture-user"),
            vec![0xa5; 128],
            unicode("fixture-guid"),
        ] {
            p.extend_from_slice(&(value.len() as u32).to_le_bytes());
            p.extend_from_slice(&value);
        }
        let len = p.len() as u16;
        p[2..4].copy_from_slice(&len.to_le_bytes());
        let total = (p.len() + 8) as u16;
        let mut outer = total.to_le_bytes().to_vec();
        outer.extend_from_slice(&[0x1a, 0, 0, 0, 0, 0]);
        outer.extend(p);
        outer
    }
    #[test]
    fn same_endpoint_packet_is_bounded_and_translated_without_plain_password() {
        let data = packet();
        let p = parse(&data).unwrap().unwrap();
        assert_eq!(p.session, 42);
        assert_eq!(p.routing_token().unwrap(), "12345");
        assert_eq!(p.username().unwrap(), "fixture-user");
        let request = p.request().unwrap();
        assert_eq!(&request.0[..6], &[1, 0, 2, 0, 1, 0]);
        assert!(request.0.ends_with(&[0xa5; 128]));
        let mut p = p;
        p.route = b"Cookie: msts=12345\r\n".to_vec();
        assert_eq!(p.routing_token().unwrap(), "12345");
        p.route = b"Cookie: msts=12345\r\nInjected: bad".to_vec();
        assert!(p.routing_token().is_err());
    }
    #[test]
    fn malformed_destinations_lengths_plain_credentials_and_header_types_are_refused() {
        let good = packet();
        for cut in 0..good.len() {
            assert!(parse(&good[..cut]).is_err() || parse(&good[..cut]).unwrap().is_none());
        }
        for bit in [1u32, 0x100, 0x200, 0x800, 0x1000, 0x80] {
            let mut bad = good.clone();
            let flags = u32::from_le_bytes(bad[16..20].try_into().unwrap()) | bit;
            bad[16..20].copy_from_slice(&flags.to_le_bytes());
            assert!(parse(&bad).is_err());
        }
        let mut bad = good.clone();
        bad[17] &= !0x40;
        assert!(parse(&bad).is_err());
        let mut bad = good.clone();
        bad[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&bad).is_err());
    }
    #[test]
    fn rdstls_capabilities_and_result_are_checked_before_session_activation() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for accepted in [true, false] {
                let plan = parse(&packet()).unwrap().unwrap();
                let expected = plan.request().unwrap();
                let (mut client, mut server) = tokio::io::duplex(1024);
                let peer = tokio::spawn(async move {
                    server.write_all(&[1, 0, 1, 0, 1, 0, 3, 0]).await.unwrap();
                    let mut received = vec![0u8; expected.0.len()];
                    server.read_exact(&mut received).await.unwrap();
                    assert_eq!(received, expected.0);
                    let code = if accepted { 0u32 } else { 0x52e };
                    let mut response = vec![1, 0, 4, 0, 1, 0];
                    response.extend_from_slice(&code.to_le_bytes());
                    server.write_all(&response).await.unwrap();
                });
                assert_eq!(authenticate(&mut client, &plan).await.is_ok(), accepted);
                peer.await.unwrap();
            }
        });
    }
}
