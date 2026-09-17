// 映像を実際に張る層（M5-c2）。
//
// **ここだけが副作用を持つ。**規則は 3 つの純ロジックが持っている。
//
//   negotiation.ts … いつ何をすべきか（順序）
//   apply.ts       … 決めた手をどう打つか
//   media.ts       … 何を求めるか・断られた理由の読み方
//   ../link/path.ts / watch.ts … 経路が直接か中継か・表示を振動させない
//
// この層は**それらを繋ぐだけ**で、判断を持たない。持たせると、
// カメラの無い機械で確かめられなくなる。

import { pathFromStats, type LinkPath, type RtcStatLike } from '../link/path';
import { initialWatch, observe, type WatchState } from '../link/watch';
import { log, sendSignal, type SignalPayload } from '../bridge';
import { applyAction, type PeerLike } from './apply';
import { ICE_SERVERS, shouldSendVideo } from './media';
import type { Prefs } from './devices';
import { ハウリングの危険, 送ってよいか } from '../meeting/sending';
import { onLocalMediaReady, onRemote, start, type NegotiationState } from './negotiation';
import {
  候補を言い表す,
  対を言い表す,
  数えて言い表す,
  組の様子,
  送り受けを言い表す,
  映像の向き,
  音の向き,
  追跡の版,
  type 向き,
  type 統計の行,
  同じ網に居るか,
} from './trace';

/** 経路を見に行く間隔。短くしても、`watch.ts` が表示を落ち着かせる。 */
const STATS_EVERY_MS = 1000;

/**
 * 付かないまま、これだけ経ったら**様子を 1 回だけ記録へ出す**（issues/11 / #17）。
 *
 * `link/blocked.ts` の `黙っている秒` と同じ 15 秒にしてある ——
 * **画面が「ふさがっているかも」と言い出す時刻と、記録に残る時刻を揃える。**
 * ずれていると、人が見ている画面と記録が別の話に見える。
 */
const 様子を出すまでのミリ秒 = 15_000;

export interface CallHandlers {
  /** 相手の映像。 */
  onRemoteStream(stream: MediaStream): void;
  /** 表示すべき経路（直接 / 中継 / 不明）。 */
  onPath(path: LinkPath): void;
  /**
   * **映像がどちらへ流れているか**（**#39**）。
   *
   * 「つながっています」しか出していなかったので、**片方だけ流れていても人に分からなかった。**
   * **変わったときだけ呼ぶ**（1 秒ごとに呼ばない）。
   */
  on映像の向き?(向き: 向き): void;
  /**
   * **ハウリングの危険が在るか**（2026-09-17・オーナー依頼）。
   *
   * エコー除去は**自分の出力しか知らない。**
   * **同じ網に居る相手と音が往復している**とき、
   * 机の隣で鳴っている可能性があるので**その時に案内する。**
   * **支度のときだけ言っていては遅い。**
   */
  onHowlingRisk?(危ない: boolean): void;
}

/** 1 本の通話。閉じるまで生きている。 */
export class Call {
  private pc: RTCPeerConnection;
  private adapter: PeerLike;
  private state: NegotiationState;
  private watch: WatchState = initialWatch();
  private timer: ReturnType<typeof setInterval> | null = null;
  private closed = false;
  private local: MediaStream | null = null;
  /** 記録は**同じことを何度も書かない。**1 秒ごとに出すと読めなくなる。 */
  private 様子を出した = false;
  private 組を出した = false;
  private 始めた = 0;
  /** **前に伝えた映像の向き**（**#39**）。変わったときだけ伝える */
  private 前の向き: 向き | null = null;

  /**
   * **この部屋で「映像と音を足す」と決めたか。**
   *
   * **既定は偽。**押していない人から音や映像を送らない（**D113**）。
   * 2026-09-17 まで**音だけが関門を 1 つしか通っておらず**、
   * 前の会議のマイク設定が持ち越されて**入った瞬間に声が流れていた。**
   */
  private 映像を使う = false;

