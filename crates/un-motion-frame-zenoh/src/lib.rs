//! UNMotionFrame を **Zenoh で Pub/Sub** するための公式 wire 規約とユーティリティ。
//!
//! 設計目標:
//!
//! - `un-motion-frame` で定義されたスキーマを意味的に保ったまま、Zenoh を通じて Publisher / Subscriber 間で交換する。
//! - **トポロジ非依存**: peer-to-peer / router 経由 / loopback いずれでも同じコードで動く。
//! - **ペイロード形式の単一化**: 送受双方が同じ MessagePack エンコーディングを共有する。
//! - **テスト容易性**: 実 Zenoh セッションを開かなくても `InMemoryBackend` / `ReplayBackend` で
//!   Publisher / Subscriber を結線できる。
//! - **スキーマ進化の互換性**: Zenoh の key expression に `vN` セグメントを含め、
//!   major 互換版だけが同じ key で交換されるようにする。
//!
//! # 典型的な使い方
//!
//! ## Publisher 側 (UNMotion)
//!
//! ```ignore
//! use un_motion_frame_zenoh::{Publisher, ZenohSessionBackend, ZenohTopicStrategy};
//! use un_motion_frame::UNMotionFrame;
//!
//! let backend = ZenohSessionBackend::open_default()?;
//! let mut publisher = Publisher::new(backend).with_strategy(ZenohTopicStrategy::default());
//!
//! let frame = UNMotionFrame::new(1);
//! publisher.send(&frame)?;
//! # anyhow::Ok(())
//! ```
//!
//! ## Subscriber 側 (UN Avatar)
//!
//! ```ignore
//! use un_motion_frame_zenoh::{Subscriber, ZenohSubscriberBackend, ZenohTopicStrategy};
//!
//! let mut backend = ZenohSubscriberBackend::open_default()?;
//! let subscriber = Subscriber::declare(&mut backend, ZenohTopicStrategy::default())?;
//!
//! while let Some(frame) = subscriber.try_recv_frame()? {
//!     // 受信した UNMotionFrame を retarget / render に流す。
//! }
//! # anyhow::Ok(())
//! ```
//!
//! # Wire 規約
//!
//! - エンコーディング: **MessagePack** (`rmp-serde`)。Producer/Consumer 双方で `encode_frame` /
//!   `decode_frame` を経由することを推奨。
//! - key 形式: 既定で `un-motion/frame/v1`。`ZenohTopicStrategy::ByPrimarySource` を選ぶと
//!   `un-motion/frame/v1/<primary-source-id>`、`ByStreamId` を選ぶと
//!   `un-motion/frame/v1/<stream-id>` を組み立てる。
//! - Subscriber は既定で `un-motion/frame/v1` または `un-motion/frame/v1/**` を購読する。
//! - schema major が上がるたびに `vN` セグメントが上がる。旧版の subscriber は旧 key だけ
//!   購読し、新版の publisher は新 key にだけ流すことで **クロスメジャー間の事故** を避ける。
//!
//! `docs/zenoh-transport.md` も参照。

#![forbid(unsafe_code)]

mod codec;
mod error;
mod in_memory;
mod publisher;
mod replay;
mod subscriber;
mod topic;

pub use codec::{decode_frame, encode_frame};
pub use error::Error;
pub use in_memory::{InMemoryBackend, PublishedMessage};
pub use publisher::{Publisher, PublisherBackend};
pub use replay::ReplayBackend;
pub use subscriber::{ReceivedMessage, Subscriber, SubscriberBackend, SubscriptionHandle};
pub use topic::{ZenohTopicStrategy, TopicMode, sanitize_segment};

#[cfg(feature = "zenoh-transport")]
pub use publisher::ZenohSessionBackend;
#[cfg(feature = "zenoh-transport")]
pub use subscriber::ZenohSubscriberBackend;

/// この crate が想定する UNMotionFrame の schema major version。
///
/// `ZenohTopicStrategy::default()` の `schema_major` と一致する。
pub const SCHEMA_MAJOR: u16 = 1;
