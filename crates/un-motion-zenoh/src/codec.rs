use un_motion_frame::UNMotionFrame;

use crate::Error;

/// `UNMotionFrame` を MessagePack バイト列にエンコードする。
///
/// `un-motion-zenoh` の wire 規約上、Publisher 側はこの関数経由で payload を作る。
pub fn encode_frame(frame: &UNMotionFrame) -> Result<Vec<u8>, Error> {
	rmp_serde::to_vec(frame).map_err(Error::from)
}

/// MessagePack バイト列を `UNMotionFrame` にデコードする。
///
/// Subscriber 側はこの関数経由で payload を解釈する。
pub fn decode_frame(payload: &[u8]) -> Result<UNMotionFrame, Error> {
	rmp_serde::from_slice(payload).map_err(Error::from)
}

#[cfg(test)]
mod tests {
	use super::*;
	use un_motion_frame::{MotionSourceInfo, MotionSourceKind, TrackingState};

	#[test]
	fn encode_then_decode_preserves_frame() {
		let mut frame = UNMotionFrame::new(11);
		frame.header.stream_id = Some("stream-A".to_string());
		frame.header.expected_dt_ns = Some(33_333_333);
		frame.sources.push(MotionSourceInfo {
			source_id: "vmc:Waidayo".to_string(),
			source_kind: MotionSourceKind::VmcInput,
			display_name: Some("Waidayo".to_string()),
			confidence: 1.0,
			latency_ns: None,
			state: TrackingState::Valid,
		});

		let payload = encode_frame(&frame).expect("encode ok");
		let decoded = decode_frame(&payload).expect("decode ok");

		assert_eq!(decoded, frame);
	}
}
