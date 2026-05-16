//! `InMemoryBackend` を使った Publisher → Subscriber の roundtrip 検証。
//!
//! 本物の Zenoh セッションを開かずに wire 規約 (`encode_frame` / `decode_frame` /
//! `ZenohTopicStrategy::key_expr_for_frame` / Subscriber 受信ハンドラ) を一気通貫で確認する。

use un_motion_frame::{MotionSourceInfo, MotionSourceKind, TrackingState, UNMotionFrame};
use un_motion_frame_zenoh::{
	InMemoryBackend, Publisher, Subscriber, TopicMode, ZenohTopicStrategy,
};

fn make_frame(seq: u64, stream_id: &str, source_id: &str) -> UNMotionFrame {
	let mut frame = UNMotionFrame::new(seq);
	frame.header.stream_id = Some(stream_id.to_string());
	frame.header.expected_dt_ns = Some(16_666_667);
	frame.sources.push(MotionSourceInfo {
		source_id: source_id.to_string(),
		source_kind: MotionSourceKind::WebcamPose,
		display_name: Some(source_id.to_string()),
		confidence: 1.0,
		latency_ns: None,
		state: TrackingState::Valid,
	});
	frame
}

#[test]
fn publisher_to_subscriber_default_topic_roundtrip() {
	let mut backend = InMemoryBackend::new();

	let strategy = ZenohTopicStrategy::default();
	let subscriber = Subscriber::declare(&mut backend, strategy.clone()).expect("subscriber declared");

	let mut publisher = Publisher::new(backend.clone()).with_strategy(strategy);
	let sent = make_frame(7, "rt0", "webcam:cam0");
	publisher.send(&sent).expect("send ok");

	let received = subscriber.try_recv_frame().expect("decode ok").expect("got frame");
	assert_eq!(received, sent);

	let published = backend.published();
	assert_eq!(published.len(), 1);
	assert_eq!(published[0].key_expr, "un-motion/frame/v1");
}

#[test]
fn publisher_to_subscriber_by_primary_source_uses_wildcard_subscribe() {
	let mut backend = InMemoryBackend::new();

	let strategy = ZenohTopicStrategy::new("un-motion/frame", TopicMode::ByPrimarySource);
	let subscriber = Subscriber::declare(&mut backend, strategy.clone()).expect("subscriber declared");
	assert_eq!(subscriber.strategy().subscribe_key_expr(), "un-motion/frame/v1/**");

	let mut publisher = Publisher::new(backend.clone()).with_strategy(strategy);
	let frame_a = make_frame(1, "rt0", "webcam:cam0");
	let frame_b = make_frame(2, "rt0", "webcam:cam1");
	publisher.send(&frame_a).expect("send a");
	publisher.send(&frame_b).expect("send b");

	let r_a = subscriber.try_recv_frame().expect("decode a").expect("got a");
	let r_b = subscriber.try_recv_frame().expect("decode b").expect("got b");
	assert_eq!(r_a.header.sequence, 1);
	assert_eq!(r_b.header.sequence, 2);

	let published = backend.published();
	assert_eq!(published[0].key_expr, "un-motion/frame/v1/webcam_cam0");
	assert_eq!(published[1].key_expr, "un-motion/frame/v1/webcam_cam1");
}

#[test]
fn publisher_to_subscriber_by_stream_id_routes_per_stream() {
	let mut backend = InMemoryBackend::new();

	let strategy = ZenohTopicStrategy::new("un-motion/frame", TopicMode::ByStreamId);
	let subscriber = Subscriber::declare(&mut backend, strategy.clone()).expect("subscriber declared");

	let mut publisher = Publisher::new(backend.clone()).with_strategy(strategy);
	let frame_a = make_frame(11, "rt0", "webcam:cam0");
	let frame_b = make_frame(12, "rt1", "webcam:cam0");
	publisher.send(&frame_a).expect("send a");
	publisher.send(&frame_b).expect("send b");

	let r_a = subscriber.try_recv_frame().expect("decode a").expect("got a");
	let r_b = subscriber.try_recv_frame().expect("decode b").expect("got b");
	assert_eq!(r_a.header.stream_id.as_deref(), Some("rt0"));
	assert_eq!(r_b.header.stream_id.as_deref(), Some("rt1"));

	let published = backend.published();
	assert_eq!(published[0].key_expr, "un-motion/frame/v1/rt0");
	assert_eq!(published[1].key_expr, "un-motion/frame/v1/rt1");
}

#[test]
fn replay_backend_delivers_queued_frames() {
	let mut backend = un_motion_frame_zenoh::ReplayBackend::new();
	let frame_a = make_frame(101, "rep", "replay:cam0");
	let frame_b = make_frame(102, "rep", "replay:cam0");
	backend.push_frame("un-motion/frame/v1", &frame_a).unwrap();
	backend.push_frame("un-motion/frame/v1", &frame_b).unwrap();

	let subscriber = Subscriber::declare(&mut backend, ZenohTopicStrategy::default()).expect("subscriber declared");

	let r_a = subscriber.try_recv_frame().expect("decode a").expect("got a");
	let r_b = subscriber.try_recv_frame().expect("decode b").expect("got b");
	assert_eq!(r_a, frame_a);
	assert_eq!(r_b, frame_b);

	// declare 後に push_frame したものも live で届く。
	let frame_c = make_frame(103, "rep", "replay:cam0");
	backend.push_frame("un-motion/frame/v1", &frame_c).unwrap();
	let r_c = subscriber.try_recv_frame().expect("decode c").expect("got c");
	assert_eq!(r_c, frame_c);
}