  constructor(
    offering: boolean,
    private handlers: CallHandlers,
    private prefs: Prefs,
    /** 相手の公開鍵。**送り先を間違えると経路が壊れる**ので必ず持つ（M6）。 */
    private peer: string,
  ) {
    this.state = start(offering);
    this.pc = new RTCPeerConnection({ iceServers: [...ICE_SERVERS] });
    this.adapter = {
      createOffer: () => this.pc.createOffer(),
      createAnswer: () => this.pc.createAnswer(),
      setLocalDescription: (d) => this.pc.setLocalDescription(d as RTCSessionDescriptionInit),
      setRemoteDescription: (d) => this.pc.setRemoteDescription(d as RTCSessionDescriptionInit),
      // 相手の候補は文字列で運ばれてくる。**ここで初めて WebRTC の型へ戻す**
      addIceCandidate: (c) => this.pc.addIceCandidate(JSON.parse(c) as RTCIceCandidateInit),
    };

    // **どの版のフロントが動いているかを、記録の頭に残す。**
    // Rust だけ建て直すと、画面は前の版のまま動く（2026-09-15 に実機で踏んだ）
    log(`記録の版 ${追跡の版}`);

    // **経路の見張りは、支度を待たない**（2026-09-15・Windows で踏んだ）。
    //
    // 以前は `begin()` の中でだけ時計を張っていた。**支度（カメラ・マイク）で
    // 止まると、ICE が `connected` になっても画面は `unknown` のまま**になる ——
    // 実機では「候補は流れ、`connected` まで行っているのに、経路が付かない」
    // という、いちばん切り分けにくい形で出た。
    //
    // **経路を見るのに、こちらの映像は要らない。**だから通話ができた時点で見始める。
    this.見張りを始める();

    this.pc.onicecandidate = (e) => {
      if (!e.candidate) {
        void sendSignal('candidate', '', this.peer);
        return;
      }
      log(候補を言い表す('送る', e.candidate.candidate));
      void sendSignal('candidate', JSON.stringify(e.candidate.toJSON()), this.peer);
    };
    // **移り変わりを残す。**付かなかったとき、どこまで進んだのかが
    // これしか手掛かりにならない（Windows・2026-09-14）
    this.pc.oniceconnectionstatechange = () => {
      const 具合 = this.pc.iceConnectionState;
      log(`経路の具合 ${具合}`);
      if (具合 === 'failed' || 具合 === 'disconnected') void this.様子を記録する();
    };
    this.pc.ontrack = (e) => {
      const [stream] = e.streams;
      if (stream) this.handlers.onRemoteStream(stream);
    };
  }

  /**
   * **支度で取った映像をそのまま使う。**
   *
   * ここで `getUserMedia` を呼び直さない。同じカメラを二重に掴むと、
   * 環境によっては後から取ったほうが失敗する（`NotReadableError`）。
   *
   * **`null` を渡せる。**カメラもマイクも無い機械はある（画面だけの端末、
   * 会場のモニタ、見るだけの人）。そのときは**受け取る枠だけ張る** —
   * 何も足さないと、相手から送る先が無くて**何も流れてこない。**
   */
  async begin(stream: MediaStream | null): Promise<void> {
    if (stream === null) {
      // 送らないが受け取る。**この 2 行が無いと、相手の映像も音も来ない**
      this.pc.addTransceiver('audio', { direction: 'recvonly' });
      this.pc.addTransceiver('video', { direction: 'recvonly' });
      const [next, actions] = onLocalMediaReady(this.state);
      this.state = next;
      for (const action of actions) await applyAction(this.adapter, action, this.送る);
      this.見張りを始める();
      return;
    }
    for (const track of stream.getTracks()) this.pc.addTrack(track, stream);
    // **測る前に映像を出さない**（D29）。枠は最初から張っておき、流すのは測れてから。
    // こうすると、後から足すための張り直し（再交渉）が要らない
    this.local = stream;
    // **押していないなら、音も映像も流さない。**
    // 枠だけ張って黙っている —— ここを `prefs.micOn` にしていたのが音漏れだった
    this.送り直す();

    const [next, actions] = onLocalMediaReady(this.state);
    this.state = next;
    for (const action of actions) await applyAction(this.adapter, action, this.送る);

    this.見張りを始める();
  }

  /** 経路を見に行く時計を張る。**何度呼んでも 1 本だけ。** */
  private 見張りを始める(): void {
    if (this.timer !== null || this.closed) return;
    this.始めた = Date.now();
    this.timer = setInterval(() => void this.pollPath(), STATS_EVERY_MS);
  }

