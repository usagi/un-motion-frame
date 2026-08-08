use un_motion_frame::UNMotionFrame;

use crate::{Error, ZenohTopicStrategy, encode_frame};

/// Publisher のバックエンドが満たす最低限の契約。
///
/// 「指定された key に payload を put する」だけを抽象化することで、Zenoh セッションを実際に
/// 開いた状態と、テスト用 InMemory バックエンドを同じコードで使い分けられるようにする。
pub trait PublisherBackend: Send {
	fn put(&mut self, key_expr: &str, payload: &[u8]) -> Result<(), Error>;
}

/// UNMotionFrame を Zenoh (またはモック) に publish するエントリーポイント。
///
/// `Publisher` 自体は backend (`PublisherBackend`) と key 構築規約 (`ZenohTopicStrategy`)
/// を保持するだけで、本物の Zenoh セッションは [`ZenohSessionBackend`] が握っている。
pub struct Publisher<B: PublisherBackend> {
	backend: B,
	strategy: ZenohTopicStrategy,
}

impl<B: PublisherBackend> Publisher<B> {
	pub fn new(backend: B) -> Self {
		Self {
			backend,
			strategy: ZenohTopicStrategy::default(),
		}
	}

	pub fn with_strategy(mut self, strategy: ZenohTopicStrategy) -> Self {
		self.strategy = strategy;
		self
	}

	pub fn strategy(&self) -> &ZenohTopicStrategy {
		&self.strategy
	}

	pub fn backend(&self) -> &B {
		&self.backend
	}

	pub fn backend_mut(&mut self) -> &mut B {
		&mut self.backend
	}

	pub fn into_backend(self) -> B {
		self.backend
	}

	/// `frame` を MessagePack エンコードして strategy が指定する key に put する。
	pub fn send(&mut self, frame: &UNMotionFrame) -> Result<(), Error> {
		let key_expr = self.strategy.key_expr_for_frame(frame);
		let payload = encode_frame(frame)?;
		self.backend.put(&key_expr, &payload)
	}
}

impl<B: PublisherBackend + core::fmt::Debug> core::fmt::Debug for Publisher<B> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Publisher")
			.field("backend", &self.backend)
			.field("strategy", &self.strategy)
			.finish()
	}
}

#[cfg(feature = "zenoh-transport")]
pub use real::ZenohSessionBackend;

#[cfg(feature = "zenoh-transport")]
mod real {
	use zenoh::{Session, Wait};

	use super::PublisherBackend;
	use crate::{Error, ZenohSessionConfig};

	/// 実 Zenoh セッションをラップした Publisher バックエンド。
	pub struct ZenohSessionBackend {
		session: Session,
	}

	impl ZenohSessionBackend {
		/// 既定設定 (`zenoh::Config::default()`) でセッションを開く。
		pub fn open_default() -> Result<Self, Error> {
			Self::open(&ZenohSessionConfig::default())
		}

		/// listen endpoint や scouting policy を指定してセッションを開く。
		pub fn open(config: &ZenohSessionConfig) -> Result<Self, Error> {
			let session = zenoh::open(config.to_zenoh_config()?)
				.wait()
				.map_err(|e| Error::transport(format!("zenoh open failed: {e}")))?;
			Ok(Self { session })
		}

		/// 既存の `zenoh::Session` を引き取る (Publisher と Subscriber でセッションを共有したい場合)。
		pub fn from_session(session: Session) -> Self {
			Self { session }
		}

		pub fn session(&self) -> &Session {
			&self.session
		}
	}

	impl core::fmt::Debug for ZenohSessionBackend {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.debug_struct("ZenohSessionBackend").finish_non_exhaustive()
		}
	}

	impl PublisherBackend for ZenohSessionBackend {
		fn put(&mut self, key_expr: &str, payload: &[u8]) -> Result<(), Error> {
			self.session
				.put(key_expr, payload.to_vec())
				.wait()
				.map_err(|e| Error::transport(format!("zenoh put failed on {key_expr}: {e}")))?;
			Ok(())
		}
	}
}
