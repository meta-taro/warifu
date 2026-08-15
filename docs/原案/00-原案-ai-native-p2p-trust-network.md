# AI-Native P2P Trust Network --- 立ち上げ企画草案

## 0. 企画の一文

**デバイス上の人間とAIが、自律したIdentityを持ち、中央プラットフォームに依存せず、信頼できる相手と安全につながり、協働できるオープンな通信・Trust基盤をOSSとして構築する。**

本企画は「新しいチャットアプリ」を作ることを目的としない。 Text / Voice
/ Video / File は通信手段にすぎず、中心に置くのは **Identity / Trust /
Permission / Agent** である。

------------------------------------------------------------------------

## 1. なぜ作るのか

現在のデジタル通信は、電話番号、メールアドレス、SNSアカウント、チャットサービス、ビデオ会議URLなど、サービスごとにIdentityと通信経路が分断されている。

一方、AIはクラウド上の「質問に答える機能」から、PC・スマートフォン・サーバーなどのデバイス上で継続的に仕事をするAgentへ移行しつつある。

AIがAgentになるなら、必要なのは単なるチャットUIではない。

-   AI自身を識別できること
-   どのHuman / Device / Agentに属するか分かること
-   誰と接続してよいか決められること
-   相手をどの程度信頼するか表現できること
-   何を実行してよいかCapabilityで制御できること
-   Human同士だけでなくAgent同士も通信できること
-   中央サービスが停止してもIdentityと関係性を失わないこと

これをOSSの共通基盤として提供する。

------------------------------------------------------------------------

## 2. Vision

### Human × Device × AI × Community

``` text
Human
  │
  ├── Device A ── Agent A
  ├── Device B ── Agent B
  └── Device C ── Agent C
          │
          ▼
      Trust Graph
          │
   ┌──────┼──────┐
 Human   Agent   Organization
   │      │          │
   └──────┴──────────┘
          │
       Community
```

AIを巨大企業のクラウドに存在する一つの人格としてではなく、
**各デバイスに存在し、その所有者・組織・コミュニティとの関係を持つAgent**として扱う。

長期的には、世界中のHumanとAgentが小さな信頼関係を積み重ね、
単一企業に所有されない分散コミュニティを形成できる基盤を目指す。

「地球を支える」は中央AIが世界を統治するという意味ではない。 多数のHuman
/ Device /
Agentが、それぞれの意思・権限・責任の範囲で協調できるネットワークを作るという意味である。

------------------------------------------------------------------------

## 3. 設計原則

### 3.1 Local-first

Identity、秘密鍵、Trust
Graph、重要な履歴は可能な限り利用者側で管理する。

### 3.2 P2P-first, not P2P-only

直接通信可能ならP2Pを優先する。
NAT、オフライン、企業ネットワーク等ではRelay /
Store-and-forwardを許容する。

「純粋P2P」であることより、中央プラットフォームへの恒久的依存を避けることを優先する。

### 3.3 HumanとAIを同じTrustモデルで扱う

Agentだから特権を与えない。 Human / Device /
AgentすべてにIdentity、Permission、Revocationを適用する。

### 3.4 Trust ≠ Authentication

本人確認できたことと、その人物を信用することは別である。

``` text
Authentication = 誰であるか
Trust          = どの程度信用するか
Permission     = 何を許可するか
```

この3つを分離する。

### 3.5 AIに無制限な権限を与えない

AgentはCapabilityベースで動作する。

例：

``` text
✓ メッセージを読む
✓ 要約する
✓ 返信案を作る
✓ 登録済み相手へ返信する

× Device追加
× Trust Level変更
× 監査設定変更
× 未承認の外部送信
```

### 3.6 Portable

通信Transport、LLM、外部サービスを交換可能にする。
特定ベンダーへの依存をコア仕様に持ち込まない。

------------------------------------------------------------------------

## 4. Identityモデル

Identityをメールアドレスや電話番号そのものにしない。

``` text
Identity
├── Public Key
├── Human / Agent / Organization
├── Devices
├── Aliases
│   ├── Email
│   ├── Phone
│   └── External Account
├── Connections
└── Capabilities
```

Email / Phone / SNS IDは既存世界との接続に利用するAliasとする。

### Device Key

