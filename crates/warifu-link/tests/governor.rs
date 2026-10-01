//! 粗さの自動調整。
//!
//! `plan` は**その瞬間の**割り当てを出すだけで、回線が変われば合わなくなる。
//! ここは**時間の経過ごと**に段を上げ下げする。
//!
//! # 素朴に作ると必ず振動する
//!
//! 「入るなら上げる・入らないなら下げる」を毎回やると、
//! **境目で 1 秒ごとに段が変わる。**見ている側には、
//! 粗い映像より**粗さが変わり続ける映像のほうが辛い。**
//!
//! だから 3 つ入れる — **上げ下げの境目をずらす / 上げるのは様子を見てから /
//! 上げるのは 1 段ずつ**。

use warifu_link::{Governor, Quality, RAISE_AFTER, Sample};

const 今: u64 = 1_756_800_000;

fn mbps(v: f64) -> u64 {
    (v * 1_000_000.0) as u64
}

/// 取りこぼしの無い観測。
fn 順調(bps: u64) -> Sample {
    Sample::new(bps, 0)
}

#[test]
fn 始まりは音声だけ() {
    // **測る前に映像を出さない。**「測っていない」を「たぶん速い」にしない
    assert_eq!(Governor::new().quality(), Quality::AudioOnly);
}

#[test]
fn 余裕があってもすぐには上げない() {
    let mut 調速 = Governor::new();

    // 1 秒後。余裕は充分にある
    assert_eq!(調速.observe(順調(mbps(100.0)), 今 + 1), Quality::AudioOnly);
}

#[test]
fn 余裕が続けば上がる() {
    let mut 調速 = Governor::new();
    調速.observe(順調(mbps(100.0)), 今 + 1);

    let 段 = 調速.observe(順調(mbps(100.0)), 今 + 1 + RAISE_AFTER);

    assert_eq!(段, Quality::P180, "音声の次は 1 段だけ上がる");
}

#[test]
fn 一度に上げるのは一段だけ() {
    // 100 Mbps あっても、いきなり 1080p にしない。
    // **上げて落ちるより、上げないほうがまし**
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;

    for _ in 0..3 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(mbps(100.0)), 時刻);
    }

    assert_eq!(調速.quality(), Quality::P360, "3 回で 3 段");
}

#[test]
fn 足りなくなったら即落ちる() {
    // **落とすのは速く、上げるのはゆっくり。**待っている間に映像は壊れる
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;
    for _ in 0..5 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(mbps(100.0)), 時刻);
    }
    assert_eq!(調速.quality(), Quality::P720);

    let 段 = 調速.observe(順調(mbps(0.5)), 時刻 + 1);

    assert!(段 < Quality::P720, "落ちていません: {段:?}");
}

#[test]
fn 落ちるときは何段でも落ちる() {
    // 上げるのは 1 段ずつだが、**落ちるのは収まる所まで一気に**
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;
    for _ in 0..5 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(mbps(100.0)), 時刻);
    }
    assert_eq!(調速.quality(), Quality::P720);

    let 段 = 調速.observe(順調(mbps(0.25)), 時刻 + 1);

    assert_eq!(段, Quality::P180, "収まる所まで一気に落ちていません");
}

#[test]
fn 取りこぼしが出たら帯域が足りていても落とす() {
    // **ここが要。**取りこぼしながら送っていると、
    // 見かけの流量は出ているのに映像は壊れている。
    // 流量だけを見ていると、壊れたまま「足りている」と判断し続ける
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;
    for _ in 0..4 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(mbps(100.0)), 時刻);
    }
    let 前 = 調速.quality();

    // 流量は充分。だが 3% 取りこぼしている
    let 段 = 調速.observe(Sample::new(mbps(100.0), 30), 時刻 + 1);

    assert!(段 < 前, "取りこぼしを無視しています: {前:?} → {段:?}");
}