  /** 相手から届いた下ごしらえを 1 通入れる。 */
  async receive(payload: SignalPayload): Promise<void> {
    if (this.closed) return;
    // `end`（もう候補は無い）は状態を動かさない
    if (payload.step === 'end' || payload.blob === '') return;
    if (payload.step === 'candidate') this.来た候補を記録する(payload.blob);
    const step = payload.step === 'candidate' ? 'ice' : payload.step;
    const [next, actions] = onRemote(this.state, step, payload.blob);
    this.state = next;
    for (const action of actions) await applyAction(this.adapter, action, this.送る);
  }

  /** 経路を見に行き、**落ち着かせてから**画面へ渡す。 */
  private async pollPath(): Promise<void> {
    if (this.closed) return;
    const report = await this.pc.getStats();
    const stats: RtcStatLike[] = [...report.values()] as RtcStatLike[];

    // **付かないまま 15 秒経ったら、そのときの様子を 1 回だけ残す。**
    // 「付かなかった」しか残らないのを終わらせる（#17・2026-09-14）
    if (
      !this.様子を出した &&
      this.watch.shown === 'unknown' &&
      Date.now() - this.始めた >= 様子を出すまでのミリ秒
    ) {
      this.様子を出した = true;
      log(`${様子を出すまでのミリ秒 / 1000} 秒たっても経路がありません。${数えて言い表す(stats as 統計の行[])}`);
      // **組の中身まで出す。**`connected` なのに `unknown` という形を
      // 2026-09-14 に Windows で踏んだ。数だけでは、どちらが引けなかったのか分からない
      for (const 行 of 組の様子(stats as 統計の行[])) log(行);
      log(送り受けを言い表す(stats as 統計の行[]));
      const 組 = 対を言い表す(stats as 統計の行[]);
      if (組) log(組);
    }

    // **映像の向きが変わったら伝える**（**#39**）——
    // 「受けているのに送っていない」を画面が言えるようにする
    const 向き = 映像の向き(stats as 統計の行[]);
    if (
      this.前の向き === null ||
      this.前の向き.送っている !== 向き.送っている ||
      this.前の向き.受けている !== 向き.受けている
    ) {
      this.前の向き = 向き;
      this.handlers.on映像の向き?.(向き);
    }

    const before = this.watch.shown;
    this.watch = observe(this.watch, pathFromStats(stats));
    if (this.watch.shown === before) return;

    // **どの組で付いたかを 1 回だけ残す。**
    // **`unknown` でも出す** —— 2026-09-14、`unknown` の回にこそ組を見たかったのに、
    // ここに `!== 'unknown'` と書いていたせいで**いちばん要る回に 1 行も出なかった。**
    if (!this.組を出した) {
      const 組 = 対を言い表す(stats as 統計の行[]);
      if (組) {
        this.組を出した = true;
        log(組);
        // **経路が付いた回に、送り受けの数も残す**（2026-09-15）——
        // 「経路は direct なのに映像が来ない」を、**送り側と受け側に切り分ける**
        log(送り受けを言い表す(stats as 統計の行[]));
      }
    }
      // **ハウリングの危険を、危なくなった時に言う**（2026-09-17・オーナー依頼）。
    // 支度のときだけ案内していたが、**危なくなるのは 2 人目が音を出した時**である
    const 同じ網 = 同じ網に居るか(stats as 統計の行[]);
    if (同じ網 !== null) {
      const 音 = 音の向き(stats as 統計の行[]);
      this.handlers.onHowlingRisk?.(
        ハウリングの危険({
          音を送っている: 音.送っている,
          音を受けている数: 音.受けている ? 1 : 0,
          同じ網の相手: 同じ網 ? 1 : 0,
        }),
      );
    }
    this.handlers.onPath(this.watch.shown);
    // 測れたら映像を流す。測れなくなったら止める（D29）。
    // **支度で「カメラ切」にしていたら、測れても流さない** — 人の指定が優先する
    this.送り直す();
  }

