//! 1 つの話。**過去スレッド全文を毎回渡さない。**
//!
//! 同じ引用・同じ署名・同じ免責文を毎回モデルに読ませているのが token の無駄で、
//! **落とせる部分は、解釈器を呼ぶ前に落とす。**
//!
//! # hash を使っていない（`decisions.md` **D21**）
//!
//! `issues/007` は「hash で除外する」と書いているが、原文どうしを比べている。
//! hash にすると、**衝突を作れる相手が、読み手から任意の 1 ブロックを消せる。**
//! 消えたことは読み手に見えないので、気づきようがない。
//!
//! 突き合わせる数は 1 つの話に出てくる塊ぶんしかない。
//! **速さの損より、消されない得を取る。**

use crate::Received;

/// 本文の塊 1 つ。空行で区切る。
///
/// 比べるときだけ空白を潰す。**返すのは原文のほう**で、
/// 潰した側は外に出さない（読み手が受け取るのは相手が書いた文字列でなければならない）。
#[derive(Debug, Clone)]
struct Block {
    原文: String,
    比較用: String,
}

impl Block {
    /// 本文を塊に割る。**引用行はここで落ちる。**
    fn 割る(text: &str) -> Vec<Self> {
        text.split("\n\n")
            .map(引用を落とす)
            .filter(|b| !b.trim().is_empty())
            .map(|b| Block {
                比較用: 比較用にする(&b),
                原文: b.trim().to_owned(),
            })
            .collect()
    }
}

/// 行頭の `>` で始まる行を落とす。
///
/// 相手が新しく書いた文ではないので、渡す意味が無い。
fn 引用を落とす(塊: &str) -> String {
    塊.lines()
        .filter(|l| !l.trim_start().starts_with('>'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 比べるためだけの形。
///
/// 行ごとに前後の空白を落として繋ぐ。
/// 引用で字下げが 1 つ増えただけの塊を「新しい」と数えないため。
fn 比較用にする(塊: &str) -> String {
    塊.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 1 つの話。
///
/// **話ごとに 1 つ持つ。**跨いで使い回すと、
/// 別の相手が同じ文を書いたときに片方が消えて読み違える。
#[derive(Debug, Clone, Default)]
pub struct Thread {
    読んだ: Vec<String>,
    定型: Vec<String>,
    落とした: usize,
}

impl Thread {
    /// 空の話。
    pub fn new() -> Self {
        Self::default()
    }

    /// 定型（署名・免責文）を登録した話。
    ///
    /// **人が登録する。**本文から自動で覚えない。
    /// 覚えると、同じ文を 2 回送るだけで**読み手から任意の 1 ブロックを消せる**（D21）。
    pub fn new_with_boilerplate(boilerplate: Vec<String>) -> Self {
        Self {
            定型: boilerplate.iter().map(|b| 比較用にする(b)).collect(),
            ..Self::default()
        }
    }

    /// 登録されている定型。
    pub fn boilerplate(&self) -> &[String] {
        &self.定型
    }

    /// これまでに落とした塊の数。
    ///
    /// 効いているかどうかは、この数でしか測れない（`decisions.md` **D20** と同じ理屈）。
    pub fn dropped(&self) -> usize {
        self.落とした
    }

    /// 1 通を足し、**まだ読んでいない部分だけ**を返す。
    ///
    /// 落ちるのは 3 つ — 引用行 / 既に読んだ塊 / 登録された定型。
    /// **順序は変えない。**並べ替えると、話の流れが読めなくなる。
    pub fn add(&mut self, received: &Received) -> String {
        let 本文 = String::from_utf8_lossy(received.body().as_bytes()).into_owned();
        let mut 残す: Vec<String> = Vec::new();

        for b in Block::割る(&本文) {
            if self.定型.contains(&b.比較用) || self.読んだ.contains(&b.比較用) {
                self.落とした += 1;
                continue;
            }
            self.読んだ.push(b.比較用);
            残す.push(b.原文);
        }
        残す.join("\n\n")
    }
}
