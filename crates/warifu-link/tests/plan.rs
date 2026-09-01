//! 回線から割り当てを決める。
//!
//! フルメッシュは各自が `K×(N−1)` を上下とも負う（`decisions.md` **D9** / **D27**）。
//! 12 人 720p なら 16.5 Mbps。**携帯回線では出ない。**
//!
//! そこで**回線の種類を当てるのではなく、測った値で決める。**
//! 「携帯かどうか」を判定する形にすると、
//! 携帯でも速い回線と、光でも混んでいる回線を、両方とも読み違える。

use warifu_link::{Error, FRESH_FOR, Link, Quality, plan};

const 今: u64 = 1_756_800_000;

/// Mbps で書くための助け。
fn mbps(v: f64) -> u64 {
    (v * 1_000_000.0) as u64
}

fn 回線(上り: f64, 下り: f64) -> Link {
    Link::new(mbps(上り), mbps(下り), 今)
}

#[test]
fn 速い回線なら高い画質が出る() {
    // 光どうし 4 人。上下 100 Mbps あれば 1080p が通る
    let 自分 = 回線(100.0, 100.0);
    let 相手 = vec![回線(100.0, 100.0); 3];

    let 割り当て = plan(&自分, &相手, 今).unwrap();

    assert_eq!(割り当て.send(), &[Quality::P1080; 3]);
    assert_eq!(割り当て.receive(), &[Quality::P1080; 3]);
}

#[test]
fn 遅い相手には落として送る() {
    // **相手ごとに変える。**1 人が携帯でも、他の人まで巻き添えにしない
    let 自分 = 回線(100.0, 100.0);
    let 相手 = vec![回線(100.0, 100.0), 回線(1.0, 1.0)];

    let 割り当て = plan(&自分, &相手, 今).unwrap();

    assert_eq!(割り当て.send()[0], Quality::P1080);
    assert!(
        割り当て.send()[1] < Quality::P1080,
        "遅い相手にも同じ画質を送っています: {:?}",
        割り当て.send()
    );
}

#[test]
fn 自分の上りを超えて送らない() {
    // **相手の申告で自分の回線を超えない。**相手が「下り 1 Gbps」と言っても、
    // 出せるのは自分の上りまで
    let 自分 = 回線(5.0, 100.0);
    let 相手 = vec![回線(1000.0, 1000.0); 4];

    let 割り当て = plan(&自分, &相手, 今).unwrap();

    let 合計: u64 = 割り当て.send().iter().map(|q| q.bitrate_bps()).sum();
    assert!(
        合計 <= mbps(5.0),
        "上り 5 Mbps に対して {合計} bps 送ろうとしています"
    );
}

#[test]
fn 自分の下りを超えて受け取らない() {
    let 自分 = 回線(100.0, 5.0);
    let 相手 = vec![回線(1000.0, 1000.0); 4];

    let 割り当て = plan(&自分, &相手, 今).unwrap();

    let 合計: u64 = 割り当て.receive().iter().map(|q| q.bitrate_bps()).sum();
    assert!(
        合計 <= mbps(5.0),
        "下り 5 Mbps に対して {合計} bps 受けようとしています"
    );
}

#[test]
fn 人が増えれば一人あたりが下がる() {
    let 自分 = 回線(20.0, 20.0);
    let 速い = 回線(100.0, 100.0);

    let 四人 = plan(&自分, &[速い; 3], 今).unwrap();
    let 十二人 = plan(&自分, &[速い; 11], 今).unwrap();

    assert!(
        十二人.receive()[0] < 四人.receive()[0],
        "12 人でも 4 人と同じ画質を受けようとしています"
    );
}

#[test]
fn 足りなければ画質を落として人は落とさない() {
    // **落とすのは画質であって人ではない。**入れないのは、音声すら通らないときだけ
    let 自分 = 回線(1.5, 1.5);
    let 相手 = vec![回線(100.0, 100.0); 11];

    let 割り当て = plan(&自分, &相手, 今).unwrap();

    assert_eq!(割り当て.receive().len(), 11, "人を減らしています");
    assert!(割り当て.receive().iter().all(|q| *q <= Quality::P180));
}

#[test]
fn 音声ぶんも通らなければ参加できないと言う() {
    // 黙って 0 人ぶんを返さない。**通らないなら通らないと言う**
    let 自分 = 回線(0.1, 0.1);
    let 相手 = vec![回線(100.0, 100.0); 11];

    assert_eq!(plan(&自分, &相手, 今).unwrap_err(), Error::TooWeak);
}

#[test]
fn 音声は最後まで残る() {
    // 映像から先に落ちる。**声が切れるくらいなら映像を捨てる**
    let 自分 = 回線(0.5, 0.5);
    let 相手 = vec![回線(100.0, 100.0); 11];

    let 割り当て = plan(&自分, &相手, 今).unwrap();

    assert!(割り当て.receive().iter().all(|q| *q == Quality::AudioOnly));
}

#[test]
fn 古い測定値は使わない() {
    // **回線は変わる。**30 分前の実測で今の割り当てを決めない
    let 古い = Link::new(mbps(100.0), mbps(100.0), 今 - FRESH_FOR - 1);

    assert_eq!(
        plan(&古い, &[回線(100.0, 100.0)], 今).unwrap_err(),
        Error::Stale
    );
    // ちょうどは使う
    let ぎりぎり = Link::new(mbps(100.0), mbps(100.0), 今 - FRESH_FOR);
    assert!(plan(&ぎりぎり, &[回線(100.0, 100.0)], 今).is_ok());
}

#[test]
fn 相手の測定値が古ければその相手にだけ控えめに送る() {
    // 相手 1 人が古いからといって、会議ぜんぶを止めない
    let 自分 = 回線(100.0, 100.0);
    let 古い相手 = Link::new(mbps(100.0), mbps(100.0), 今 - FRESH_FOR - 1);

    let 割り当て = plan(&自分, &[回線(100.0, 100.0), 古い相手], 今).unwrap();

    assert_eq!(割り当て.send()[0], Quality::P1080);
    assert_eq!(
        割り当て.send()[1],
        Quality::AudioOnly,
        "古い測定値をそのまま信じています"
    );
}

#[test]
fn 相手が一人もいなければ何も割り当てない() {
    let 割り当て = plan(&回線(100.0, 100.0), &[], 今).unwrap();

    assert!(割り当て.send().is_empty());
    assert!(割り当て.receive().is_empty());
}

#[test]
fn 十二人_720p_には上下_16_5_mbps_要る() {
    // D27 の表と実装を突き合わせる。**表だけが正しくても意味が無い**
    let 要る = Quality::P720.bitrate_bps() * 11;
    assert_eq!(要る, 16_500_000);

    // 20 Mbps では届かない（安全率を引くため）。25 Mbps なら届く
    let 速い = 回線(100.0, 100.0);
    assert!(plan(&回線(100.0, 20.0), &[速い; 11], 今).unwrap().receive()[0] < Quality::P720);
    assert!(plan(&回線(100.0, 25.0), &[速い; 11], 今).unwrap().receive()[0] >= Quality::P720);
}
