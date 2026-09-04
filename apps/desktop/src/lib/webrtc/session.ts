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
import { sendSignal, type SignalPayload } from '../bridge';
import { applyAction, type PeerLike } from './apply';
import { ICE_SERVERS, shouldSendVideo } from './media';
import type { Prefs } from './devices';
import { onLocalMediaReady, onRemote, start, type NegotiationState } from './negotiation';

/** 経路を見に行く間隔。短くしても、`watch.ts` が表示を落ち着かせる。 */
const STATS_EVERY_MS = 1000;

export interface CallHandlers {
  /** 相手の映像。 */
  onRemoteStream(stream: MediaStream): void;
  /** 表示すべき経路（直接 / 中継 / 不明）。 */
  onPath(path: LinkPath): void;
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

    this.pc.onicecandidate = (e) => {
      if (!e.candidate) {
        void sendSignal('candidate', '', this.peer);
        return;
      }
      void sendSignal('candidate', JSON.stringify(e.candidate.toJSON()), this.peer);
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
   */
  async begin(stream: MediaStream): Promise<void> {
    for (const track of stream.getTracks()) this.pc.addTrack(track, stream);
    // **測る前に映像を出さない**（D29）。枠は最初から張っておき、流すのは測れてから。
    // こうすると、後から足すための張り直し（再交渉）が要らない
    this.local = stream;
    this.setVideoEnabled(false);
    // 支度で「マイク切」にしていたら、入室してもそのまま切のまま
    this.setAudioEnabled(this.prefs.micOn);

    const [next, actions] = onLocalMediaReady(this.state);
    this.state = next;
    for (const action of actions) await applyAction(this.adapter, action, this.送る);

    this.timer = setInterval(() => void this.pollPath(), STATS_EVERY_MS);
  }

  /** 相手から届いた下ごしらえを 1 通入れる。 */
  async receive(payload: SignalPayload): Promise<void> {
    if (this.closed) return;
    // `end`（もう候補は無い）は状態を動かさない
    if (payload.step === 'end' || payload.blob === '') return;
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
    const before = this.watch.shown;
    this.watch = observe(this.watch, pathFromStats(stats));
    if (this.watch.shown === before) return;
    this.handlers.onPath(this.watch.shown);
    // 測れたら映像を流す。測れなくなったら止める（D29）。
    // **支度で「カメラ切」にしていたら、測れても流さない** — 人の指定が優先する
    this.setVideoEnabled(this.prefs.cameraOn && shouldSendVideo(this.watch.shown));
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

  /** 会議中に入と切を変える。**支度で決めた値を上書きする。** */
  setPrefs(prefs: Prefs): void {
    this.prefs = prefs;
    this.setAudioEnabled(prefs.micOn);
    this.setVideoEnabled(prefs.cameraOn && shouldSendVideo(this.watch.shown));
  }

  close(): void {
    this.closed = true;
    if (this.timer) clearInterval(this.timer);
    this.pc.close();
  }
}