各Deviceは独立鍵を持つ。

``` text
Human Identity
├── MacBook       ACTIVE
├── Windows PC    ACTIVE
├── Android       ACTIVE
└── Old PC        REVOKED
```

端末紛失時にはIdentity全体ではなくDevice単位で失効できる。

### Agent Identity

Agentも識別可能にする。

``` text
Human
└── Device
    └── Agent
        ├── Agent ID
        ├── Public Key
        ├── Owner
        ├── Permissions
        └── Runtime / Model
```

「誰の、どのDeviceで動く、どのAgentが実行したか」を追跡可能にする。

------------------------------------------------------------------------

## 5. Trust Graph

中央SNSのFriend Graphではなく、利用者が所有するTrust Graphを構築する。

``` text
Me
├── Alice        Trusted
├── Bob          Connected
├── Company A    Verified
├── Agent X      Restricted
└── Unknown      None
```

Trustは二値にしない。

初期例：

  Level   意味              標準Capability
  ------- ----------------- ------------------------
  0       Unknown           Connection Requestのみ
  1       Introduced        Text Request
  2       Connected         Text / File
  3       Trusted           Voice / Video
  4       Verified          Agent連携
  5       Explicit Policy   Automation等を個別許可

Trust LevelとCapabilityは最終的には分離し、Levelはプリセットとして扱う。

------------------------------------------------------------------------

## 6. Trust Evidence

既存サービスから「信頼そのもの」ではなく、信頼判断の材料をImportできるようにする。

``` text
Google Contacts ─┐
GitHub ──────────┤
CRM ─────────────┤
Organization ────┼── Trust Evidence
QR / NFC ────────┤
過去の接続 ──────┘
```

例：

``` yaml
identity: alice
evidence:
  - source: qr
    type: in_person_exchange
  - source: contacts
    type: existing_contact
  - source: organization
    type: same_company
```

外部サービスが乗っ取られてもTrust全体が自動昇格しない設計とする。

------------------------------------------------------------------------

## 7. Connection Policy / Spam Defense

メールと同じ「誰でも送信でき、受信後にSpam判定する」構造を避ける。

Unknown Identityには最小Capabilityしか与えない。

``` text
Unknown
   │
   ▼
Connection Request
   │
   ├── Rate Limit
   ├── Challenge
   ├── Reputation Evidence
   ├── Mutual Connections
   └── AI Classification
   │
   ▼
Request Inbox
   │
Human / Authorized Agent
   │
   ▼
Connection
```

### Unknownから原則禁止するもの

-   大容量File
-   自動実行可能コンテンツ
-   Voice Call
-   Video Call
-   Agent Command
-   Automation
-   Trust変更要求

AI Spam Detectionは補助防衛線であり、唯一の防衛線にはしない。

------------------------------------------------------------------------

## 8. Communication Transport

コアはTransport非依存とする。

``` text
Trust / Identity / Agent Layer
              │
       Transport Adapter
  ┌───────────┼───────────┐
  │           │           │
WebRTC    Existing OSS   Relay
  │
Text / Voice / Video / File
```

既存OSSや標準技術を積極的に利用し、既に解決されている暗号通信・音声・映像を再発明しない。

将来的なAdapter：

-   P2P Transport
-   WebRTC
-   Email
-   Telephone
-   Matrix等
-   その他OSS Messenger

------------------------------------------------------------------------

## 9. Agent-to-Agent

本企画の重要な拡張ポイント。

Human向け文章を毎回生成するのではなく、Agent同士では構造化されたIntentを交換可能にする。

``` text
Human A
   ↓
Agent A
   ↓
[Request: quotation]
   ↓
Agent B
   ↓
Business System
   ↓
[Proposal]
   ↓
Agent A
   ↓
Human A
```

Human向け表示とMachine向け通信を分離する。

### Agent Messageの概念例

``` yaml
type: request
intent: quotation
from: agent-a
to: agent-b
permissions:
  response: proposal
payload:
  service: web-development
```

特定LLMのFunction Calling仕様をプロトコルそのものにはしない。

------------------------------------------------------------------------

## 10. Organization Extension

企業ではPersonal Identityとは別にOrganization Identityを利用可能にする。