  /** 来た候補を、送ったものと**同じ形で**残す。並べて読めないと突き合わせられない。 */
  private 来た候補を記録する(blob: string): void {
    try {
      const 中身 = JSON.parse(blob) as { candidate?: string };
      if (中身.candidate) log(候補を言い表す('来た', 中身.candidate));
    } catch {
      // 握り潰す理由: 記録のためだけの処理で、ここで止めると通話そのものが壊れる。
      // 読めなかったことは、この下の行（実際の追加）が断る
    }
  }

  /** いまの様子を 1 回だけ残す。**同じことを何度も書かない。** */
  private async 様子を記録する(): Promise<void> {
    if (this.closed || this.様子を出した) return;
    this.様子を出した = true;
    const report = await this.pc.getStats();
    log(数えて言い表す([...report.values()] as 統計の行[]));
  }

  /** 映像の枠はそのままに、流すかどうかだけを切り替える。 */
  private setVideoEnabled(on: boolean): void {
    for (const track of this.local?.getVideoTracks() ?? []) track.enabled = on;
  }

  private setAudioEnabled(on: boolean): void {
    for (const track of this.local?.getAudioTracks() ?? []) track.enabled = on;
  }

  /** この通話の相手へ 1 通送る。**中身は解釈しない。** */
  private 送る = (step: 'offer' | 'answer' | 'candidate', blob: string) => {
    void sendSignal(step, blob, this.peer);
  };

  /**
   * 送っているものを入れ替える。**通話を張り直さない。**
   *
   * 会議中に支度をやり直すと、前のトラックは `stop()` されている。
   * 入れ替えないまま放っておくと、**相手には静止画のあと真っ黒が映る**
   * （2026-09-04 に実機で踏んだ）。
   */
  async replaceTracks(stream: MediaStream | null): Promise<void> {
    if (this.closed) return;
    this.local = stream;
    for (const tr of this.pc.getTransceivers()) {
      const kind = tr.sender.track?.kind ?? tr.receiver.track?.kind;
      if (kind !== 'audio' && kind !== 'video') continue;
      const 次 = kind === 'audio' ? (stream?.getAudioTracks()[0] ?? null) : (stream?.getVideoTracks()[0] ?? null);
      try {
        await tr.sender.replaceTrack(次);
      } catch {
        // 握り潰す理由: 1 本の入れ替えに失敗しても、残りは入れ替える。
        // ここで止めると、音だけ古いまま・映像だけ新しい、という半端な状態で固まる
      }
    }
    this.送り直す();
  }

  /** 会議中に入と切を変える。**支度で決めた値を上書きする。** */
  setPrefs(prefs: Prefs): void {
    this.prefs = prefs;
    this.送り直す();
  }

  /**
   * **この部屋で映像と音を足す／やめる。**
   *
   * **これを入にするまで、音も映像も流れない**（**D113**）。
   * 切にすると**その場で止まる** —— トラックは持ったままなので、
   * 入れ直すのに張り直し（再交渉）は要らない。
   */
  映像と音を足す(足す: boolean): void {
    this.映像を使う = 足す;
    this.送り直す();
  }

  /**
   * **送ってよいものを 1 か所で決めて、トラックへ入れる。**
   *
   * 判断は [`送ってよいか`] に置いてある（試験できる形）。
   * **ここでは入れるだけ** —— 条件をここに書くと、また片方だけ抜ける。
   */
  private 送り直す(): void {
    const 送る = 送ってよいか({
      映像を使う: this.映像を使う,
      マイク入: this.prefs.micOn,
      カメラ入: this.prefs.cameraOn,
      測れた: shouldSendVideo(this.watch.shown),
    });
    this.setAudioEnabled(送る.音);
    // **映像は、まだこの関門を通していない。**
    //
    // 支度の「自分の姿」と送り手が**同じトラックを見ている**ので、
    // ここで `enabled = false` にすると**自分の姿まで黒くなる。**
    // **送らずに自分だけ見る**には `sender.replaceTrack(null)` が要る ——
    // それは支度と参加を分ける仕事（**#39 の 3**）でやる。
    //
    // **漏れていたのは音だけである**（映像は 3 回とも実測 0 パケット）。
    // 直し方が要る所と、いま漏れている所を、混ぜない
    this.setVideoEnabled(送る.映像 || (this.prefs.cameraOn && shouldSendVideo(this.watch.shown)));
  }

  close(): void {
    this.closed = true;
    if (this.timer) clearInterval(this.timer);
    this.pc.close();
  }
}


