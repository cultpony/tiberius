use std::ops::Deref;

use crate::tokenizer::fold::{
    fold_pass1, fold_pass2, fold_pass3, fold_pass4, fold_pass5, FoldStateVec,
};

pub(crate) mod fold;

#[derive(Debug, Clone, PartialEq)]
pub struct Token(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct TokenVec(pub Vec<Token>);

impl From<&str> for Token {
    fn from(v: &str) -> Self {
        Self(v.into())
    }
}

impl Deref for TokenVec {
    type Target = [Token];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Token {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<Token>> for TokenVec {
    fn from(s: Vec<Token>) -> Self {
        Self(s)
    }
}

impl From<Vec<&str>> for TokenVec {
    fn from(s: Vec<&str>) -> Self {
        Self(s.into_iter().map(|x| Token(x.to_string())).collect())
    }
}

impl Token {
    // used for testing
    #[allow(dead_code)]
    fn from_vec(v: Vec<&str>) -> Vec<Token> {
        let v: TokenVec = v.into();
        v.0
    }
}

impl TokenVec {
    /// This parsing step removes duplicate token and moves escapes into their tokens
    /// The stream is now ready for the first parser pass (unambigious syntax parsing)
    pub fn compact(mut self) -> TokenVec {
        let mut out = Vec::new();
        self.0.reverse();
        while !self.0.is_empty() {
            let t = self.0.pop();
            if let Some(mut t) = t {
                if t.0 == "\\" {
                    if let Some(t2) = self.0.pop() {
                        t.0 += &t2.0;
                        out.push(t);
                        continue;
                    } else {
                        out.push(t);
                    }
                } else if t.0 == " " {
                    out.push(t);
                    while let Some(t2) = self.0.pop() {
                        if t2.0 == " " {
                            continue;
                        } else {
                            self.0.push(t2);
                            break;
                        }
                    }
                } else if t.0 == "&" {
                    if let Some(t2) = self.0.pop() {
                        if t2.0 == "&" {
                            self.0.push(Token("&&".to_string()));
                            continue;
                        } else {
                            self.0.push(t);
                            self.0.push(t2);
                            break;
                        }
                    } else {
                        self.0.push(t);
                        continue;
                    }
                } else if t.0 == "|" {
                    if let Some(t2) = self.0.pop() {
                        if t2.0 == "|" {
                            self.0.push(Token("||".to_string()))
                        } else {
                            self.0.push(t);
                            self.0.push(t2);
                        }
                    } else {
                        self.0.push(t);
                    }
                } else {
                    out.push(t);
                    continue;
                }
            }
        }
        out.into()
    }
}

pub struct Tokenizer {
    pub tokens: Vec<Token>,
    remainder: Vec<char>,
    cur: Option<char>,
    next: Option<char>,
    symbol_stack: Vec<char>,
}

impl Tokenizer {
    pub fn new<S: Into<String>>(s: S) -> Self {
        let s: String = s.into();
        let mut s: Vec<char> = s.chars().rev().collect();
        let cur = s.pop();
        let next = s.pop();
        let start_stack = vec![];
        Self {
            tokens: vec![],
            cur,
            next,
            remainder: s,
            symbol_stack: start_stack,
        }
    }
    fn flush_symbolstack(&mut self) {
        if self.remainder.is_empty() {
            self.tokens.push(Token(self.symbol_stack.iter().collect()));
        } else {
            let border = self.cur.take();
            self.tokens.push(Token(self.symbol_stack.iter().collect()));
            self.tokens
                .push(Token(border.unwrap_or_default().to_string()));
        }
        self.symbol_stack.drain(..);
    }
    pub fn tokenize(mut self) -> Vec<Token> {
        loop {
            if self.is_forced_token_border() {
                self.flush_symbolstack();
            }
            if self.done() {
                break;
            }
            self.advance();
        }
        self.advance_rest();
        self.flush_symbolstack();
        self.tokens
            .into_iter()
            .filter(|x| x.0 != "\0" && !x.0.is_empty())
            .collect()
    }
    fn done(&self) -> bool {
        self.remainder.is_empty()
    }
    fn advance_rest(&mut self) {
        if let Some(cur) = self.cur {
            self.symbol_stack.push(cur);
        }
        if let Some(next) = self.next {
            if Self::is_forced_token_border_char(Some(next)) {
                self.flush_symbolstack();
            }
            self.symbol_stack.push(next);
        }
        self.symbol_stack.append(&mut self.remainder);
    }
    fn advance(&mut self) {
        if self.next.is_some() {
            if let Some(cur) = self.cur {
                self.symbol_stack.push(cur);
            }
            self.cur = self.next;
        }
        self.next = self.remainder.pop();
    }
    fn is_forced_token_border_char(cur: Option<char>) -> bool {
        cur.map(|cur| " -~^*,():|&\\.".contains(cur))
            .unwrap_or(false)
    }
    fn is_forced_token_border(&mut self) -> bool {
        if self.cur.is_none() && self.next.is_none() {
            true
        } else {
            Self::is_forced_token_border_char(self.cur)
        }
    }
}

pub(crate) fn parse<S: Into<String>>(s: S) -> FoldStateVec {
    let s: String = s.into();
    let t: TokenVec = Tokenizer::new(s).tokenize().into();
    let t = t.compact().0.into();
    let t = fold_pass1(t);
    let t = fold_pass2(t);
    let t = fold_pass3(t);
    let t = fold_pass4(t);
    fold_pass5(t)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tokens() {
        let tests: Vec<(&str, Vec<Token>)> = vec![
            ("sg", Token::from_vec(vec!["sg"])),
            ("sglong", Token::from_vec(vec!["sglong"])),
            (
                "species:eqg human",
                Token::from_vec(vec!["species", ":", "eqg", " ", "human"]),
            ),
            (
                "artist:bibi_8_8_ and eqg human",
                Token::from_vec(vec![
                    "artist",
                    ":",
                    "bibi_8_8_",
                    " ",
                    "and",
                    " ",
                    "eqg",
                    " ",
                    "human",
                ]),
            ),
            ("sg,cute", Token::from_vec(vec!["sg", ",", "cute"])),
            (
                "-(species:pony || species:eqg human&&pony)",
                Token::from_vec(vec![
                    "-", "(", "species", ":", "pony", " ", "|", "|", " ", "species", ":", "eqg",
                    " ", "human", "&", "&", "pony", ")",
                ]),
            ),
            (
                "time\\,space",
                Token::from_vec(vec!["time", "\\", ",", "space"]),
            ),
            (
                "time     space",
                Token::from_vec(vec!["time", " ", " ", " ", " ", " ", "space"]),
            ),
            (
                "created_at.gte:3 days ago",
                Token::from_vec(vec![
                    "created_at",
                    ".",
                    "gte",
                    ":",
                    "3",
                    " ",
                    "days",
                    " ",
                    "ago",
                ]),
            ),
            (
                "created_at:2015-04 01:00:50Z",
                Token::from_vec(vec![
                    "created_at",
                    ":",
                    "2015",
                    "-",
                    "04",
                    " ",
                    "01",
                    ":",
                    "00",
                    ":",
                    "50Z",
                ]),
            ),
        ];
        for test in tests.into_iter() {
            let input = test.0;
            let expected = test.1;
            let t = Tokenizer::new(input).tokenize();
            assert_eq!(
                expected, t,
                "The text {:?} did not parse into {:?}, got {:?} instead",
                input, expected, t
            );
        }
    }

    #[test]
    fn test_tokens_pass1() {
        let tests: Vec<(&str, Vec<Token>)> = vec![
            ("sg", Token::from_vec(vec!["sg"])),
            ("sglong", Token::from_vec(vec!["sglong"])),
            (
                "species:eqg human",
                Token::from_vec(vec!["species", ":", "eqg", " ", "human"]),
            ),
            (
                "artist:bibi_8_8_ and eqg human",
                Token::from_vec(vec![
                    "artist",
                    ":",
                    "bibi_8_8_",
                    " ",
                    "and",
                    " ",
                    "eqg",
                    " ",
                    "human",
                ]),
            ),
            ("sg,cute", Token::from_vec(vec!["sg", ",", "cute"])),
            (
                "-(species:pony || species:eqg human&&pony)",
                Token::from_vec(vec![
                    "-", "(", "species", ":", "pony", " ", "||", " ", "species", ":", "eqg", " ",
                    "human", "&&", "pony", ")",
                ]),
            ),
            (
                "time\\,space",
                Token::from_vec(vec!["time", "\\,", "space"]),
            ),
            (
                "time     space",
                Token::from_vec(vec!["time", " ", "space"]),
            ),
            (
                "created_at.gte:3 days ago",
                Token::from_vec(vec![
                    "created_at",
                    ".",
                    "gte",
                    ":",
                    "3",
                    " ",
                    "days",
                    " ",
                    "ago",
                ]),
            ),
            (
                "created_at:2015-04 01:00:50Z",
                Token::from_vec(vec![
                    "created_at",
                    ":",
                    "2015",
                    "-",
                    "04",
                    " ",
                    "01",
                    ":",
                    "00",
                    ":",
                    "50Z",
                ]),
            ),
            (
                "pony OR human",
                Token::from_vec(vec!["pony", " ", "OR", " ", "human"]),
            ),
        ];
        for test in tests.into_iter() {
            let input = test.0;
            let expected: TokenVec = test.1.into();
            let t = Tokenizer::new(input).tokenize();
            let t: TokenVec = t.into();
            let t = t.compact();
            assert_eq!(
                expected, t,
                "The text {:?} did not parse into {:?}, got {:?} instead",
                input, expected, t
            );
        }
    }
}
