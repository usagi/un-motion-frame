use std::sync::{Arc, Mutex};

use crossbeam_channel::Sender;

use crate::{
	Error,
	publisher::PublisherBackend,
	subscriber::{ReceivedMessage, SubscriberBackend, SubscriptionHandle},
};

/// プロセス内 publish/subscribe 用の testing バックエンド。
///
/// 同じ `InMemoryBackend` インスタンス (および `.clone()` で得られる同等の view) を
/// Publisher と Subscriber の両方に渡すと、Zenoh セッションを開かずに Pub/Sub の
/// **wire 規約** (key 構築・MessagePack エンコード・受信側ハンドラ) を統合テストできる。
///
/// 内部状態は `Arc<Mutex<_>>` で共有しており、`clone()` は安価。
#[derive(Clone, Default)]
pub struct InMemoryBackend {
	inner: Arc<Mutex<InMemoryInner>>,
}

#[derive(Default)]
struct InMemoryInner {
	published: Vec<PublishedMessage>,
	subscriptions: Vec<Subscription>,
}

struct Subscription {
	pattern: String,
	sender: Sender<ReceivedMessage>,
}

/// `InMemoryBackend` が記録する 1 件の publish 履歴。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedMessage {
	pub key_expr: String,
	pub payload: Vec<u8>,
}

impl InMemoryBackend {
	pub fn new() -> Self {
		Self::default()
	}

	/// これまで `put` 経由で送られたメッセージのコピーを返す。テスト用。
	pub fn published(&self) -> Vec<PublishedMessage> {
		self.inner.lock().expect("InMemoryBackend mutex poisoned").published.clone()
	}

	/// 履歴と subscription を空にする。
	pub fn clear(&self) {
		let mut inner = self.inner.lock().expect("InMemoryBackend mutex poisoned");
		inner.published.clear();
		inner.subscriptions.clear();
	}
}

impl core::fmt::Debug for InMemoryBackend {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let inner = self.inner.lock().expect("InMemoryBackend mutex poisoned");
		f.debug_struct("InMemoryBackend")
			.field("published", &inner.published.len())
			.field("subscriptions", &inner.subscriptions.len())
			.finish()
	}
}

impl PublisherBackend for InMemoryBackend {
	fn put(&mut self, key_expr: &str, payload: &[u8]) -> Result<(), Error> {
		let mut inner = self.inner.lock().expect("InMemoryBackend mutex poisoned");
		inner.published.push(PublishedMessage {
			key_expr: key_expr.to_string(),
			payload: payload.to_vec(),
		});
		let message = ReceivedMessage {
			key_expr: key_expr.to_string(),
			payload: payload.to_vec(),
		};
		// receiver が drop されている subscription はここで剥がす。
		inner.subscriptions.retain(|sub| {
			if matches_pattern(&sub.pattern, key_expr) {
				sub.sender.send(message.clone()).is_ok()
			} else {
				true
			}
		});
		Ok(())
	}
}

impl SubscriberBackend for InMemoryBackend {
	fn declare_frame_subscriber(&mut self, key_expr: &str) -> Result<SubscriptionHandle, Error> {
		let (tx, rx) = crossbeam_channel::unbounded::<ReceivedMessage>();
		let mut inner = self.inner.lock().expect("InMemoryBackend mutex poisoned");
		inner.subscriptions.push(Subscription {
			pattern: key_expr.to_string(),
			sender: tx,
		});
		Ok(SubscriptionHandle::from_rx(rx))
	}
}

/// Zenoh の wildcard を testing 用途で再現する非常に簡易なマッチャ。
///
/// 対応するのは:
/// - 完全一致
/// - `**` (1 つ以上のセグメントすべて)
/// - 各セグメント内の `*` (1 セグメントのみ、任意文字)
///
/// 完全な Zenoh の key matching ルールではないが、Pub/Sub 単体テストとしては十分。
pub(crate) fn matches_pattern(pattern: &str, key: &str) -> bool {
	let pattern_segments: Vec<&str> = pattern.split('/').collect();
	let key_segments: Vec<&str> = key.split('/').collect();
	match_segments(&pattern_segments, &key_segments)
}

fn match_segments(pattern: &[&str], key: &[&str]) -> bool {
	match (pattern.split_first(), key.split_first()) {
		(None, None) => true,
		(Some((&"**", rest)), _) => {
			if rest.is_empty() {
				// `**` が末尾なら残り key を 1 セグメント以上吸収する。
				!key.is_empty()
			} else if key.is_empty() {
				false
			} else {
				// `**` をできるだけ短く / できるだけ長く食わせる両方を試す。
				(1..=key.len()).any(|skip| match_segments(rest, &key[skip..]))
			}
		}
		(Some((&"*", rest_pat)), Some((_, rest_key))) => match_segments(rest_pat, rest_key),
		(Some((p_seg, rest_pat)), Some((k_seg, rest_key))) if p_seg == k_seg => {
			match_segments(rest_pat, rest_key)
		}
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn matches_exact_segments() {
		assert!(matches_pattern("a/b/c", "a/b/c"));
		assert!(!matches_pattern("a/b/c", "a/b/d"));
	}

	#[test]
	fn matches_single_segment_wildcard() {
		assert!(matches_pattern("a/*/c", "a/b/c"));
		assert!(!matches_pattern("a/*/c", "a/b/d"));
		assert!(!matches_pattern("a/*/c", "a/b/c/d"));
	}

	#[test]
	fn matches_multi_segment_wildcard_at_tail() {
		assert!(matches_pattern("a/**", "a/b"));
		assert!(matches_pattern("a/**", "a/b/c/d"));
		assert!(!matches_pattern("a/**", "a"));
	}

	#[test]
	fn matches_multi_segment_wildcard_in_middle() {
		assert!(matches_pattern("a/**/d", "a/b/d"));
		assert!(matches_pattern("a/**/d", "a/b/c/d"));
		assert!(!matches_pattern("a/**/d", "a/b/c"));
	}
}
