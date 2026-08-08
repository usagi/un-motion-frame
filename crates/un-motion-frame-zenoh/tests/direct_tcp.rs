#![cfg(feature = "zenoh-transport")]

use std::net::TcpListener;
use std::time::Duration;

use un_motion_frame::UNMotionFrame;
use un_motion_frame_zenoh::{Publisher, Subscriber, ZenohSessionBackend, ZenohSessionConfig, ZenohSubscriberBackend, ZenohTopicStrategy};

#[test]
fn subscriber_connects_to_explicit_publisher_endpoint() {
	let port = reserve_tcp_port();
	let endpoint = format!("tcp/127.0.0.1:{port}");

	let publisher_config = ZenohSessionConfig::default()
		.with_listen_endpoint(endpoint.clone())
		.with_multicast_scouting(false);
	let publisher_backend = ZenohSessionBackend::open(&publisher_config).expect("open publisher");
	let mut publisher = Publisher::new(publisher_backend);

	let subscriber_config = ZenohSessionConfig::default()
		.with_connect_endpoint(endpoint)
		.with_multicast_scouting(false);
	let mut subscriber_backend = ZenohSubscriberBackend::open(&subscriber_config).expect("open subscriber");
	let subscriber = Subscriber::declare(&mut subscriber_backend, ZenohTopicStrategy::default()).expect("declare subscriber");

	let mut received = None;
	for sequence in 1..=20 {
		publisher.send(&UNMotionFrame::new(sequence)).expect("publish frame");
		if let Some(frame) = subscriber.recv_frame_timeout(Duration::from_millis(100)).expect("receive frame") {
			received = Some(frame);
			break;
		}
	}
	let frame = received.expect("frame before retry budget expires");
	assert!((1..=20).contains(&frame.header.sequence));
}

fn reserve_tcp_port() -> u16 {
	let listener = TcpListener::bind("127.0.0.1:0").expect("reserve TCP port");
	listener.local_addr().expect("reserved address").port()
}