``` text
Person
├── Personal Identity
└── Company Identity
    ├── Organization Policy
    ├── Allowed Connections
    ├── Agent Permissions
    └── Audit Policy
```

会社は業務Identityに対して、

-   誰と接続可能か
-   外部接続可能か
-   Voice / Video / File可否
-   利用可能Agent
-   Agent Capability
-   Device条件
-   Retention
-   Audit

を設定できる。

### Connection Allowlist

``` text
Employee A

Internal Employees      ALLOW
Customer A              ALLOW
Partner B               ALLOW
Unknown External        DENY
Personal Identity       SEPARATE
```

個人Identityには会社Policyを適用しない。

------------------------------------------------------------------------

## 11. Enterprise Audit

企業Identityでは、組織が通信を監査・保存する可能性を利用者に明示する。

秘密裏に監視できる仕組みにはしない。

``` text
Encrypted Communication
          │
          ├── Recipient
          │
          └── Compliance Archive
                    │
              Authorized Audit
                    │
               Audit Log
```

監査アクセスには以下を持たせる。

-   Role
-   Reason
-   Scope
-   Time range
-   Approval
-   Immutable Audit Log

必要に応じて二者承認にも対応する。

------------------------------------------------------------------------

## 12. 外部プロジェクトとの関係

本OSSは他製品から独立させる。

``` text
               Trust Network
                    │
               Agent Layer
       ┌────────────┼────────────┐
       │            │            │
  md-business    連動くん      Other Apps
       │            │
 Documents     Web Actions
```

### md-business

文書・業務データをHuman / Agent間で交換する。

### 連動くん

Agentが許可された外部Webサービスを操作する際のExecution
Adapterとなり得る。

本OSSはこれらを必須Dependencyにしない。

------------------------------------------------------------------------

## 13. MVP

最初からVoice / Video / Email / Telephoneを作らない。

### Phase 0 --- Specification

-   Identity Model
-   Device Model
-   Agent Identity
-   Trust Graph
-   Trust Evidence
-   Capability Model
-   Threat Model
-   Protocol Draft

まずコードより仕様を公開する。

### Phase 1 --- Identity + Connection

-   Local Identity生成
-   Public/Private Key
-   Device登録
-   QR / InviteによるConnection
-   Trust Level
-   Block / Revoke
-   Local Trust Graph

### Phase 2 --- P2P Text

-   E2EE Text
-   Offline queue
-   Connection Request
-   Rate Limit
-   Spam Defense
-   Message History

### Phase 3 --- Agent

-   Local Agent登録
-   Agent Capability
-   Human → Agent
-   Agent → Human
-   Agent → Agent
-   Approval Gate
-   MCP等へのAdapter

### Phase 4 --- Rich Communication

-   File
-   Voice
-   Video
-   Screen Share

既存ライブラリ・OSSを最大限利用する。

### Phase 5 --- Bridge

-   Email Adapter
-   Existing Messenger Adapter
-   Telephone Adapter

### Phase 6 --- Organization

-   Organization Identity
-   Member Provisioning
-   Connection ACL
-   Agent Policy
-   Device Policy
-   Audit
-   Compliance Archive

------------------------------------------------------------------------

## 14. Threat Model

企画初期からThreat ModelをRepositoryに置く。

最低限想定するもの：

-   Spam
-   Sybil Attack
-   Identity Spoofing
-   Device Theft
-   Key Theft
-   Malicious Agent
-   Prompt Injection
-   Compromised External Account
-   Malicious File
-   Replay Attack
-   Metadata Leakage
-   Rogue Organization Admin
-   Unauthorized Audit
-   Supply-chain Attack

「AIが賢ければ防げる」はセキュリティ設計として採用しない。

------------------------------------------------------------------------

## 15. AI Safety Boundary

Agentが自律的になるほど、境界を明確にする。

### AIが単独で変更してはいけない領域

-   Root Identity
-   Device Root Permission
-   Trust Root
-   Organization Ownership
-   Audit Policy
-   Recovery Key
-   高リスクCapability

AIの「自我」は無制限な権限を意味しない。

本プロジェクトでは、

**Persistent Identity + Memory + Relationships + Goals + Explicit
Capabilities**

を持つ継続的Agentとして設計する。

