# UNMotionFrame スキーマ

この文書は `un-motion-frame` **v1.1.0** のスキーマ規約を定義する。
v1.0.0 との差分は本ページ末尾の「v1.0 → v1.1 変更点」節を参照。

`UNMotionFrame` は、フレーム単位のモーションデータを表すトランスポート非依存の
スキーマである。送信側は JSONL、MessagePack、CBOR、Zenoh、in-process channel など
任意の経路で送信できる。ただし、どの経路を使ってもフレームの意味は変わらない。

## 互換性

- `MotionHeader.magic` は常に `UNMF`。
- `version_major = 1` は現在の安定版 major version。
- `version_minor = 1` は現在の安定版 minor version。
- 受信側は未知の object field を無視する。
- 送信側は、他の repository が使い始めた enum variant 名を安定させる。
- serde format では public API に書かれた Rust field 名と enum variant 名を使う。
  例: `timestamp_basis`、`ModelLocal`。
- 新しい optional field には、古い frame を decode できるよう妥当な default を用意する。
- minor version の更新で field を追加することは許されるが、既存 field の意味を変えない。
  意味を変える破壊的変更は **major version の更新** を伴う。
- transport 層 (例: Zenoh) で **schema major** をトピック名に埋めるなどして、互換性のない major
  版が同じバスに同居しても誤デコードしない構造を作ることが望ましい。`docs/zenoh-transport.md`
  も参照。

## 時刻

すべての timestamp は、`timestamp_basis` が示す clock における unsigned nanoseconds。

- `capture_timestamp_ns`: camera、tracker、source が input を sample した時刻。
- `frame_timestamp_ns`: buffer と interpolation が使う論理的な presentation time。
- `processed_timestamp_ns`: 送信側がこの frame を構築し終えた時刻。
- `expected_dt_ns` (v1.1, optional): 送信側が想定している 1 frame あたりの公称インターバル
  (nanoseconds)。受信側のバッファや補間は actual な inter-arrival time を優先するが、
  この値があると latency 補正や再生レート推定の初期値として使える。値が無い場合は受信側
  が arrival 間隔から推定するか、固定値を仮定してよい。

UN Avatar などの live avatar playback を行う受信側は、`frame_timestamp_ns` を基準に
sort / buffer する。値が 0 の場合は `capture_timestamp_ns`、それも 0 なら arrival
time に fallback する。`sequence` は送信ストリームごとの単調増加値であり、drop
や reorder の検出に使う。playback clock ではない。

## ストリーム識別

`stream_id` (v1.1, optional) は、同じ producer (`MotionMetadata.producer`) が複数の論理的に
独立したストリームを並走させるときの識別子。例えば「右側カメラの Mediapipe」と「左側カメラの
Mediapipe」を同じプロセスから流す場合、両者の `producer` が同一でも `stream_id` で受信側が
区別できる。`source_id` (各 sample の出所) と `stream_id` (フレーム全体の文脈) は別物である
点に注意する。

`stream_id` が無い frame は、producer ごとに単一ストリームと見なしてよい。transport 側で
ストリーム単位の topic 分離をしたい場合 (例: Zenoh `TopicMode::ByStreamId`) も、
`stream_id` が無いときは `"default"` のような既定セグメントに集約される。

## 座標

`MotionHeader` の `coordinate_space`、`handedness`、`length_unit` は、その frame 内の
全 transform の数値基準を表す。将来の extension が明示しない限り、個別 sample ごと
には変わらない。

UNMotionFrame の送信側に推奨する canonical space は次の通り。

- `coordinate_space = UNMotion`
- `handedness = RightHanded`
- `length_unit = Meter`
- +X は subject の右
- +Y は上
- +Z は subject の前方

他の space も許可するが、送信側は実際の座標系に合わせて header を設定する。

- `Camera`: camera-relative metric、または source-native camera space。
- `NormalizedImage`: normalized image coordinates。通常は 0.0 から 1.0。
- `ModelLocal`: target model または parent に相対な local transform space。
- `ModelWorld`: model scene 内の world transform space。
- `Vmc`: VMC/VMCP-compatible coordinates。

quaternion は `(x, y, z, w)` で、normalized であることが望ましい。受信側は zero
ではない quaternion を適用前に renormalize できる。

## Transform の意味

`TransformSample` は partial transform data を運ぶ。`None` は「この component が
sample に存在しない」という意味で、zero ではない。

