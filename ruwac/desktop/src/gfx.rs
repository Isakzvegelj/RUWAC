//! GNOME graphics channel: upstream decoding/composition into the canonical RDP image.
use ironrdp::{
    dvc::{DvcClientProcessor, DvcMessage, DvcProcessor},
    pdu::{PduResult, geometry::InclusiveRectangle},
    session::image::DecodedImage,
};
use ironrdp_egfx::{
    client::{GraphicsPipelineClient, GraphicsPipelineHandler},
    compositor::OutputUpdate,
};
use std::sync::{Arc, Mutex};

pub const MAX_WIRE: usize = 64 * 1024 * 1024 + 65536;
/// Source locations identify decoder failures without formatting peer data or credentials.
pub fn decoder_location(error: &ironrdp::session::SessionError) -> String {
    let mut parts = Vec::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    for _ in 0..8 {
        let Some(e) = current else { break };
        if let Some(e) = e.downcast_ref::<ironrdp::graphics::progressive::ProgressiveDecodeError>()
        {
            use ironrdp::graphics::progressive::ProgressiveDecodeError as E;
            parts.push(
                match e {
                    E::Pdu(_) => "Progressive: malformed PDU",
                    E::Rlgr(_) => "Progressive: RLGR stream",
                    E::Srl(_) => "Progressive: SRL stream",
                    E::MissingBlock(_) => "Progressive: required block missing",
                    E::TileOutOfBounds { .. } => "Progressive: tile outside surface",
                    E::InvalidQuantIndex { .. } => "Progressive: quantization index",
                    E::MissingTileReference { .. } => "Progressive: missing reference tile",
                    E::SurfaceTooLarge { .. } => "Progressive: surface exceeds limit",
                }
                .into(),
            );
        }
        if let Some(e) = e.downcast_ref::<ironrdp::graphics::srl::SrlError>() {
            use ironrdp::graphics::srl::SrlError as E;
            parts.push(
                match e {
                    E::MissingTerminator => "SRL: missing terminator",
                    E::Truncated => "SRL: truncated bit stream",
                    E::InvalidBitCount(_) => "SRL: invalid magnitude width",
                    E::MagnitudeOutOfRange { .. } => "SRL: magnitude outside width",
                    E::ZeroRunTooLong => "SRL: zero run exceeds component",
                }
                .into(),
            );
        }
        let location = if let Some(e) = e.downcast_ref::<ironrdp::session::SessionError>() {
            Some(e.location())
        } else if let Some(e) = e.downcast_ref::<ironrdp::pdu::PduError>() {
            Some(e.location())
        } else if let Some(e) = e.downcast_ref::<ironrdp_core::DecodeError>() {
            if let ironrdp_core::DecodeErrorKind::InvalidField { field, reason } = e.kind() {
                // Both values have &'static str types in this API; never format peer-owned strings.
                parts.push(format!("{field}: {reason}"));
            }
            Some(e.location())
        } else {
            None
        };
        if let Some(l) = location {
            let file = std::path::Path::new(l.file())
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("decoder");
            parts.push(format!("{file}:{}", l.line()));
        }
        current = e.source();
    }
    format!("RDP decode failed ({})", parts.join(" → "))
}
#[derive(Default)]
pub struct Pending {
    pub reset: Option<(u16, u16)>,
    updates: Vec<OutputUpdate>,
    closed: bool,
}
pub type Shared = Arc<Mutex<Pending>>;
struct Handler;
impl GraphicsPipelineHandler for Handler {
    fn capabilities(&self) -> Vec<ironrdp_egfx::pdu::CapabilitySet> {
        use ironrdp_egfx::pdu::{CapabilitiesV107Flags, CapabilitySet};
        vec![CapabilitySet::V10_7 {
            flags: CapabilitiesV107Flags::AVC_DISABLED,
        }]
    }
}
pub struct Channel {
    client: GraphicsPipelineClient,
    output: Shared,
}
impl Channel {
    pub fn new(output: Shared) -> Self {
        Self {
            client: GraphicsPipelineClient::new(Box::new(Handler), None),
            output,
        }
    }
}
ironrdp_core::impl_as_any!(Channel);
impl DvcClientProcessor for Channel {}
impl DvcProcessor for Channel {
    fn channel_name(&self) -> &str {
        ironrdp_egfx::CHANNEL_NAME
    }
    fn start(&mut self, id: u32) -> PduResult<Vec<DvcMessage>> {
        self.client.start(id)
    }
    fn close(&mut self, id: u32) {
        self.client.close(id);
        self.output.lock().unwrap().closed = true;
    }
    fn process(&mut self, id: u32, data: &[u8]) -> PduResult<Vec<DvcMessage>> {
        if data.len() > MAX_WIRE {
            return Err(ironrdp::pdu::pdu_other_err!(
                "EGFX",
                "Wire message exceeds bounds"
            ));
        }
        let response = self.client.process(id, data)?;
        let mut out = self.output.lock().unwrap();
        if let Some((w, h)) = self.client.take_output_reset() {
            if w == 0 || h == 0 || w > 4096 || h > 4096 {
                return Err(ironrdp::pdu::pdu_other_err!(
                    "EGFX",
                    "Output exceeds framebuffer limits"
                ));
            }
            out.reset = Some((w, h));
            out.updates.clear();
        }
        out.updates.extend(self.client.drain_output());
        if out.updates.len() > 4096
            || out.updates.iter().map(|u| u.data.len()).sum::<usize>() > 64 * 1024 * 1024
        {
            return Err(ironrdp::pdu::pdu_other_err!(
                "EGFX",
                "Output queue exceeds bounds"
            ));
        }
        Ok(response)
    }
}
pub fn apply(shared: &Shared, image: &mut DecodedImage) -> Result<(bool, bool), String> {
    let mut out = shared.lock().unwrap();
    if out.closed {
        return Err("RDP graphics channel closed".into());
    }
    let reset = out.reset.take();
    if let Some((w, h)) = reset {
        *image = DecodedImage::new(
            ironrdp::graphics::image_processing::PixelFormat::BgrX32,
            w,
            h,
        );
    }
    let changed = !out.updates.is_empty();
    for update in out.updates.drain(..) {
        let r = update.region;
        if r.right <= r.left
            || r.bottom <= r.top
            || r.right > image.width()
            || r.bottom > image.height()
            || update.data.len()
                != usize::from(r.right - r.left) * usize::from(r.bottom - r.top) * 4
        {
            return Err("Invalid graphics output tile".into());
        }
        let rect = InclusiveRectangle {
            left: r.left,
            top: r.top,
            right: r.right - 1,
            bottom: r.bottom - 1,
        };
        image
            .apply_rgba32(&update.data, &rect, false)
            .map_err(|_| "Graphics tile application failed")?;
    }
    Ok((reset.is_some(), changed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironrdp::pdu::geometry::ExclusiveRectangle;
    use ironrdp_egfx::pdu::*;
    #[test]
    fn microsoft_clearcodec_short_bar_wire_and_srl_end_chunk() {
        // MS-RDPEGFX Example 4: little-endian 00 0f means YOn=0,YOff=15.
        let mut band = vec![0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 0, 0, 15];
        band.extend_from_slice(&[11, 22, 33].repeat(15));
        let parsed = ironrdp::pdu::codecs::clearcodec::decode_bands_layer(&band).unwrap();
        match &parsed[0].vbars[0] {
            ironrdp::pdu::codecs::clearcodec::VBar::ShortCacheMiss(v) => {
                assert_eq!(v.y_on, 0);
                assert_eq!(v.y_off_delta, 15);
            }
            _ => panic!("expected inline short bar"),
        }
        let cache_hit = [0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0x42, 0x40, 1];
        let parsed = ironrdp::pdu::codecs::clearcodec::decode_bands_layer(&cache_hit).unwrap();
        assert!(matches!(
            parsed[0].vbars[0],
            ironrdp::pdu::codecs::clearcodec::VBar::ShortCacheHit {
                index: 0x42,
                y_on: 1
            }
        ));
        assert_eq!(
            ironrdp::graphics::srl::decode_srl(&[0, 0], 60, 1).unwrap(),
            vec![0; 60]
        );
    }
    fn send(channel: &mut Channel, pdu: GfxPdu) {
        let raw = ironrdp_core::encode_vec(&pdu).unwrap();
        let wire = ironrdp::graphics::zgfx::wrap_uncompressed(&raw);
        channel.process(1, &wire).unwrap();
    }
    #[test]
    fn graphics_wire_clearcodec_composition_reaches_canonical_image_without_row_flip() {
        let shared = Arc::new(Mutex::new(Pending::default()));
        let mut channel = Channel::new(shared.clone());
        channel.start(1).unwrap();
        send(
            &mut channel,
            GfxPdu::CapabilitiesConfirm(CapabilitiesConfirmPdu::from_typed(
                &CapabilitySet::V10_7 {
                    flags: CapabilitiesV107Flags::AVC_DISABLED,
                },
            )),
        );
        send(
            &mut channel,
            GfxPdu::ResetGraphics(ResetGraphicsPdu {
                width: 20,
                height: 10,
                monitors: vec![],
            }),
        );
        send(
            &mut channel,
            GfxPdu::CreateSurface(CreateSurfacePdu {
                surface_id: 1,
                width: 4,
                height: 2,
                pixel_format: PixelFormat::XRgb,
            }),
        );
        send(
            &mut channel,
            GfxPdu::MapSurfaceToOutput(MapSurfaceToOutputPdu {
                surface_id: 1,
                output_origin_x: 3,
                output_origin_y: 4,
            }),
        );
        let bgra: Vec<u8> = (0..8u8).flat_map(|i| [i, 40 + i, 80 + i, 255]).collect();
        let encoded = ironrdp::graphics::clearcodec::ClearCodecEncoder::new().encode(&bgra, 4, 2);
        send(
            &mut channel,
            GfxPdu::WireToSurface1(WireToSurface1Pdu {
                surface_id: 1,
                codec_id: Codec1Type::ClearCodec,
                pixel_format: PixelFormat::XRgb,
                destination_rectangle: ExclusiveRectangle {
                    left: 0,
                    top: 0,
                    right: 4,
                    bottom: 2,
                },
                bitmap_data: encoded,
            }),
        );
        send(&mut channel, GfxPdu::EndFrame(EndFramePdu { frame_id: 1 }));
        let mut image = DecodedImage::new(
            ironrdp::graphics::image_processing::PixelFormat::BgrX32,
            1,
            1,
        );
        assert_eq!(apply(&shared, &mut image).unwrap(), (true, true));
        for y in 0..2usize {
            let offset = ((y + 4) * 20 + 3) * 4;
            assert_eq!(
                &image.data()[offset..offset + 16],
                &bgra[y * 16..(y + 1) * 16]
            );
        }
        assert_eq!(apply(&shared, &mut image).unwrap(), (false, false));
        channel.close(1);
        assert!(apply(&shared, &mut image).is_err());
    }
    #[test]
    fn rgba_public_bridge_rejects_invalid_payload_before_pixel_writes() {
        let mut image = DecodedImage::new(
            ironrdp::graphics::image_processing::PixelFormat::BgrX32,
            4,
            4,
        );
        let before = image.data().to_vec();
        let rect = InclusiveRectangle {
            left: 1,
            top: 1,
            right: 2,
            bottom: 2,
        };
        assert!(image.apply_rgba32(&[0; 15], &rect, false).is_err());
        assert_eq!(image.data(), before);
    }
    #[test]
    fn caps_do_not_claim_h264_and_oversize_output_is_refused() {
        let shared = Arc::new(Mutex::new(Pending::default()));
        let mut channel = Channel::new(shared);
        let caps = Handler.capabilities();
        assert!(
            matches!(caps.as_slice(),[CapabilitySet::V10_7{flags}]if flags.contains(CapabilitiesV107Flags::AVC_DISABLED))
        );
        channel.start(1).unwrap();
        let pdu = GfxPdu::ResetGraphics(ResetGraphicsPdu {
            width: 5000,
            height: 1,
            monitors: vec![],
        });
        let raw = ironrdp_core::encode_vec(&pdu).unwrap();
        assert!(
            channel
                .process(1, &ironrdp::graphics::zgfx::wrap_uncompressed(&raw))
                .is_err()
        );
    }
}
