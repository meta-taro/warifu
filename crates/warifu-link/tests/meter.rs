//! 実際に流れた量から回線を測る。**申告ではなく観測。**
//!
//! 相手に「あなたの回線は何 Mbps ですか」と尋ねる形にしない。
//! 尋ねれば、**多く送ってほしい側は多めに答える**（`warifu-read` で
//! 送信者に優先度を申告させなかったのと同じ理屈・**D18**）。

use warifu_link::{FRESH_FOR, Meter};

const 今: u64 = 1_756_800_000;

#[test]
fn 流れた量から速さを出す() {
    let mut 計 = Meter::new();
    // 2 秒で 2.5 MB = 10 Mbps
    計.observe(2_500_000, 今 - 2, 今);

    let 回線 = 計.measured(今).unwrap();

    assert_eq!(回線, 10_000_000);
}

#[test]
fn 観測が無ければ測れないと言う() {
    // **0 Mbps と答えない。**「測っていない」と「0」は別のこと
    assert!(Meter::new().measured(今).is_none());
}

#[test]
fn 長さ_0_の観測は捨てる() {
    let mut 計 = Meter::new();
    計.observe(1_000_000, 今, 今);

    assert!(計.measured(今).is_none(), "0 秒で割っています");
}

#[test]
fn 一番速かった観測を採る() {
    // 途中で何も送っていない区間があると、平均は実力より低く出る。
    // **回線の太さを知りたいので、出せた最大を採る**
    let mut 計 = Meter::new();
    計.observe(1_250_000, 今 - 11, 今 - 10); // 10 Mbps
    計.observe(125_000, 今 - 6, 今 - 5); //  1 Mbps（送るものが無かった区間）

    assert_eq!(計.measured(今).unwrap(), 10_000_000);
}

#[test]
fn 古い観測は落ちる() {
    // **回線は変わる。**古い実測をいつまでも「実力」として持ち続けない
    let mut 計 = Meter::new();
    計.observe(1_250_000, 今 - FRESH_FOR - 2, 今 - FRESH_FOR - 1);

    assert!(計.measured(今).is_none());
}

#[test]
fn 新しい観測が古いものを押し出す() {
    let mut 計 = Meter::new();
    計.observe(12_500_000, 今 - FRESH_FOR - 2, 今 - FRESH_FOR - 1); // 100 Mbps・古い
    計.observe(125_000, 今 - 2, 今 - 1); // 1 Mbps・新しい

    assert_eq!(
        計.measured(今).unwrap(),
        1_000_000,
        "古い最大値が残っています"
    );
}

#[test]
fn 未来の観測は受け取らない() {
    // 時計がずれた相手の値をそのまま入れると、いつまでも古くならない観測ができる
    let mut 計 = Meter::new();
    計.observe(1_250_000, 今 + 10, 今 + 11);

    assert!(計.measured(今).is_none());
}
