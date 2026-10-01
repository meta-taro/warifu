//! 預かり所。**相手が起動していない間、封を預かる。**
//!
//! これが LINE との本当の差だった —— LINE は中央のサーバが預かるから、
//! 相手が寝ていても届く。割符は**サーバーを 1 台も立てない**ので、
//! 2026-09-08 まで**相手が起動していなければ消えていた。**
//!
//! **預かり所は「割符が用意する中央」ではない。**
//! **導入した人が自分で立てるもの**である（**D68**）。

use warifu_core::{Device, PublicKey, Seed};
use warifu_post::{Box as 私書箱, Error, Item, 預かる上限, 預かれる日数};

fn 鍵(seed: [u8; 32]) -> PublicKey {
    let d: Device = Seed::from_bytes(seed).profile("Personal").device("PC");
    d.public_key()
}

fn 封(中身: &str) -> Vec<u8> {
    warifu_seal::seal(鍵([9u8; 32]), 中身.as_bytes())
        .unwrap()
        .to_bytes()
}

#[test]
fn 預けたものを_その人が受け取れる() {
    let mut 箱 = 私書箱::new();
    箱.put(鍵([2u8; 32]), 封("こんばんは"), 100).unwrap();

    let 受け取り = 箱.take(鍵([2u8; 32]), 100);

    assert_eq!(受け取り.len(), 1);
}

#[test]
fn 受け取ったら_預かり所から消える() {
    // **溜め込む場所にしない。**渡したら手放す
    let mut 箱 = 私書箱::new();
    箱.put(鍵([2u8; 32]), 封("x"), 100).unwrap();

    箱.take(鍵([2u8; 32]), 100);

    assert!(箱.take(鍵([2u8; 32]), 100).is_empty());
}

#[test]
fn 別人は受け取れない() {
    // **宛先の人だけが受け取る**
    let mut 箱 = 私書箱::new();
    箱.put(鍵([2u8; 32]), 封("x"), 100).unwrap();

    assert!(箱.take(鍵([3u8; 32]), 100).is_empty());
}

#[test]
fn 預かり所は中身を読まない() {
    // **封のまま持つ。**開ける鍵を持っていないので、読みようがない（D69）
    let mut 箱 = 私書箱::new();
    let 中身 = 封("SHIRUSHI-honbun-post");
    箱.put(鍵([2u8; 32]), 中身.clone(), 100).unwrap();

    let 出た: Vec<Item> = 箱.take(鍵([2u8; 32]), 100);

    assert_eq!(出た[0].sealed(), &中身[..]);
}

#[test]
fn 古いものは渡さない() {
    // **溜め込む場所にしない。**相手が次に起動するまでが預かる理由である
    let mut 箱 = 私書箱::new();
    let 昔 = 100;
    箱.put(鍵([2u8; 32]), 封("古い"), 昔).unwrap();

    let あと = 昔 + 預かれる日数 * 24 * 60 * 60 + 1;

    assert!(箱.take(鍵([2u8; 32]), あと).is_empty());
}

#[test]
fn 古いものは_受け取らなくても片付く() {
    let mut 箱 = 私書箱::new();
    箱.put(鍵([2u8; 32]), 封("古い"), 100).unwrap();

    箱.forget_old(100 + 預かれる日数 * 24 * 60 * 60 + 1);

    assert_eq!(箱.len(), 0);
}

#[test]
fn 一人あたりの上限を超えたら断る() {
    // **無限に置ける所を作らない。**置くだけで機械を埋められる
    let mut 箱 = 私書箱::new();
    for _ in 0..預かる上限 {
        箱.put(鍵([2u8; 32]), 封("x"), 100).unwrap();
    }

    assert_eq!(箱.put(鍵([2u8; 32]), 封("あふれる"), 100), Err(Error::Full));
}

#[test]
fn 断られても_他の人の分は減らない() {
    // **1 人が埋めても、他の人が使えなくならない**
    let mut 箱 = 私書箱::new();
    for _ in 0..預かる上限 {
        箱.put(鍵([2u8; 32]), 封("x"), 100).unwrap();
    }

    assert!(箱.put(鍵([3u8; 32]), 封("べつの人"), 100).is_ok());
}

#[test]
fn 大きすぎるものは預からない() {
    let mut 箱 = 私書箱::new();
    let 大きい = vec![0u8; warifu_post::一通の上限 + 1];

    assert_eq!(箱.put(鍵([2u8; 32]), 大きい, 100), Err(Error::TooLarge));
}

#[test]
fn 預けた順に受け取る() {
    // **並び替えない。**話の順が変わると読めなくなる
    let mut 箱 = 私書箱::new();
    箱.put(鍵([2u8; 32]), 封("1"), 100).unwrap();
    箱.put(鍵([2u8; 32]), 封("2"), 101).unwrap();

    let 出た = 箱.take(鍵([2u8; 32]), 102);

    assert_eq!(出た[0].at(), 100);
    assert_eq!(出た[1].at(), 101);
}

// ── 預かり所とのやり取り ──

use warifu_post::{Ask, Reply};

#[test]
fn 預けるときは_宛先と封だけを渡す() {
    // **差出人を名乗らない。**名乗ると、預かり所が「誰が誰に」を読める（D69）
    let 元 = Ask::Put {
        to: 鍵([2u8; 32]),
        sealed: 封("x"),
    };
    assert_eq!(Ask::from_bytes(&元.to_bytes()).unwrap(), 元);
}

#[test]
fn 受け取りは_自分あてを尋ねるだけ() {
    // **誰あてかは経路が決める。**尋ねる側が宛先を名乗ると、
    // **他人あてを引き取れる**ことになる
    let 元 = Ask::Take;
    assert_eq!(Ask::from_bytes(&元.to_bytes()).unwrap(), 元);
}

#[test]
fn 預かり所の返事を読める() {
    let 元 = Reply::Kept;
    assert_eq!(Reply::from_bytes(&元.to_bytes()).unwrap(), 元);
    let 断り = Reply::Refused;
    assert_eq!(Reply::from_bytes(&断り.to_bytes()).unwrap(), 断り);
}

#[test]
fn 渡すときは_封を並べる() {
    let 元 = Reply::Handed(vec![封("1"), 封("2")]);
    assert_eq!(Reply::from_bytes(&元.to_bytes()).unwrap(), 元);
}

#[test]
fn 何も無いことを_断りと混ぜない() {
    // **「預かっていない」と「断った」は別。**混ぜると、
    // 受け取りに来た人が「拒まれた」と読む
    let 空 = Reply::Handed(Vec::new());
    assert_ne!(空, Reply::Refused);
    assert_eq!(Reply::from_bytes(&空.to_bytes()).unwrap(), 空);
}

#[test]
fn 壊れた塊は読めない() {
    assert!(Ask::from_bytes(b"").is_none());
    assert!(Ask::from_bytes("なにこれ".as_bytes()).is_none());
    assert!(Reply::from_bytes(b"").is_none());
}
