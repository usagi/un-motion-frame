use un_motion_frame::UNMotionFrame;

/// Publisher が key expression を組み立てるときのモード。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicMode {
	/// すべてのフレームを単一 key に流す (`<base>/v<major>`)。
	Frame,
	/// 1 番目の `MotionSourceInfo.source_id` を末尾に付加する (`<base>/v<major>/<source>`)。
	/// source が無いときは `"unknown"`。
	ByPrimarySource,
	/// `MotionHeader.stream_id` を末尾に付加する (`<base>/v<major>/<stream>`)。
	/// stream_id が無いときは `"default"`。
	ByStreamId,
}

/// Zenoh の key expression 構築規約。
///
/// Publisher 側でフレームから key を組み立てるとともに、Subscriber 側で対応する
/// wildcard key を導出するためにも使う (互いに合意する規約)。
#[derive(Clone, Debug)]
pub struct ZenohTopicStrategy {
	/// 末尾 `/` を含まないベース key。例: `"un-motion/frame"`。
	pub base_key_expr: String,
	/// schema major version。topic レベルで major 互換版を分離する。
	pub schema_major: u16,
	/// key 構築モード。
	pub mode: TopicMode,
}

impl Default for ZenohTopicStrategy {
	fn default() -> Self {
		Self {
			base_key_expr: "un-motion/frame".to_string(),
			schema_major: crate::SCHEMA_MAJOR,
			mode: TopicMode::Frame,
		}
	}
}

impl ZenohTopicStrategy {
	pub fn new(base_key_expr: impl Into<String>, mode: TopicMode) -> Self {
		Self {
			base_key_expr: base_key_expr.into(),
			schema_major: crate::SCHEMA_MAJOR,
			mode,
		}
	}

	pub fn with_schema_major(mut self, schema_major: u16) -> Self {
		self.schema_major = schema_major;
		self
	}

	/// `<base>/v<major>` までの固定 prefix を返す。
	pub fn versioned_prefix(&self) -> String {
		let trimmed = self.base_key_expr.trim_end_matches('/');
		format!("{trimmed}/v{}", self.schema_major)
	}

	/// publish 1 フレームの key expression を組み立てる。
	pub fn key_expr_for_frame(&self, frame: &UNMotionFrame) -> String {
		let prefix = self.versioned_prefix();
		match self.mode {
			TopicMode::Frame => prefix,
			TopicMode::ByPrimarySource => {
				let segment = frame
					.sources
					.first()
					.map(|s| sanitize_segment(&s.source_id))
					.unwrap_or_else(|| "unknown".to_string());
				format!("{prefix}/{segment}")
			}
			TopicMode::ByStreamId => {
				let segment = frame
					.header
					.stream_id
					.as_deref()
					.map(sanitize_segment)
					.unwrap_or_else(|| "default".to_string());
				format!("{prefix}/{segment}")
			}
		}
	}

	/// Subscriber 側が一括 subscribe するための key expression を返す。
	///
	/// - `Frame`: `<base>/v<major>` を返す (完全一致)。
	/// - `ByPrimarySource` / `ByStreamId`: `<base>/v<major>/**` を返し、配下すべてを購読する。
	pub fn subscribe_key_expr(&self) -> String {
		let prefix = self.versioned_prefix();
		match self.mode {
			TopicMode::Frame => prefix,
			TopicMode::ByPrimarySource | TopicMode::ByStreamId => format!("{prefix}/**"),
		}
	}
}

/// Zenoh key segment として安全な文字列に変換する。
///
/// Zenoh では `/` がセグメント区切り、`*` / `**` が wildcard なので、source_id や stream_id に
/// それらが含まれていると意図しないトピックに流れる。ASCII 英数字と `-_.` 以外はすべて `_` に置換する。
/// 空文字列になった場合は `"unknown"` を返す。
pub fn sanitize_segment(input: &str) -> String {
	let mut out = String::with_capacity(input.len());
	for ch in input.chars() {
		if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
			out.push(ch);
		} else {
			out.push('_');
		}
	}
	if out.is_empty() { "unknown".to_string() } else { out }
}

#[cfg(test)]
mod tests {
	use super::*;
	use un_motion_frame::{MotionSourceInfo, MotionSourceKind, TrackingState};

	#[test]
	fn default_strategy_uses_v1_segment() {
		let strategy = ZenohTopicStrategy::default();
		assert_eq!(strategy.versioned_prefix(), "un-motion/frame/v1");
		assert_eq!(strategy.subscribe_key_expr(), "un-motion/frame/v1");
	}

	#[test]
	fn by_primary_source_appends_sanitized_source_id() {
		let strategy = ZenohTopicStrategy::new("un-motion/frame", TopicMode::ByPrimarySource);
		let mut frame = UNMotionFrame::new(1);
		frame.sources.push(MotionSourceInfo {
			source_id: "webcam:cam 0".to_string(),
			source_kind: MotionSourceKind::WebcamPose,
			display_name: None,
			confidence: 1.0,
			latency_ns: None,
			state: TrackingState::Valid,
		});

		assert_eq!(strategy.key_expr_for_frame(&frame), "un-motion/frame/v1/webcam_cam_0");
		assert_eq!(strategy.subscribe_key_expr(), "un-motion/frame/v1/**");
	}

	#[test]
	fn by_stream_id_uses_header_stream_id() {
		let strategy = ZenohTopicStrategy::new("un-motion/frame", TopicMode::ByStreamId);
		let mut frame = UNMotionFrame::new(1);
		frame.header.stream_id = Some("rt0".to_string());

		assert_eq!(strategy.key_expr_for_frame(&frame), "un-motion/frame/v1/rt0");
	}

	#[test]
	fn by_stream_id_falls_back_to_default_when_missing() {
		let strategy = ZenohTopicStrategy::new("un-motion/frame", TopicMode::ByStreamId);
		let frame = UNMotionFrame::new(1);

		assert_eq!(strategy.key_expr_for_frame(&frame), "un-motion/frame/v1/default");
	}

	#[test]
	fn sanitize_segment_replaces_unsafe_chars() {
		assert_eq!(sanitize_segment("a/b*c"), "a_b_c");
		assert_eq!(sanitize_segment(""), "unknown");
	}
}
