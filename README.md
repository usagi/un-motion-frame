# UNMotionFrame

`un-motion-frame` リポジトリは、UNMotion と関連プロジェクトで使う **モーションフレーム
スキーマと、その公式 transport ユーティリティ** を束ねた Cargo workspace である。

| Crate | 役割 |
|-------|------|
| [`un-motion-frame`](crates/un-motion-frame) | トランスポート非依存のフレームスキーマ (`UNMotionFrame`、`MotionHeader`、`HumanoidPose` 等)。 |
| [`un-motion-zenoh`](crates/un-motion-zenoh) | `UNMotionFrame` を Zenoh で Pub/Sub するための公式 wire 規約 (MessagePack エンコード、key expression、Publisher / Subscriber、テスト用 In-Memory / Replay バックエンド)。 |

`un-motion-frame` は意図的に transport を持たない。実行時の Pub/Sub を使いたい
アプリケーションは `un-motion-zenoh` (および将来追加され得る他 transport crate) と組み合わせる。

## 利用例

### スキーマだけ使いたい

```toml
[dependencies]
un-motion-frame = "1.1"
```

```rust
use un_motion_frame::UNMotionFrame;

let frame = UNMotionFrame::new(1);
```

### Zenoh で publish したい (UNMotion 側)

```toml
[dependencies]
un-motion-frame = "1.1"
un-motion-zenoh = "0.1"
```

```rust
use un_motion_frame::UNMotionFrame;
use un_motion_zenoh::{Publisher, ZenohSessionBackend, ZenohTopicStrategy};

let backend = ZenohSessionBackend::open_default()?;
let mut publisher = Publisher::new(backend).with_strategy(ZenohTopicStrategy::default());
publisher.send(&UNMotionFrame::new(1))?;
# anyhow::Ok(())
```

### Zenoh から subscribe したい (UN Avatar 側)

```rust
use un_motion_zenoh::{Subscriber, ZenohSubscriberBackend, ZenohTopicStrategy};

let mut backend = ZenohSubscriberBackend::open_default()?;
let subscriber = Subscriber::declare(&mut backend, ZenohTopicStrategy::default())?;

while let Some(frame) = subscriber.try_recv_frame()? {
    // retarget / render に流す
}
# anyhow::Ok(())
```

## スコープ

`un-motion-frame`:

- frame、signal、source、humanoid pose の共有データ構造。
- timestamp、coordinate space、confidence、sample state の明示的な規約。
- UN Avatar の retargeting に必要な humanoid、hand、gaze、expression の規約。
- 生成元情報と小さなプロジェクト固有値のための metadata / extension slot。
- 既定で有効な optional `serde` support。
- 実行時 transport への依存なし。
- UNMotion desktop application への依存なし。

`un-motion-zenoh`:

- MessagePack エンコード / デコード。
- schema major version を埋め込んだ key expression (`un-motion/frame/v1/...`)。
- Pub / Sub の Backend trait と、Zenoh セッションを使った既定実装。
- 統合テスト用の In-Memory / Replay バックエンド。

## ドキュメント

- [UNMotionFrame スキーマ v1.1.0](docs/schema.md)
- [UNMotionFrame Zenoh Transport](docs/zenoh-transport.md)

## ライセンス

- [MIT](LICENSE)

## 作者

- [usagi / USAGI.NETWORK](https://usagi.network/)
