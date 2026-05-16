use crossbeam_channel::Sender;
use un_motion_frame::UNMotionFrame;

use crate::{
	Error, encode_frame,
	subscriber::{ReceivedMessage, SubscriberBackend, SubscriptionHandle},
};

/// あらかじめ用意したフレーム列を Subscriber に流し込む、テスト/デバッグ向けバックエンド。
///
/// `declare_frame_subscriber` が呼ばれた瞬間にキュー全件を一度に送る素朴な実装。
/// 「再生開始のタイミング制御」「実時間に合わせた間隔」などが欲しい場合は呼び出し側で
/// `push_frame` を時間差で呼ぶか、`take_sender` で `Sender` を取り出して任意のタイミングで送る。
#[derive(Default)]
pub struct ReplayBackend {
	queued: Vec<(String, Vec<u8>)>,
	live_sender: Option<Sender<ReceivedMessage>>,
}

impl ReplayBackend {
	pub fn new() -> Self {
		Self::default()
	}

	/// MessagePack エンコードを介してフレームを 1 つキューに積む。
	pub fn push_frame(&mut self, key_expr: impl Into<String>, frame: &UNMotionFrame) -> Result<(), Error> {
		let payload = encode_frame(frame)?;
		self.push_payload(key_expr, payload);
		Ok(())
	}

	/// 既にエンコード済みの payload を積む。
	pub fn push_payload(&mut self, key_expr: impl Into<String>, payload: Vec<u8>) {
		let key = key_expr.into();
		let message = ReceivedMessage {
			key_expr: key.clone(),
			payload: payload.clone(),
		};
		if let Some(sender) = &self.live_sender {
			if sender.send(message).is_ok() {
				return;
			}
			self.live_sender = None;
		}
		self.queued.push((key, payload));
	}
}

impl core::fmt::Debug for ReplayBackend {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("ReplayBackend")
			.field("queued", &self.queued.len())
			.field("live", &self.live_sender.is_some())
			.finish()
	}
}

impl SubscriberBackend for ReplayBackend {
	fn declare_frame_subscriber(&mut self, _key_expr: &str) -> Result<SubscriptionHandle, Error> {
		let (tx, rx) = crossbeam_channel::unbounded::<ReceivedMessage>();
		for (key, payload) in self.queued.drain(..) {
			tx
				.send(ReceivedMessage { key_expr: key, payload })
				.map_err(|e| Error::transport(format!("replay send failed: {e}")))?;
		}
		self.live_sender = Some(tx);
		Ok(SubscriptionHandle::from_rx(rx))
	}
}
