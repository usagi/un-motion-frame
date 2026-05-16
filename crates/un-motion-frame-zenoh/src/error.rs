use thiserror::Error;

/// `un-motion-frame-zenoh` の操作全般で返るエラー。
#[derive(Debug, Error)]
pub enum Error {
	/// MessagePack エンコードに失敗。
	#[error("MessagePack encode failed: {0}")]
	EncodeMessagePack(#[from] rmp_serde::encode::Error),
	/// MessagePack デコードに失敗。
	#[error("MessagePack decode failed: {0}")]
	DecodeMessagePack(#[from] rmp_serde::decode::Error),
	/// Zenoh など transport 層からのエラー (文字列で握り潰す)。
	///
	/// Zenoh 側のエラー型はバージョン間で変動が激しいため、ここでは表現を String に寄せて
	/// 上位レイヤ (un-avatar-zenoh など) の API 安定性を優先する。
	#[error("transport error: {0}")]
	Transport(String),
}

impl Error {
	pub fn transport<E: core::fmt::Display>(err: E) -> Self {
		Self::Transport(err.to_string())
	}
}