- `translation`: `length_unit` に基づく position。
- `rotation`: frame coordinate basis に基づく orientation quaternion。
- `scale`: component scale。missing scale は identity scale と扱う。
- `linear_velocity`: translation delta per second。
- `angular_velocity`: radians per second の angular velocity vector。

`HumanoidPose` では次の規約を使う。

- `root` は subject の model/root transform。
- `BoneSample.transform` は、送信側が metadata や extension で別途明示しない限り
  humanoid bone の parent-local transform。
- local bone rotation は、retargeter が target avatar の rest pose を基準に適用する。
- world-space source data は、可能な限り publish 前に parent-local bone transform へ
  変換する。

この規約は UN Avatar にとって重要である。renderer は bone ごとに world か parent-local
かを推測せず、humanoid bone を VRM/glTF skeleton へ retarget できる。

## Humanoid Bones

`HumanoidBone` は一般的な VRM humanoid set に従う。

- torso: `Hips`, `Spine`, `Chest`, `UpperChest`, `Neck`, `Head`
- arms: `LeftShoulder`, `LeftUpperArm`, `LeftLowerArm`, `LeftHand`,
  `RightShoulder`, `RightUpperArm`, `RightLowerArm`, `RightHand`
- legs: `LeftUpperLeg`, `LeftLowerLeg`, `LeftFoot`, `LeftToes`,
  `RightUpperLeg`, `RightLowerLeg`, `RightFoot`, `RightToes`
- face attachments: `LeftEye`, `RightEye`, `Jaw`

enum discriminant は stable identifier である。並べ替えない。bone がさらに必要な場合は、
新しい variant を末尾に追加する。

UN Avatar はこれらの bone を `SkeletonProfile` / `RetargetMap` 経由で map する。
missing bone は許可される。fallback、twist-bone distribution、rest-pose correction、
scale correction は受信側の責務。

## Hands

`HandMotion.wrist` はその hand の wrist/root transform。`FingerPose` は 1 本の finger
と ordered joint transform を持つ。

finger joint order:

- `Thumb`: metacarpal, proximal, distal, tip
- `Index`, `Middle`, `Ring`, `Little`: metacarpal, proximal, intermediate,
  distal, tip

tracker が全 joint を提供できない場合は、分かる prefix だけを入れ、confidence/state
を適切に設定する。受信側は vector length を固定と仮定せず、必ず確認する。

## Face, Eyes, Expressions

`FaceMotion.head` は、提供される場合の face/head transform。humanoid `Head` bone data
と `FaceMotion.head` は同時に存在し得る。受信側は skeleton retargeting には
humanoid bone を優先し、face-specific stabilization や diagnostics には
`FaceMotion.head` を使う。

`EyeMotion` は left、right、combined gaze を運ぶ。gaze direction は frame coordinate
basis における unit vector が望ましい。

expression name は UTF-8 identifier。推奨 namespace は次の通り。

- `vrm0:<BlendShapeClipName>`
- `vrm1:<ExpressionName>`
- `arkit:<BlendShapeName>`
- `vmc:<BlendShapeName>`
- `un:<semantic-name>`

simple stream では unprefixed name も許可する。ただし application 間で流す送信側
は namespace を付けることを推奨する。`value` は通常 `0.0..=1.0`。namespace が別 range を
文書化していない限り、受信側は範囲外 value を clamp できる。

UN Avatar では expression sample は expression layer への input である。priority、
additive/override/multiply blending、preset、conflict resolution は avatar 側 layer
の責務。

### 重複と既定

- 同一 frame 内で **同じ name の expression sample を複数 entry にしない**。送信側は
  最終 value を 1 件だけ載せる。値の合成は送信側で済ませる。
- 受信側は、互換のため複数 entry を受け取ったときは **後勝ち** で 1 件に丸める。
- `ExpressionSample.state` が無い (v1.0 sender 由来) frame をデコードしたときの **既定値は
  `Valid`** (v1.1)。v1.0 では `Missing` だったが、「entry が存在する」こと自体が
  「value が利用可能」を意味するため。state を「保留」「フェード」させたい sender は
  明示的に `Held` / `Decayed` を入れる。

## MotionSignal の互換規約

`MotionSignal` は、typed field にまだ載せていない tracker-specific data や
compatibility data を運ぶ escape hatch である。新しい送信側は可能な限り `body`、
`face`、`eyes`、`left_hand`、`right_hand` を使い、既存の受信側との互換が必要な
場合に signal も併用する。

