# UNMotionFrame Zenoh Transport

この文書は、`un-motion-frame` v1.1.0 を **Zenoh で Pub/Sub** するときの wire 規約を定義する。
実装は同 workspace の `un-motion-frame-zenoh` crate が提供する。

スキーマ規約 (`UNMotionFrame` の意味論) は `docs/schema.md`。本ページは「その frame を
Zenoh で交換するときの **key / payload / QoS / 互換ルール**」だけを扱う。

## 役割分担

- `un-motion-frame`: トランスポート非依存のフレームスキーマ (`UNMotionFrame` 等)。
- `un-motion-frame-zenoh`: Zenoh 経由で `UNMotionFrame` を交換するための encode/decode、
  key expression 構築、Publisher / Subscriber、テスト用 In-Memory / Replay バックエンド。

Publisher 側 (UNMotion など) と Subscriber 側 (UN Avatar など) は、**同じ `un-motion-frame-zenoh`
crate を使う** ことで wire 規約が崩れないことを保証する。

## エンコーディング

- 形式: **MessagePack** (`rmp-serde::to_vec` / `rmp-serde::from_slice`)。
- 1 Zenoh メッセージ = 1 `UNMotionFrame`。
- Publisher 側は `un_motion_frame_zenoh::encode_frame(&frame)` を使う。
- Subscriber 側は `un_motion_frame_zenoh::decode_frame(&payload)` を使う。
- 直接 serde で別形式 (JSON 等) を流すことは **互換性の対象外**。

理由: MessagePack は self-describing で順序 / 欠落 field に対する serde の挙動が安定しており、
かつ binary なので JSON より圧倒的に小さい (1 フレームあたりの差分は実測で 1/3 〜 1/5 程度)。
60 Hz でフルボディ + ハンド + パーフェクトシンクを流す典型ケースで CPU/帯域を有意に節約できる。

## Key Expression

Zenoh の key expression は **schema major version をセグメントとして埋める**:

```
<base>/v<schema_major>[/<discriminator>]
```

- `<base>`: アプリケーション識別。既定 `"un-motion/frame"`。
- `<schema_major>`: `MotionHeader.version_major` と一致。v1.x なら `v1`。
- `<discriminator>` (optional): topic 分割モードに応じて付与される追加セグメント。

### TopicMode

| Mode | Publisher が組み立てる key | Subscriber が購読する key |
|------|---------------------------|---------------------------|
| `Frame` | `<base>/v<major>` | `<base>/v<major>` |
| `ByPrimarySource` | `<base>/v<major>/<sanitize(primary source_id)>` | `<base>/v<major>/**` |
| `ByStreamId` | `<base>/v<major>/<sanitize(stream_id) or "default">` | `<base>/v<major>/**` |

- `Frame`: 単一プロデューサ・単一ストリーム想定。最もシンプル。
- `ByPrimarySource`: 同じ producer が複数 source から流す場合に、Subscriber 側で source 単位
  で filter / fan-out したいときに使う。
- `ByStreamId`: producer が `MotionHeader.stream_id` で論理ストリームを区別している場合に、
  Subscriber 側で stream 単位で filter / fan-out したいときに使う。

discriminator セグメントは `un_motion_frame_zenoh::sanitize_segment` で正規化する。
ASCII 英数字と `-_.` 以外は `_` に置換し、空文字列は `"unknown"` に置換する。Zenoh の
セパレータ `/` や wildcard `*` `**` を意図せず注入しないための保険。

### スキーマ進化と major version

- minor version の更新 (v1.0 → v1.1 等) では key を変えない。Subscriber は同じ key で受信できる。
- major version の更新 (v1 → v2 等) では `vN` セグメントの数値を更新する。
- 互換のない新版 Publisher と旧版 Subscriber が **同じ Zenoh バスに居ても topic が分離されている
  ので誤受信しない**。これが major version を topic に埋める最大の理由。
- 旧版を退役させる際は、producer 側が両 major に二重 publish するか、ブリッジを 1 台立てる。

## 互換性ルール

### Publisher (送信側)

1. `MotionHeader.version_major` / `version_minor` を実際の発行版に合わせる。
2. key の `vN` セグメントは `version_major` と一致させる。
3. payload は必ず `un_motion_frame_zenoh::encode_frame` を経由してエンコードする。

### Subscriber (受信側)

1. 既知の major version の key だけを subscribe する。
2. `un_motion_frame_zenoh::decode_frame` でデコードし、`MotionHeader.version_minor` を確認する。
   未知の minor を受信したら、その frame は未知 field を無視して既知 field だけで処理する
   (これが minor 互換の原則)。
3. 未知 major が混じった場合は、subscribe key を絞ることで通常は到達しないが、万一受信したら
   `decode_frame` がエラーになるか、`version_major` で reject する。

## QoS と reliability

`un-motion-frame-zenoh` v0.1 は Zenoh の **既定 QoS** を使う。すなわち:

- Reliability: `Reliable`
- Priority: `DataMedium`
- Congestion control: `Drop`
- Express: false

実時間 60 Hz の avatar tracking では多少の drop は許容され、最新フレームが優先されるべきなので、
将来 `Best Effort` + `Express` への切替を選択肢として提供する可能性がある (v0.2 以降)。
それまではアプリケーション側で過剰なバッファリングをしないことで遅延を抑える。

## レイテンシのヒント

UNMotionFrame には以下のフィールドが乗っており、Subscriber 側は遅延の見積もりに使える。

- `MotionHeader.capture_timestamp_ns`: センサ取得時刻。
- `MotionHeader.processed_timestamp_ns`: producer が frame を組み終えた時刻。
- `MotionHeader.expected_dt_ns` (v1.1): 公称 frame 周期。

Zenoh は arrival 時刻を別途返す機構を持つので、Subscriber 側で `now - capture` を計算するだけで
end-to-end 遅延を観測できる。Avatar 側のレンダラはこれを使って interpolation / extrapolation
の量を決めるとよい。

## テスト戦略

- **roundtrip 単体**: `un-motion-frame-zenoh::InMemoryBackend` を Pub/Sub 双方に渡して、Zenoh セッション
  を開かずに encode/decode + key 構築を一気通貫で検証する (`tests/roundtrip.rs` 参照)。
- **replay**: `un-motion-frame-zenoh::ReplayBackend` を Subscriber に渡して、保存しておいた frame 列を
  決定論的に再生する。
- **実 Zenoh 環境**: 同じ machine 上で `ZenohSessionBackend` (Publisher) と
  `ZenohSubscriberBackend` (Subscriber) を別プロセスで起動し、loopback 経由で確認する。

## 既知の制限

- v0.1 では publisher の **declare_publisher** (固定 key を Zenoh に事前宣言してルーティング
  最適化を有効化) は未対応。`session.put` のみで、毎回 ad-hoc に key を投げる。将来 v0.2 以降
  で declare 経由に切り替える。
- v0.1 では **attachment / encoding metadata** の付与は行わない。Subscriber は payload を
  常に MessagePack として解釈する。Zenoh の `Encoding` フィールドを使った将来の自己記述化
  も検討中。
- v0.1 では Publisher / Subscriber 用に **別の Zenoh セッション** を開く想定。同一プロセス内
  で 1 セッションを共有したい場合は、`ZenohSessionBackend::from_session`
  / `ZenohSubscriberBackend::from_session` に `zenoh::Session::clone()` を渡せばよい。
