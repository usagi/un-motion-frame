# UNMotionFrame

UNMotionFrame = `un-motion-frame` は、UNMotion と関連プロジェクトで使う
トランスポート非依存のモーションフレームスキーマ。

Rust の crate 名は `un-motion-frame`、import path は `un_motion_frame`。

```toml
[dependencies]
un-motion-frame = "1"
```

```rust
use un_motion_frame::UNMotionFrame;

let frame = UNMotionFrame::new(1);
```

## スコープ

- frame、signal、source、humanoid pose の共有データ構造。
- timestamp、coordinate space、confidence、sample state の明示的な規約。
- UN Avatar の retargeting に必要な humanoid、hand、gaze、expression の規約。
- 生成元情報と小さなプロジェクト固有値のための metadata / extension slot。
- 既定で有効な optional `serde` support。
- 実行時 transport への依存なし。
- UNMotion desktop application への依存なし。

## スキーマ

- [UNMotionFrame スキーマ v1.0.0](docs/schema.md)

## ライセンス

- [MIT](LICENSE)

## 作者

- [usagi / USAGI.NETWORK](https://usagi.network/)