UNMotion / VMC 互換で使う既定 signal 名は次の通り。

- head: `head.yaw`, `head.pitch`, `head.roll`
- eye: `eye.left.yaw`, `eye.right.yaw`, `eye.left.pitch`, `eye.right.pitch`
- face ARKit/Perfect Sync: `face.<ARKitBlendShapeName>`
- arm point: `arm.<left|right>.<shoulder|elbow|wrist>.<x|y|z>`
- hand palm basis: `hand.<left|right>.palm.<forward|across|normal>.<x|y|z>`
- hand wrist scalar: `hand.<left|right>.wrist.<x|y|z|pitch|yaw|roll>`
- finger curl: `hand.<left|right>.<thumb|index|middle|ring|little>.curl`
- finger joint curl:
  `hand.<left|right>.<thumb|index|middle|ring|little>.<mcp|pip|dip>.curl`
- finger spread: `hand.<left|right>.<thumb|index|middle|ring|little>.spread`
- presence flag: `hand.<left|right>.present`

これらの scalar value は原則 `-1.0..=1.0` または `0.0..=1.0` の normalized 値。
ただし point や basis vector の component は header の coordinate basis に従う。
受信側は unknown signal を無視し、known signal でも `state != Valid` や
non-finite value を低信頼として扱う。

## Confidence と State

confidence value は `0.0..=1.0`。

- `1.0`: direct high-confidence source data。
- `0.0`: useful confidence なし。

送信側は NaN と infinity を避ける。受信側は non-finite value を `0.0` と扱ってよい。

`TrackingState` は body/face/eye/hand/source 全体を表す。`SampleState` は個別 sample
を表す。

- `Valid`: measured かつ usable。
- `Inferred`: 他の data から derived。
- `Held`: 前 frame から repeat。
- `Decayed`: held だが reliability が低下中。
- `Missing`: unavailable。

## Sources

`sources` は indexed list。`source_index` field はこの vector を参照する。index は
frame ごと。送信側は可能なら stream 全体で source order を安定させるが、受信側
は identity 判定にそれへ依存しない。identity には `source_id` を使う。

## Metadata と Extensions

`MotionMetadata` は人間向けの notes、生成元情報、schema identity、小さな extension
value を運ぶ。extension entry は次を使う。

- `namespace`: reverse-DNS または短い project namespace。例:
  `network.usagi.unmotion`。
- `name`: その namespace 内の key。
- `value`: UTF-8 payload。structured data が必要なら JSON text を使う。

大きな binary blob、tracker-specific landmark、calibration file は frame 外で運び、
extension metadata から参照する。

## Transport の指針

この crate は transport を定義しない。推奨 envelope は次の通り。

- JSONL: 1 行に serialized `UNMotionFrame` 1 個。
- MessagePack/CBOR: message ごとに frame 1 個。
- Zenoh: motion stream ごとに topic を分け、value は serialized frame。
  公式の Zenoh wire 規約は同 workspace の `un-motion-zenoh` crate と
  `docs/zenoh-transport.md` を参照。

transport envelope は routing、topic、QoS、arrival timestamp metadata を追加できる。
ただし frame semantics は変えない。

## Keyframe ヒント (forward-compat)

v1.1 では keyframe / delta frame の区別を明示する field は導入していない。
v1.0 sender / v1.1 sender ともに「すべての frame は full frame」として扱う。

将来 minor version で `MotionExtension` の予約 namespace
`network.usagi.unmotion.keyframe` を経由した keyframe ヒントを追加する可能性がある。
受信側は、未知の extension は無視する規約 (上記「互換性」節) に従い、現状ではこの
slot を意識する必要は無い。

## v1.0 → v1.1 変更点

- `MotionHeader.version_minor` を `0` から `1` に更新。
- `MotionHeader.stream_id: Option<String>` を追加 (optional, default `None`)。
- `MotionHeader.expected_dt_ns: Option<u64>` を追加 (optional, default `None`)。
- `ExpressionSample.state` の **欠落時の既定値** を `Missing` から `Valid` に変更。
  既存 sender が `state` を毎フレーム明示している場合は影響なし。
- `MotionMetadata.schema_version` の既定値を `"1.0.0"` から `"1.1.0"` に更新。
- transport 規約として **`un-motion-zenoh` crate** を新設 (本 crate の外側で定義)。
- 既存 field の意味・型は変更していない。v1.0 sender が生成したフレームは v1.1 receiver で
  そのまま decode できる。