#[test]
fn 境目で振動しない() {
    // 上げる境目を、今の段の帯域そのものにしない。
    // **同じ境目で上げ下げすると、1 秒ごとに段が変わる**
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;
    時刻 += RAISE_AFTER + 1;
    調速.observe(順調(mbps(100.0)), 時刻); // → P180

    // P360 が「ちょうど入る」量。**ちょうどでは上げない**
    let ちょうど = Quality::P360.bitrate_bps();
    for _ in 0..5 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(ちょうど), 時刻);
    }

    assert_eq!(調速.quality(), Quality::P180, "ちょうどで上げています");
}

#[test]
fn 余裕が途切れたら数え直す() {
    let mut 調速 = Governor::new();
    調速.observe(順調(mbps(100.0)), 今 + 1);
    // あと少しで上がる所で、一度細くなる
    調速.observe(順調(mbps(0.05)), 今 + RAISE_AFTER);
    // また太くなったが、**数え直しなのでまだ上がらない**
    let 段 = 調速.observe(順調(mbps(100.0)), 今 + RAISE_AFTER + 1);

    assert_eq!(段, Quality::AudioOnly);
}

#[test]
fn 音声より下には落ちない() {
    let mut 調速 = Governor::new();

    let 段 = 調速.observe(Sample::new(0, 500), 今 + 1);

    assert_eq!(段, Quality::AudioOnly, "音声を切っています");
}

#[test]
fn 時刻が戻っても壊れない() {
    // 時計は戻ることがある（NTP の補正）。**戻ったぶんを経過時間として数えない**
    let mut 調速 = Governor::new();
    調速.observe(順調(mbps(100.0)), 今 + 100);

    let 段 = 調速.observe(順調(mbps(100.0)), 今);

    assert_eq!(段, Quality::AudioOnly, "時刻が戻ったのに上がりました");
}

#[test]
fn 今の段は何度でも読める() {
    let 調速 = Governor::new();
    assert_eq!(調速.quality(), 調速.quality());
}

#[test]
fn 割り当てを超えて上げない() {
    // 回線が空いていても、**持ち分は持ち分**。
    // 頭が無いと、1 人が空いている帯域を全部使い、他の人ぶんが無くなる
    let mut 調速 = Governor::new();
    調速.set_ceiling(Quality::P360);

    let mut 時刻 = 今;
    for _ in 0..5 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(mbps(100.0)), 時刻);
    }

    assert_eq!(調速.quality(), Quality::P360, "頭を超えました");
}

#[test]
fn 頭を下げれば今の段もその場で下がる() {
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;
    for _ in 0..4 {
        時刻 += RAISE_AFTER + 1;
        調速.observe(順調(mbps(100.0)), 時刻);
    }
    assert_eq!(調速.quality(), Quality::P540);

    調速.set_ceiling(Quality::P180);

    assert_eq!(調速.quality(), Quality::P180);
}

#[test]
fn 出している量を入れると上がらない() {
    // **この層が受け取るのは「出せると見込まれる量」であって「出している量」ではない。**
    // 取り違えたときにどうなるかを、ここに残しておく。
    //
    // 180p で送っていれば流れるのは 200 kbps しかなく、
    // それを入れ続けると「360p に上げる余裕は無い」と毎回判断する。
    // **自分が絞っているせいで上げられない**という循環になる。
    let mut 調速 = Governor::new();
    let mut 時刻 = 今;
    // 1 回目は経過 0 なので上がらない。2 回目で P180 になる
    調速.observe(順調(mbps(100.0)), 時刻);
    時刻 += RAISE_AFTER + 1;
    調速.observe(順調(mbps(100.0)), 時刻);
    assert_eq!(調速.quality(), Quality::P180);

    for _ in 0..10 {
        時刻 += RAISE_AFTER + 1;
        // 今の段ぶんしか入れない（＝出している量を入れてしまった場合）
        調速.observe(順調(調速.quality().bitrate_bps()), 時刻);
    }

    assert_eq!(調速.quality(), Quality::P180, "上がってしまいました");
}
