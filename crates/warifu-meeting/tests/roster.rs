//! 会議の名簿。
//!
//! 定員は**会議ごとに決める**（`decisions.md` **D27**）。既定は 4 人だが、上限ではない。
//!
//! フルメッシュは各自が**自分の映像だけ**を全員へ直接送る。
//! 5 人でも 6 人でも**誰も他人の通信を中継しない**ので、D7 は発火しない。
//! 効くのは上りの帯域（`K×(N−1)`）で、外枠は [`HARD_LIMIT`]。
//!
//! **定員を「運用で気をつける」にしない。**送る側でも受け取る側でも数える。

use warifu_core::{PublicKey, Seed};
use warifu_meeting::{DEFAULT_CAPACITY, Error, HARD_LIMIT, MeetingId, Notice, Roster};

fn 鍵(seed: u8) -> PublicKey {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

#[test]
fn 会議は主催者ひとりから始まる() {
    let 名簿 = Roster::new(鍵(1));

    assert_eq!(名簿.host(), 鍵(1));
    assert_eq!(名簿.len(), 1, "主催者も参加者のひとり");
    assert!(名簿.contains(&鍵(1)));
}

#[test]
fn 四人まで入れる() {
    let mut 名簿 = Roster::new(鍵(1));

    名簿.add(鍵(2)).unwrap();
    名簿.add(鍵(3)).unwrap();
    名簿.add(鍵(4)).unwrap();

    assert_eq!(名簿.len(), DEFAULT_CAPACITY);
    assert!(名簿.is_full());
}

#[test]
fn 既定では五人目が入れない() {
    // 既定は控えめ（4 人）。**上限ではなく既定**なので、増やしたければ定員を決めて始める
    let mut 名簿 = Roster::new(鍵(1));
    名簿.add(鍵(2)).unwrap();
    名簿.add(鍵(3)).unwrap();
    名簿.add(鍵(4)).unwrap();

    assert!(matches!(名簿.add(鍵(5)), Err(Error::Full)));
    assert_eq!(名簿.len(), DEFAULT_CAPACITY, "断った分が混ざっていない");
}

#[test]
fn 定員を決めれば五人以上でも入れる() {
    // フルメッシュは誰も他人の通信を中継しないので、ここに D7 は掛からない（D27）
    let mut 名簿 = Roster::with_capacity(鍵(1), 8).unwrap();
    for i in 2..=8 {
        名簿.add(鍵(i)).unwrap();
    }

    assert_eq!(名簿.len(), 8);
    assert!(名簿.is_full());
    assert!(matches!(名簿.add(鍵(9)), Err(Error::Full)));
}

#[test]
fn 外枠は超えられない() {
    // 超えたいなら誰かが他人の映像を運ぶことになる。**そこで初めて D7 の決着が要る**
    assert!(Roster::with_capacity(鍵(1), HARD_LIMIT).is_ok());
    assert!(matches!(
        Roster::with_capacity(鍵(1), HARD_LIMIT + 1),
        Err(Error::Full)
    ));
}

#[test]
fn 一人の会議は作れない() {
    assert!(matches!(Roster::with_capacity(鍵(1), 1), Err(Error::Full)));
    assert!(Roster::with_capacity(鍵(1), 2).is_ok());
}

#[test]
fn 招待は定員も運ぶ() {
    // 運ばないと、受け取った側は**その会議が何人までなのか**を知らず、外枠でしか数えられない
    let mut 名簿 = Roster::with_capacity(鍵(1), 6).unwrap();
    名簿.add(鍵(2)).unwrap();

    let 塊 = Notice::Invite {
        meeting: MeetingId::generate(),
        roster: 名簿.clone(),
    }
    .to_intent()
    .unwrap();

    match Notice::from_intent(&塊).unwrap() {
        Notice::Invite { roster, .. } => {
            assert_eq!(roster.capacity(), 6, "定員が運ばれていません");
            assert_eq!(roster.len(), 2);
        }
        その他 => panic!("招待として読めない: {その他:?}"),
    }
}

#[test]
fn 同じ人を二度入れない() {
    let mut 名簿 = Roster::new(鍵(1));
    名簿.add(鍵(2)).unwrap();

    // 二重に数えると、**定員が実質 1 人減る**
    assert!(matches!(名簿.add(鍵(2)), Err(Error::AlreadyIn)));
    assert!(matches!(名簿.add(鍵(1)), Err(Error::AlreadyIn)));
    assert_eq!(名簿.len(), 2);
}

#[test]
fn 名簿に無い相手は会議に入れない() {
    // 満たすこと 5（`issues/005`）。**割符を持たない相手が入れない**
    let 名簿 = Roster::new(鍵(1));

    assert!(!名簿.contains(&鍵(9)));
}

#[test]
fn 出た人は名簿から消える() {
    let mut 名簿 = Roster::new(鍵(1));
    名簿.add(鍵(2)).unwrap();

    assert!(名簿.remove(&鍵(2)), "居た人を消したら true");
    assert!(!名簿.contains(&鍵(2)));
    assert!(!名簿.remove(&鍵(2)), "居ない人を消しても嘘をつかない");
}

#[test]
fn 主催者は自分では抜けない() {
    // 主催者が消えると「誰の会議か」が分からなくなる。
    // **会議を終えるのは退出ではなく、会議そのものを捨てること**
    let mut 名簿 = Roster::new(鍵(1));

    assert!(!名簿.remove(&鍵(1)));
    assert_eq!(名簿.host(), 鍵(1));
}

#[test]
fn 会議_id_は毎回違う() {
    let a = MeetingId::generate();
    let b = MeetingId::generate();

    assert_ne!(a, b, "同じだと別の会議の招待が同じ話に見える");
}

#[test]
fn 会議_id_は文字列にして戻せる() {
    let 元 = MeetingId::generate();
    let 文字列 = 元.to_string();

    assert_eq!(文字列.parse::<MeetingId>().unwrap(), 元);
    assert!(
        文字列
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
        "warifu が外に出す文字列は base32 の 1 種類（M1・M2・M3 と同じ）: {文字列}"
    );
}

#[test]
fn 招待は会議_id_と名簿を運ぶ() {
    // フルメッシュでは、**入る側は他の参加者が誰かを知らないと繋ぎに行けない**
    let 会議 = MeetingId::generate();
    let mut 名簿 = Roster::new(鍵(1));
    名簿.add(鍵(2)).unwrap();
    名簿.add(鍵(3)).unwrap();

    let 塊 = Notice::Invite {
        meeting: 会議,
        roster: 名簿.clone(),
    }
    .to_intent()
    .unwrap();

    assert_eq!(塊.kind().as_str(), "meeting.invite");
    match Notice::from_intent(&塊).unwrap() {
        Notice::Invite { meeting, roster } => {
            assert_eq!(meeting, 会議);
            assert_eq!(roster.members(), 名簿.members());
        }
        その他 => panic!("招待として読めない: {その他:?}"),
    }
}

#[test]
fn 会議_id_はそのまま相関になる() {
    // 同じ会議のやり取りが 1 本の話として辿れる。
    // **別に会議 id を荷物へ書くと、相関と食い違ったときに直しようがなくなる**
    let 会議 = MeetingId::generate();
    let 塊 = Notice::Join { meeting: 会議 }.to_intent().unwrap();

    assert_eq!(塊.correlation(), 会議.into());
    assert!(塊.payload().is_empty(), "入るのに荷物は要らない");
}

#[test]
fn 入ると出るがそのまま往復する() {
    let 会議 = MeetingId::generate();

    for 元 in [
        Notice::Join { meeting: 会議 },
        Notice::Leave { meeting: 会議 },
    ] {
        let 塊 = 元.to_intent().unwrap();
        assert_eq!(Notice::from_intent(&塊).unwrap(), 元);
    }
}

#[test]
fn 定員より多い名簿は受け取らない() {
    // 相手が上限を守る保証は無い。**受け取る側でも数える**
    let 会議 = MeetingId::generate();
    let mut 名簿 = Roster::new(鍵(1));
    名簿.add(鍵(2)).unwrap();
    名簿.add(鍵(3)).unwrap();
    名簿.add(鍵(4)).unwrap();

    let 正しい = Notice::Invite {
        meeting: 会議,
        roster: 名簿,
    }
    .to_intent()
    .unwrap();

    // 定員は 4 のまま、人数だけ 5 に書き換え、鍵を 1 つ足した塊を作る。
    // 塊は `[定員 1][人数 1][鍵 32]*人数`
    let mut 荷物 = 正しい.payload().to_vec();
    荷物[1] = 5;
    荷物.extend_from_slice(&鍵(5).to_bytes());
    let 偽物 =
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物);

    assert!(matches!(Notice::from_intent(&偽物), Err(Error::Full)));
}

#[test]
fn 外枠を超える定員を名乗る招待は受け取らない() {
    // 定員そのものを大きく名乗れるなら、定員を運ばせた意味が無い
    let mut 名簿 = Roster::new(鍵(1));
    名簿.add(鍵(2)).unwrap();

    let 正しい = Notice::Invite {
        meeting: MeetingId::generate(),
        roster: 名簿,
    }
    .to_intent()
    .unwrap();

    let mut 荷物 = 正しい.payload().to_vec();
    #[allow(clippy::cast_possible_truncation)]
    {
        荷物[0] = (HARD_LIMIT + 1) as u8;
    }
    let 偽物 =
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物);

    assert!(matches!(Notice::from_intent(&偽物), Err(Error::Full)));
}

#[test]
fn 会議のものでない口は読まない() {
    // `file.offer` を会議の知らせとして読み違えない
    let 塊 = warifu_intent::Intent::new(
        warifu_intent::Kind::new("file.offer").unwrap(),
        b"xxx".to_vec(),
    );

    assert!(matches!(Notice::from_intent(&塊), Err(Error::NotMeeting)));
}