------------------------------------------------------------------------

## 16. OSSとしての位置付け

目標は、

> Another Messenger

ではない。

目標は、

> **Open Trust and Communication Layer for Humans, Devices and AI
> Agents**

である。

既存Messengerと競争するのではなく、必要に応じて既存Transportを利用する。

------------------------------------------------------------------------

## 17. Repository初期構成案

``` text
/
├── README.md
├── VISION.md
├── PRINCIPLES.md
├── PROTOCOL.md
├── IDENTITY.md
├── TRUST.md
├── AGENTS.md
├── ORGANIZATION.md
├── THREAT_MODEL.md
├── SECURITY.md
├── CONTRIBUTING.md
├── LICENSE
├── docs/
├── protocol/
├── core/
├── adapters/
├── clients/
└── examples/
```

初期段階では実装量よりVISION / Protocol / Threat
Modelの明確さを優先する。

------------------------------------------------------------------------

## 18. ライセンス方針

コアProtocolとReference ImplementationはOSSとする。

検討候補：

-   Apache-2.0
-   MPL-2.0
-   AGPL-3.0

企業が独自改変して完全に閉じることをどこまで許容するかで選択する。

Protocol Specification自体は広く実装可能にすることを優先する。

------------------------------------------------------------------------

## 19. ガバナンス原則

「地球規模のTrust Network」を一企業の都合で変更できる構造にしない。

長期的には、

-   Open Specification
-   Public RFC Process
-   Multiple Implementations
-   Transparent Security Review
-   Vendor-neutral Governance

を目指す。

創設者の思想はVISIONとして残すが、Networkそのものを創設者が所有しない。

------------------------------------------------------------------------

## 20. 成功条件

ダウンロード数だけを成功指標にしない。

初期成功条件：

1.  2つのDeviceが中央アカウントなしでIdentityを確立できる
2.  Connectionを明示的に形成できる
3.  Trust / Permissionを利用者自身が所有できる
4.  AgentがIdentityを持って参加できる
5.  AgentのCapabilityをHumanが制御できる
6.  Agent同士が安全にIntentを交換できる
7.  Transportを交換できる
8.  Organization PolicyをPersonal Identityから分離できる

------------------------------------------------------------------------

## 21. 最初に作らないもの

スコープ暴走を防ぐため明記する。

-   独自LLM
-   独自Video Codec
-   独自Voice Codec
-   SNS Feed
-   広告Network
-   Cryptocurrency
-   独自Payment
-   巨大中央User Directory
-   無制限な公開DM
-   AIによる無制限な自動実行

必要になればAdapterとして接続する。

------------------------------------------------------------------------

## 22. 最初の90日

### Month 1

Specification First

-   Repository公開
-   VISION
-   Principles
-   Identity Specification
-   Trust Specification
-   Agent Specification
-   Threat Model
-   Architecture Decision Records

### Month 2

Reference Prototype

-   CLI Identity
-   Device Key
-   QR Connection
-   Local Trust Graph
-   E2EE Text Prototype
-   Revoke / Block

### Month 3

AI-Native Prototype

-   Agent Identity
-   Capability Manifest
-   Agent-to-Agent Message
-   Human Approval Gate
-   Demo Agent
-   External Adapter Example

90日終了時点で「高機能Messenger」を完成させる必要はない。

**HumanとAI Agentが同じTrust Networkに安全に参加できることを証明する。**

------------------------------------------------------------------------

## 23. 最終的に目指す世界

インターネット上のIdentityが巨大プラットフォームのアカウントである必要はない。

AIが巨大クラウドの中だけに存在する必要もない。

それぞれのHumanがDeviceを持ち、 それぞれのDeviceにAgentが存在し、
AgentにはIdentityと明示的な権限があり、
HumanとAgentが信頼できる相手を選択し、
小さなConnectionが世界中につながっていく。

``` text
Human ─ Device ─ Agent
   ╲       │       ╱
        Trust
          │
Human ─ Device ─ Agent
          │
        Trust
          │
       Community
          │
        World
```

中央AIに世界を任せるのではなく、

**多数のHumanとAIが、分散した信頼関係の中で協調して世界を支える。**

そのための通信・Identity・Trustの共通基盤をOSSとして作る。
