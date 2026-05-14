use std::time::Duration;

use crossbeam_channel::Receiver;
use un_motion_frame::UNMotionFrame;

use crate::{Error, ZenohTopicStrategy, decode_frame};

/// Subscriber が受信した 1 メッセージ。Subscriber は通常これをすぐに `UNMotionFrame` へデコードする。
#[derive(Clone, Debug)]
pub struct ReceivedMessage {
	pub key_expr: String,
	pub payload: Vec<u8>,
}

/// バックエンドが返す subscription の「生存ハンドル」と receiver を 1 つにまとめたもの。
///
/// `keep_alive` は Zenoh subscriber 等を drop されないように保持するための任意の型。
/// Subscriber 側はこれを掴んでおく以外の用途では参照しない。
pub struct SubscriptionHandle {
	pub rx: Receiver<ReceivedMessage>,
	pub keep_alive: Box<dyn std::any::Any + Send>,
}

impl SubscriptionHandle {
	pub fn new(rx: Receiver<ReceivedMessage>, keep_alive: Box<dyn std::any::Any + Send>) -> Self {
		Self { rx, keep_alive }
	}

	pub fn from_rx(rx: Receiver<ReceivedMessage>) -> Self {
		Self {
			rx,
			keep_alive: Box::new(()),
		}
	}
}

/// Subscriber のバックエンドが満たす最低限の契約。
pub trait SubscriberBackend {
	fn declare_frame_subscriber(&mut self, key_expr: &str) -> Result<SubscriptionHandle, Error>;
}

/// UNMotionFrame を Zenoh (またはモック) から受信するエントリーポイント。
///
/// 内部に [`crossbeam_channel::Receiver`] を持つので、Subscriber を `Send` で別スレッドへ移して
/// `try_recv_frame` / `recv_frame_timeout` でポーリングするか、`rx()` を取り出して
/// `crossbeam_channel::select!` に組み込むかが想定 usage。
pub struct Subscriber {
	rx: Receiver<ReceivedMessage>,
	_keep_alive: Box<dyn std::any::Any + Send>,
	strategy: ZenohTopicStrategy,
}

impl Subscriber {
	pub fn declare<B: SubscriberBackend>(backend: &mut B, strategy: ZenohTopicStrategy) -> Result<Self, Error> {
		let handle = backend.declare_frame_subscriber(&strategy.subscribe_key_expr())?;
		Ok(Self {
			rx: handle.rx,
			_keep_alive: handle.keep_alive,
			strategy,
		})
	}

	/// 既存の receiver を直接受け取って Subscriber に変換する (テスト用に主に使う)。
	pub fn from_handle(handle: SubscriptionHandle, strategy: ZenohTopicStrategy) -> Self {
		Self {
			rx: handle.rx,
			_keep_alive: handle.keep_alive,
			strategy,
		}
	}

	pub fn strategy(&self) -> &ZenohTopicStrategy {
		&self.strategy
	}

	pub fn rx(&self) -> &Receiver<ReceivedMessage> {
		&self.rx
	}

	/// 未処理メッセージを 1 件だけ取り出す。空ならただちに `Ok(None)`。
	pub fn try_recv_message(&self) -> Option<ReceivedMessage> {
		self.rx.try_recv().ok()
	}

	/// 未処理メッセージを 1 件取り出し、UNMotionFrame までデコードする。空なら `Ok(None)`。
	pub fn try_recv_frame(&self) -> Result<Option<UNMotionFrame>, Error> {
		match self.rx.try_recv() {
			Ok(msg) => decode_frame(&msg.payload).map(Some),
			Err(_) => Ok(None),
		}
	}

	/// 指定時間まで待って 1 件取り出す。timeout で `Ok(None)`。
	pub fn recv_frame_timeout(&self, timeout: Duration) -> Result<Option<UNMotionFrame>, Error> {
		match self.rx.recv_timeout(timeout) {
			Ok(msg) => decode_frame(&msg.payload).map(Some),
			Err(_) => Ok(None),
		}
	}
}

#[cfg(feature = "zenoh-transport")]
pub use real::ZenohSubscriberBackend;

#[cfg(feature = "zenoh-transport")]
mod real {
	use zenoh::{Config, Session, Wait};

	use crate::Error;
	use super::{ReceivedMessage, SubscriberBackend, SubscriptionHandle};

	/// 実 Zenoh セッションをラップした Subscriber バックエンド。
	pub struct ZenohSubscriberBackend {
		session: Session,
	}

	impl ZenohSubscriberBackend {
		pub fn open_default() -> Result<Self, Error> {
			let session = zenoh::open(Config::default())
				.wait()
				.map_err(|e| Error::transport(format!("zenoh open failed: {e}")))?;
			Ok(Self { session })
		}

		pub fn from_session(session: Session) -> Self {
			Self { session }
		}

		pub fn session(&self) -> &Session {
			&self.session
		}
	}

	impl core::fmt::Debug for ZenohSubscriberBackend {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.debug_struct("ZenohSubscriberBackend").finish_non_exhaustive()
		}
	}

	impl SubscriberBackend for ZenohSubscriberBackend {
		fn declare_frame_subscriber(&mut self, key_expr: &str) -> Result<SubscriptionHandle, Error> {
			let (tx, rx) = crossbeam_channel::unbounded::<ReceivedMessage>();
			let subscriber = self
				.session
				.declare_subscriber(key_expr.to_string())
				.callback(move |sample| {
					let key = sample.key_expr().as_str().to_string();
					let payload = sample.payload().to_bytes().to_vec();
					// 受信側が drop されたあとは送信失敗するが、その場合は黙って捨てる
					// (Subscriber が drop された後の Zenoh からのコールバックは正常)。
					let _ = tx.send(ReceivedMessage { key_expr: key, payload });
				})
				.wait()
				.map_err(|e| Error::transport(format!("zenoh declare_subscriber failed on {key_expr}: {e}")))?;

			Ok(SubscriptionHandle::new(rx, Box::new(subscriber)))
		}
	}
}
