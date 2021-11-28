use std::ops::Deref;

use crate::tokenizer::Token;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FoldState {
    Raw(Token),
    LogicalAnd,
    LogicalOr,
    LogicalNot,
    GroupStart,
    GroupEnd,
    Group(FoldStateVec),
    /// Empty token that can be removed from the stream if found
    None,
}

impl FoldState {
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Self::LogicalAnd
                | Self::LogicalNot
                | Self::LogicalOr
                | Self::GroupStart
                | Self::GroupEnd
        )
    }
}

impl PartialEq<str> for FoldState {
    fn eq(&self, other: &str) -> bool {
        match self {
            Self::Raw(v) => v.0 == other,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FoldStateVec(pub Vec<FoldState>);

impl From<Vec<Token>> for FoldStateVec {
    fn from(v: Vec<Token>) -> Self {
        FoldStateVec(v.into_iter().map(FoldState::Raw).collect())
    }
}

/// Fold up unambigious syntax
pub(crate) fn fold_pass1(m: FoldStateVec) -> FoldStateVec {
    let mut out = Vec::new();
    for token in m.0 {
        match token {
            FoldState::Raw(token) => {
                if token.deref() == "&&" || token.deref() == "," {
                    out.push(FoldState::LogicalAnd);
                } else if token.deref() == "||" {
                    out.push(FoldState::LogicalOr);
                } else if token.deref() == "-" || token.deref() == "!" {
                    out.push(FoldState::LogicalNot);
                } else if token.deref() == "(" {
                    out.push(FoldState::GroupStart);
                } else if token.deref() == ")" {
                    out.push(FoldState::GroupEnd);
                } else {
                    out.push(FoldState::Raw(token))
                }
            }
            v => out.push(v),
        }
    }
    FoldStateVec(out)
}

/// Fold up named "and" and "or" parts of the query
pub(crate) fn fold_pass2(m: FoldStateVec) -> FoldStateVec {
    let mut out = Vec::new();
    let mut prev = &FoldState::None;
    let mut skip_one = false;
    for idx in 0..m.0.len() {
        let token = &m.0[idx];
        if skip_one {
            skip_one = false;
            continue;
        }
        match token {
            FoldState::Raw(token) => {
                if token.deref() == "AND" {
                    if prev.deref() == " " {
                        if let FoldState::Raw(q) = &m.0[(idx + 1).min(m.0.len())] {
                            if q.deref() == " " {
                                out.pop();
                                out.push(FoldState::LogicalAnd);
                                skip_one = true;
                            }
                        }
                    }
                } else if token.deref() == "OR" {
                    if prev.deref() == " " {
                        if let FoldState::Raw(q) = &m.0[(idx + 1).min(m.0.len())] {
                            if q.deref() == " " {
                                out.pop();
                                out.push(FoldState::LogicalOr);
                                skip_one = true;
                            }
                        }
                    }
                } else {
                    out.push(FoldState::Raw(token.clone()))
                }
            }
            FoldState::LogicalAnd => {
                if let Some(v) = out.pop() {
                    match v {
                        FoldState::Raw(v) => {
                            if v.deref() != " " {
                                out.push(FoldState::Raw(v));
                            }
                        }
                        v => out.push(v),
                    }
                }
                out.push(FoldState::LogicalAnd);
            }
            FoldState::LogicalOr => {
                if let Some(v) = out.pop() {
                    match v {
                        FoldState::Raw(v) => {
                            if v.deref() != " " {
                                out.push(FoldState::Raw(v));
                            }
                        }
                        v => out.push(v),
                    }
                }
                out.push(FoldState::LogicalOr);
                if let FoldState::Raw(q) = &m.0[(idx + 1).min(m.0.len())] {
                    if q.deref() == " " {
                        skip_one = true;
                    }
                }
            }
            FoldState::LogicalNot => {
                if let Some(v) = out.pop() {
                    match v {
                        FoldState::Raw(v) => {
                            if v.deref() != " " {
                                out.push(FoldState::Raw((&*(v.deref().to_string() + "-")).into()));
                                prev = token;
                                continue;
                            }
                        }
                        v => out.push(v),
                    }
                }
                out.push(FoldState::LogicalNot);
            }
            v => out.push(v.clone()),
        }
        prev = token;
    }
    FoldStateVec(out)
}

pub(crate) fn fold_pass3(mut m: FoldStateVec) -> FoldStateVec {
    let mut out = Vec::new();
    let mut prev = FoldState::None;
    let mut skip_size = 0;
    let opening_groups =
        m.0.iter()
            .filter(|x| (*x).clone() == FoldState::GroupStart)
            .count();
    let closing_groups =
        m.0.iter()
            .filter(|x| (*x).clone() == FoldState::GroupEnd)
            .count();
    let mut groups: usize = closing_groups.min(opening_groups);
    if groups == 0 {
        return m;
    }
    while groups > 0 {
        for idx in 0..m.0.len() {
            let token = m.0[idx].clone();
            if skip_size > 0 {
                if idx + skip_size >= m.0.len() {
                    break;
                }
                skip_size -= 1;
                continue;
            } else if idx > m.0.len() {
                // happens when inplace has moved a lot of things
                break;
            }
            match token.clone() {
                FoldState::GroupStart => {
                    if !prev.is_operator() && groups > 0 {
                        groups -= 1;
                        out.push(FoldState::Raw("(".into()))
                    } else {
                        let mut b = Vec::new();
                        assert!(idx < m.0.len());
                        for qidx in (idx + 1..m.0.len()).rev() {
                            let qtoken = &m.0[qidx];
                            match qtoken {
                                FoldState::GroupEnd => {
                                    let group: Vec<FoldState> =
                                        m.0.splice(idx + 1..qidx, vec![]).collect();
                                    let group = fold_pass3(FoldStateVec(group));
                                    b.extend_from_slice(&group.0);
                                    skip_size = idx + (m.0.len().saturating_sub(qidx));
                                    groups -= 1;
                                    break;
                                }
                                v => b.push(v.clone()),
                            }
                        }
                        if b.is_empty() {
                            groups = groups.saturating_sub(1);
                        } else {
                            out.push(FoldState::Group(FoldStateVec(b)));
                        }
                        break;
                    }
                }
                v => out.push(v.clone()),
            }
            prev = token.clone();
        }
    }
    FoldStateVec(out)
}

pub(crate) fn fold_pass4(m: FoldStateVec) -> FoldStateVec {
    let mut out: Vec<FoldState> = Vec::new();
    for p in m.0 {
        match p {
            FoldState::Group(v) => out.push(FoldState::Group(fold_pass4(v))),
            FoldState::GroupStart => out.push(FoldState::Raw("(".into())),
            FoldState::GroupEnd => out.push(FoldState::Raw(")".into())),
            v => out.push(v),
        }
    }
    FoldStateVec(out)
}

pub(crate) fn fold_pass5(m: FoldStateVec) -> FoldStateVec {
    let mut out: Vec<FoldState> = Vec::new();
    let mut sbuf = String::new();
    for p in m.0 {
        match p {
            FoldState::Raw(t) => sbuf.push_str(&t.0),
            FoldState::Group(v) => out.push(FoldState::Group(fold_pass5(v))),
            v => {
                if sbuf != String::new() {
                    out.push(FoldState::Raw(sbuf.as_str().into()));
                    sbuf = String::new();
                }
                out.push(v)
            }
        }
    }
    if sbuf != String::new() {
        out.push(FoldState::Raw(sbuf.as_str().into()));
    }
    FoldStateVec(out)
}

#[cfg(test)]
mod test {
    use super::super::*;
    use super::*;

    #[test]
    fn test_tokens_pass1() {
        let tests: Vec<(&str, FoldStateVec)> = vec![
            ("sg", Token::from_vec(vec!["sg"]).deref().to_vec().into()),
            (
                "sglong",
                Token::from_vec(vec!["sglong"]).deref().to_vec().into(),
            ),
            (
                "species:eqg human",
                FoldStateVec(vec![FoldState::Raw("species:eqg human".into())]),
            ),
            (
                "artist:bibi_8_8_ AND eqg human",
                FoldStateVec(vec![
                    FoldState::Raw("artist:bibi_8_8_".into()),
                    FoldState::LogicalAnd,
                    FoldState::Raw("eqg human".into()),
                ]),
            ),
            (
                "sg,cute",
                FoldStateVec(vec![
                    FoldState::Raw("sg".into()),
                    FoldState::LogicalAnd,
                    FoldState::Raw("cute".into()),
                ]),
            ),
            (
                "-(species:pony || species:eqg human&&pony)",
                FoldStateVec(vec![
                    FoldState::LogicalNot,
                    FoldState::Group(FoldStateVec(vec![
                        FoldState::Raw("species:pony".into()),
                        FoldState::LogicalOr,
                        FoldState::Raw("species:eqg human".into()),
                        FoldState::LogicalAnd,
                        FoldState::Raw("pony".into()),
                    ])),
                ]),
            ),
            (
                "time\\,space",
                Token::from_vec(vec!["time\\,space"])
                    .deref()
                    .to_vec()
                    .into(),
            ),
            (
                "time     space",
                Token::from_vec(vec!["time space"]).deref().to_vec().into(),
            ),
            (
                "created_at.gte:3 days ago",
                Token::from_vec(vec!["created_at.gte:3 days ago"])
                    .deref()
                    .to_vec()
                    .into(),
            ),
            (
                "created_at:2015-04 01:00:50Z",
                Token::from_vec(vec!["created_at:2015-04 01:00:50Z"])
                    .deref()
                    .to_vec()
                    .into(),
            ),
            (
                "pony OR human",
                FoldStateVec(vec![
                    FoldState::Raw("pony".into()),
                    FoldState::LogicalOr,
                    FoldState::Raw("human".into()),
                ])
            ),
            (
                "sg AND (-pony-,(:),human (eqg)))",
                FoldStateVec(vec![
                    FoldState::Raw("sg".into()),
                    FoldState::LogicalAnd,
                    FoldState::Group(FoldStateVec(vec![
                        FoldState::LogicalNot,
                        FoldState::Raw("pony-".into()),
                        FoldState::LogicalAnd,
                        FoldState::Group(FoldStateVec(vec![
                            FoldState::Raw(":)".into()),
                            FoldState::LogicalAnd,
                            FoldState::Raw("human (eqg)".into()),
                        ])),
                    ])),
                ]),
            ),
            (
                "pride flag, -oc, -twilight sparkle, -fluttershy, -pinkie pie,-rainbow dash, -applejack, -rarity",
                FoldStateVec(vec![
                    FoldState::Raw("pride flag".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("oc".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("twilight sparkle".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("fluttershy".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("pinkie pie".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("rainbow dash".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("applejack".into()),
                    FoldState::LogicalAnd,
                    FoldState::LogicalNot,
                    FoldState::Raw("rarity".into()),

                ]),
            ),
        ];
        for test in tests.into_iter() {
            let input = test.0;
            let expected: FoldStateVec = test.1;
            let t = Tokenizer::new(input).tokenize();
            let t: TokenVec = t.into();
            let t = t.compact().0.into();
            let t = fold_pass1(t);
            let t = fold_pass2(t);
            let t = fold_pass3(t);
            let t = fold_pass4(t);
            let t = fold_pass5(t);
            assert_eq!(
                expected, t,
                "The text {:?} did not parse into {:?}, got {:?} instead",
                input, expected, t
            );
        }
    }
}
