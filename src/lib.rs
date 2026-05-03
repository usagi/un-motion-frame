//! UNMotion、UN Avatar、関連ツールで共有するトランスポート非依存の
//! モーションフレームスキーマ。
//!
//! データモデルは意図的にトランスポート中立にしている。JSON、JSONL、
//! MessagePack、CBOR、Zenoh、UDP adapter、in-process buffer のいずれでも
//! フレームの意味は同じである。wire と retargeting の規約は `docs/schema.md`。

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Vec2f {
	pub x: f32,
	pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Vec3f {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Quatf {
	pub x: f32,
	pub y: f32,
	pub z: f32,
	pub w: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TimestampBasis {
	UnixEpoch,
	Monotonic,
	SourceLocal,
	#[default]
	Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum CoordinateSpace {
	Camera,
	NormalizedImage,
	ModelLocal,
	ModelWorld,
	Vmc,
	UNMotion,
	#[default]
	Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Handedness {
	LeftHanded,
	RightHanded,
	#[default]
	Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum LengthUnit {
	Meter,
	Centimeter,
	Millimeter,
	Normalized,
	#[default]
	Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TrackingState {
	Valid,
	Partial,
	Lost,
	Recovering,
	Stale,
	#[default]
	Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SampleState {
	Valid,
	Inferred,
	Held,
	Decayed,
	#[default]
	Missing,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MotionHeader {
	pub magic: [u8; 4],
	pub version_major: u16,
	pub version_minor: u16,
	pub sequence: u64,
	pub timestamp_basis: TimestampBasis,
	pub capture_timestamp_ns: u64,
	pub frame_timestamp_ns: u64,
	pub processed_timestamp_ns: u64,
	pub coordinate_space: CoordinateSpace,
	pub handedness: Handedness,
	pub length_unit: LengthUnit,
}

impl MotionHeader {
	pub const MAGIC: [u8; 4] = *b"UNMF";

	pub fn new(sequence: u64) -> Self {
		Self {
			magic: Self::MAGIC,
			version_major: 1,
			version_minor: 0,
			sequence,
			timestamp_basis: TimestampBasis::Unknown,
			capture_timestamp_ns: 0,
			frame_timestamp_ns: 0,
			processed_timestamp_ns: 0,
			coordinate_space: CoordinateSpace::Unknown,
			handedness: Handedness::Unknown,
			length_unit: LengthUnit::Unknown,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MotionSourceKind {
	WebcamPose,
	ImagePose,
	VideoPose,
	VmcInput,
	FaceId,
	Manual,
	Dummy,
	Replay,
	Unknown,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MotionSourceInfo {
	pub source_id: String,
	pub source_kind: MotionSourceKind,
	pub display_name: Option<String>,
	pub confidence: f32,
	pub latency_ns: Option<u64>,
	pub state: TrackingState,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TransformSample {
	pub translation: Option<Vec3f>,
	pub rotation: Option<Quatf>,
	pub scale: Option<Vec3f>,
	pub linear_velocity: Option<Vec3f>,
	pub angular_velocity: Option<Vec3f>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum HumanoidBone {
	Hips = 0,
	Spine = 1,
	Chest = 2,
	UpperChest = 3,
	Neck = 4,
	Head = 5,
	LeftShoulder = 6,
	LeftUpperArm = 7,
	LeftLowerArm = 8,
	LeftHand = 9,
	RightShoulder = 10,
	RightUpperArm = 11,
	RightLowerArm = 12,
	RightHand = 13,
	LeftUpperLeg = 14,
	LeftLowerLeg = 15,
	LeftFoot = 16,
	LeftToes = 17,
	RightUpperLeg = 18,
	RightLowerLeg = 19,
	RightFoot = 20,
	RightToes = 21,
	LeftEye = 22,
	RightEye = 23,
	Jaw = 24,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BoneSample {
	pub bone: HumanoidBone,
	pub transform: TransformSample,
	pub confidence: f32,
	pub source_index: Option<u16>,
	pub state: SampleState,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct HumanoidPose {
	pub root: Option<TransformSample>,
	pub bones: Vec<BoneSample>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BodyMotion {
	pub tracking_state: TrackingState,
	pub confidence: f32,
	pub humanoid: Option<HumanoidPose>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ExpressionSample {
	pub name: String,
	pub value: f32,
	pub confidence: f32,
	pub source_index: Option<u16>,
	#[cfg_attr(feature = "serde", serde(default))]
	pub state: SampleState,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FaceMotion {
	pub tracking_state: TrackingState,
	pub confidence: f32,
	pub head: Option<TransformSample>,
	pub expressions: Vec<ExpressionSample>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct GazeSample {
	pub origin: Option<Vec3f>,
	pub direction: Vec3f,
	pub target: Option<Vec3f>,
	pub confidence: f32,
	pub source_index: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EyeMotion {
	pub tracking_state: TrackingState,
	pub confidence: f32,
	pub left_gaze: Option<GazeSample>,
	pub right_gaze: Option<GazeSample>,
	pub combined_gaze: Option<GazeSample>,
	pub blink_left: Option<f32>,
	pub blink_right: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Finger {
	Thumb,
	Index,
	Middle,
	Ring,
	Little,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FingerPose {
	pub finger: Finger,
	pub joints: Vec<TransformSample>,
	pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct HandMotion {
	pub tracking_state: TrackingState,
	pub confidence: f32,
	pub wrist: Option<TransformSample>,
	pub fingers: Vec<FingerPose>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MotionSignalValue {
	Bool(bool),
	Scalar(f32),
	Vec2(Vec2f),
	Vec3(Vec3f),
	Quat(Quatf),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MotionSignal {
	pub name: String,
	pub value: MotionSignalValue,
	pub confidence: f32,
	pub source_index: Option<u16>,
	pub state: SampleState,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MotionExtension {
	pub namespace: String,
	pub name: String,
	pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct MotionMetadata {
	pub schema_name: String,
	pub schema_version: String,
	pub producer: Option<String>,
	pub notes: Vec<String>,
	pub extensions: Vec<MotionExtension>,
}

impl Default for MotionMetadata {
	fn default() -> Self {
		Self {
			schema_name: "UNMotionFrame".to_string(),
			schema_version: "1.0.0".to_string(),
			producer: None,
			notes: Vec::new(),
			extensions: Vec::new(),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct UNMotionFrame {
	pub header: MotionHeader,
	pub sources: Vec<MotionSourceInfo>,
	pub body: Option<BodyMotion>,
	pub face: Option<FaceMotion>,
	pub eyes: Option<EyeMotion>,
	pub left_hand: Option<HandMotion>,
	pub right_hand: Option<HandMotion>,
	pub signals: Vec<MotionSignal>,
	pub metadata: MotionMetadata,
}

impl UNMotionFrame {
	pub fn new(sequence: u64) -> Self {
		Self {
			header: MotionHeader::new(sequence),
			sources: Vec::new(),
			body: None,
			face: None,
			eyes: None,
			left_hand: None,
			right_hand: None,
			signals: Vec::new(),
			metadata: MotionMetadata::default(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn header_magic_is_unmf() {
		let header = MotionHeader::new(42);
		assert_eq!(header.magic, *b"UNMF");
	}

	#[cfg(feature = "serde")]
	#[test]
	fn frame_roundtrip_json() {
		let mut frame = UNMotionFrame::new(7);
		frame.sources.push(MotionSourceInfo {
			source_id: "dummy".to_string(),
			source_kind: MotionSourceKind::Dummy,
			display_name: Some("Dummy Engine".to_string()),
			confidence: 1.0,
			latency_ns: Some(1_000_000),
			state: TrackingState::Valid,
		});
		frame.signals.push(MotionSignal {
			name: "head.yaw".to_string(),
			value: MotionSignalValue::Scalar(0.25),
			confidence: 0.95,
			source_index: Some(0),
			state: SampleState::Valid,
		});

		let json = serde_json::to_string(&frame).expect("serialize frame");
		let decoded: UNMotionFrame = serde_json::from_str(&json).expect("deserialize frame");

		assert_eq!(decoded.header.sequence, 7);
		assert_eq!(decoded.sources.len(), 1);
		assert_eq!(decoded.signals.len(), 1);
		assert_eq!(decoded.metadata.schema_name, "UNMotionFrame");
		assert_eq!(decoded.metadata.schema_version, "1.0.0");
		assert_eq!(decoded, frame);
	}

	#[cfg(feature = "serde")]
	#[test]
	fn expression_sample_defaults_state_for_older_json() {
		let json = r#"{
			"name": "vmc:Joy",
			"value": 0.5,
			"confidence": 1.0,
			"source_index": null
		}"#;

		let decoded: ExpressionSample = serde_json::from_str(json).expect("deserialize expression sample");

		assert_eq!(decoded.name, "vmc:Joy");
		assert_eq!(decoded.state, SampleState::Missing);
	}

	#[cfg(feature = "serde")]
	#[test]
	fn metadata_defaults_for_older_json() {
		let decoded: MotionMetadata = serde_json::from_str(r#"{"notes":["legacy"]}"#).expect("deserialize metadata");

		assert_eq!(decoded.schema_name, "UNMotionFrame");
		assert_eq!(decoded.schema_version, "1.0.0");
		assert_eq!(decoded.notes, vec!["legacy"]);
		assert!(decoded.extensions.is_empty());
	}
}
