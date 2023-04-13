use std::{
    fmt::Display,
    ops::{Bound, Range},
    str::FromStr,
};

#[derive(PartialEq, Clone, Debug)]
pub enum Query {
    And {
        l: Box<Query>,
        r: Box<Query>,
    },
    Or {
        l: Box<Query>,
        r: Box<Query>,
    },
    Not {
        v: Box<Query>,
    },
    Group {
        v: Vec<Query>,
    },
    Tag {
        n: Option<String>,
        v: String,
    },
    Attribute {
        v: String,
        cmp: Comparator,
        t: AttrValue,
    },
    True,
}

#[derive(PartialEq, Clone, Debug)]
pub enum AttrValue {
    Integer(i64),
    Float(f64),
    String(String),
}

impl Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::And { l, r } => f.write_fmt(format_args!("{{{} AND {}}}", l, r))?,
            Self::Or { l, r } => f.write_fmt(format_args!("{{{} OR {}}}", l, r))?,
            Self::Not { v } => f.write_fmt(format_args!("{{NOT {}}}", v))?,
            Self::Group { v } => {
                f.write_str(" {")?;
                for t in v {
                    f.write_fmt(format_args!("{}", t))?;
                }
                f.write_str("} ")?;
            }
            Self::Tag { n, v } => {
                match n {
                    None => f.write_fmt(format_args!("{:?}", v))?,
                    Some(n) => f.write_fmt(format_args!("{:?}:{:?}", n, v))?,
                };
            }
            Self::Attribute { v, cmp, t } => {
                f.write_fmt(format_args!("?{:?} {:?} {:?}?", v, cmp, t))?;
            }
            Self::True => {
                f.write_str(" TRUE ")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Match<T, U, V>
where
    T: PartialEq,
    U: PartialOrd,
    V: Into<String>,
{
    Exact(T),
    Range(Range<U>),
    Distance(V, f64),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Comparator {
    GreaterEqual,
    Greater,
    Equal,
    Less,
    LessEqual,
    NotEqual,
    Invalid,
}

impl<T, U, V> Match<T, U, V>
where
    T: PartialEq,
    U: PartialOrd,
    V: Into<String> + Clone,
{
    pub fn matches_exact(&self, i: T) -> bool {
        match self {
            Self::Exact(r) => r.eq(&i),
            Self::Range(_) => false,
            Self::Distance(..) => false,
        }
    }
    pub fn matches_range(&self, i: U) -> bool {
        match self {
            Self::Exact(_) => false,
            Self::Range(r) => r.contains(&i),
            Self::Distance(..) => false,
        }
    }
    pub fn matches_distance(&self, i: V) -> bool {
        match self {
            Self::Exact(_) => false,
            Self::Range(_) => false,
            Self::Distance(v, d) => {
                let v: String = v.clone().into();
                let i: String = i.into();
                strsim::normalized_levenshtein(&v, &i) <= *d
            }
        }
    }
    pub fn matches_any(&self, i1: T, i2: U, i3: V) -> bool {
        self.matches_exact(i1) || self.matches_range(i2) || self.matches_distance(i3)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("An operator is missing one or two operands: {0}")]
    OperatorError(String),
    #[error("Could not parse date: {0}")]
    DateTimeError(#[from] Box<htp::HTPError>),
    #[error("Could not parse date: {0}")]
    ChronoEnglish(#[from] chrono_english::DateError),
    #[error("We're working hard on making the query syntax accept more things")]
    UnsupportedQuerySyntaxTodo,
    #[cfg(feature = "search-with-tantivy")]
    #[error("Error in index: {0}")]
    TantivyError(#[from] tantivy::TantivyError),
    #[error("Auxiliary Query Error: {0}")]
    AuxQueryError(String),
}

impl From<htp::HTPError> for QueryError {
    fn from(value: htp::HTPError) -> Self {
        Self::DateTimeError(Box::new(value))
    }
}

use either::Either;
use lazy_regex::{lazy_regex, Lazy, Regex};
#[cfg(feature = "search-with-tantivy")]
use tantivy::schema::FieldType;
#[cfg(feature = "search-with-tantivy")]
use tantivy::schema::IndexRecordOption;
#[cfg(feature = "search-with-tantivy")]
use tantivy::Term;
use tracing::debug;

use crate::tokenizer::{
    fold::{FoldState, FoldStateVec},
    parse,
};

static ATTRCOMP_REX: Lazy<Regex> = lazy_regex!(r#"^([\w_]+)\.(\w{0,4}):(.+)$"#);

impl FromStr for Query {
    type Err = (Query, QueryError);

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fs = parse(s);
        Query::from_foldstate(0, fs)
    }
}

pub(crate) type Intermediate = Either<Query, FoldState>;
pub(crate) type IntermediateResult = Result<Vec<Intermediate>, IntermediateError>;
pub(crate) type IntermediateError = (Vec<Intermediate>, QueryErrorType);
pub(crate) type QueryErrorType = (Query, QueryError);
pub(crate) type QueryIntermediateResult = Result<Query, IntermediateError>;

impl Query {
    pub(crate) fn not_conv(depth: usize, f: Vec<Intermediate>) -> IntermediateResult {
        assert!(depth < 128, "depth limit exceeded");
        let mut out: Vec<Intermediate> = Vec::new();
        let mut skip: usize = 0;
        use either::*;
        for i in 0..f.len() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let q = f[i].clone();
            match q {
                Left(v) => out.push(Either::Left(v)),
                Right(v) => match v {
                    FoldState::LogicalNot => {
                        skip = 1;
                        let op = Self::conv(depth + 1, Self::pack(f[i + 1].clone()));
                        match op {
                            Err((mut q, e)) => {
                                out.append(&mut q);
                                return Err((out, e));
                            }
                            Ok(v) => {
                                let v = Box::new(v);
                                out.push(Either::Left(Query::Not { v }));
                                continue;
                            }
                        }
                    }
                    v => out.push(Either::Right(v)),
                },
            }
        }
        Ok(out)
    }
    pub(crate) fn and_conv(depth: usize, f: Vec<Intermediate>) -> IntermediateResult {
        assert!(depth < 128, "depth limit exceeded");
        let mut out: Vec<Intermediate> = Vec::new();
        let mut skip: usize = 0;
        use either::*;
        for i in 0..f.len() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let q = f[i].clone();
            match q {
                Left(v) => out.push(Either::Left(v)),
                Right(v) => {
                    match v {
                        FoldState::LogicalAnd => {
                            skip = 1;
                            let next = Self::conv(depth + 1, Self::pack(f[i + 1].clone()));
                            // this must be a Query Type
                            let prev = out.pop().unwrap().left().unwrap();
                            match next {
                                Err((mut q, e)) => {
                                    out.append(&mut q);
                                    return Err((out, e));
                                }
                                Ok(next) => {
                                    out.push(Either::Left(Query::And {
                                        l: Box::new(prev),
                                        r: Box::new(next),
                                    }));
                                    continue;
                                }
                            }
                        }
                        v => out.push(Either::Right(v)),
                    }
                }
            }
        }
        Ok(out)
    }
    pub(crate) fn or_conv(depth: usize, f: Vec<Intermediate>) -> IntermediateResult {
        assert!(depth < 128, "depth limit exceeded");
        let mut out: Vec<Intermediate> = Vec::new();
        let mut skip: usize = 0;
        use either::*;
        for i in 0..f.len() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let q = f[i].clone();
            match q {
                Left(v) => out.push(Either::Left(v)),
                Right(v) => {
                    match v {
                        FoldState::LogicalOr => {
                            skip = 1;
                            let next = Self::conv(depth + 1, Self::pack(f[i + 1].clone()));
                            // this must be a Query Type
                            let prev = out.pop().unwrap().left().unwrap();
                            match next {
                                Err((mut q, e)) => {
                                    out.append(&mut q);
                                    return Err((out, e));
                                }
                                Ok(next) => {
                                    out.push(Either::Left(Query::Or {
                                        l: Box::new(prev),
                                        r: Box::new(next),
                                    }));
                                    continue;
                                }
                            }
                        }
                        v => out.push(Either::Right(v)),
                    }
                }
            }
        }
        Ok(out)
    }
    pub(crate) fn token_conv(depth: usize, f: Vec<Intermediate>) -> IntermediateResult {
        assert!(depth < 128);
        let mut out = Vec::new();
        for x in f {
            match x {
                Either::Left(v) => out.push(Either::Left(v)),
                Either::Right(v) => match v {
                    FoldState::Raw(v) => out.push(Either::Left(Query::Tag { n: None, v: v.0 })),
                    FoldState::Group(v) => out.push(Either::Left(
                        Self::conv(depth + 1, Self::into_im(v)).unwrap(),
                    )),
                    v => out.push(Either::Right(v)),
                },
            }
        }
        Ok(out)
    }
    pub(crate) fn attrcmp_conv(depth: usize, f: Vec<Intermediate>) -> IntermediateResult {
        assert!(depth < 128);
        let mut out = Vec::new();
        for x in f {
            match x {
                Either::Left(v) => out.push(Either::Left(v)),
                Either::Right(v) => match v {
                    FoldState::Raw(v) => {
                        if let Some(matches) = ATTRCOMP_REX.captures(&v.0) {
                            let v = matches.get(1).unwrap().as_str().to_string();
                            let cmp = matches.get(2).unwrap().as_str();
                            let cmp = match &*cmp.to_lowercase() {
                                "gte" => Comparator::GreaterEqual,
                                "gt" => Comparator::Greater,
                                "eq" => Comparator::Equal,
                                "neq" => Comparator::NotEqual,
                                "lt" => Comparator::Less,
                                "lte" => Comparator::LessEqual,
                                _ => Comparator::Invalid,
                            };
                            let t = matches.get(3).unwrap().as_str();
                            let t = match t.parse() {
                                Ok(v) => AttrValue::Integer(v),
                                Err(_) => match t.parse() {
                                    Ok(v) => AttrValue::Float(v),
                                    Err(_) => AttrValue::String(t.to_string()),
                                },
                            };
                            out.push(Either::Left(Query::Attribute { v, cmp, t }))
                        } else {
                            out.push(Either::Right(FoldState::Raw(v)))
                        }
                    }
                    FoldState::Group(v) => out.push(Either::Left(
                        Self::conv(depth + 1, Self::into_im(v)).unwrap(),
                    )),
                    v => out.push(Either::Right(v)),
                },
            }
        }
        Ok(out)
    }
    pub(crate) fn pack(i: Intermediate) -> Vec<Intermediate> {
        match i {
            Either::Left(v) => vec![Either::Left(v)],
            Either::Right(v) => vec![Either::Right(v)],
        }
    }
    pub(crate) fn conv(depth: usize, f: Vec<Intermediate>) -> QueryIntermediateResult {
        assert!(depth < 128);
        let f = Self::attrcmp_conv(depth + 1, f)?;
        let f = Self::token_conv(depth + 1, f)?;
        let f = Self::not_conv(depth + 1, f)?;
        let f = Self::and_conv(depth + 1, f)?;
        let f = Self::or_conv(depth + 1, f)?;
        let f = Self::im_into_q(f);
        Ok(f)
    }
    pub(crate) fn into_im(f: FoldStateVec) -> Vec<Intermediate> {
        f.0.into_iter().map(either::Right).collect()
    }
    pub(crate) fn im_into_q(f: Vec<Intermediate>) -> Query {
        let mut q: Vec<Query> = f
            .into_iter()
            .map(|x| match x {
                Either::Left(v) => v,
                Either::Right(v) => {
                    panic!("IM not fully Q: {:?}", v);
                }
            })
            .collect();
        if q.len() == 1 {
            q.pop().unwrap()
        } else if q.is_empty() {
            Query::True
        } else {
            Query::Group { v: q }
        }
    }
    pub(crate) fn from_foldstate(
        depth: usize,
        f: FoldStateVec,
    ) -> Result<Query, (Query, QueryError)> {
        let im = Self::into_im(f);
        let conv = Self::conv(depth + 1, im);
        let conv = conv.unwrap();
        Ok(conv)
    }

    #[cfg(feature = "search-with-tantivy")]
    pub fn into_tantivy_search(
        self,
        schema: &tantivy::schema::Schema,
    ) -> Result<Box<dyn tantivy::query::Query>, QueryError> {
        use tantivy::query::{AllQuery, BooleanQuery, EmptyQuery, Occur, RangeQuery, TermQuery};
        debug!("Converting {} to tantivy query type", self);
        Ok(match self {
            Query::Not { v } => Box::new(BooleanQuery::new(vec![(
                Occur::MustNot,
                v.into_tantivy_search(schema)?,
            )])),
            Query::And { l, r } => Box::new(BooleanQuery::new(vec![
                (Occur::Must, l.into_tantivy_search(schema)?),
                (Occur::Must, r.into_tantivy_search(schema)?),
            ])),
            Query::Or { l, r } => Box::new(BooleanQuery::new(vec![
                (Occur::Should, l.into_tantivy_search(schema)?),
                (Occur::Should, r.into_tantivy_search(schema)?),
            ])),
            Query::Group { v } => Box::new(BooleanQuery::new(
                v.into_iter()
                    .flat_map(|q| q.into_tantivy_search(schema))
                    .map(|q| (Occur::Must, q))
                    .collect(),
            )),
            Query::Tag { n, v } => {
                assert!(n.is_none(), "Namespaced tags not supported yet");
                Box::new(TermQuery::new(
                    tantivy::Term::from_field_text(
                        schema.get_field("tag").expect("non-existent tag field"),
                        &v,
                    ),
                    tantivy::schema::IndexRecordOption::Basic,
                ))
            }
            Query::Attribute { v, cmp, t } => {
                let field = schema.get_field(&v);
                let field = match field {
                    Some(f) => f,
                    None => return Ok(Box::new(EmptyQuery)),
                };
                let field_entry = schema.get_field_entry(field);
                let field_type = field_entry.field_type();
                match (field_type, t) {
                    (FieldType::I64(_), AttrValue::Integer(intval)) => match cmp {
                        Comparator::Equal => Box::new(TermQuery::new(
                            tantivy::Term::from_field_i64(field, intval),
                            IndexRecordOption::Basic,
                        )),
                        Comparator::NotEqual => Box::new(BooleanQuery::new(vec![(
                            Occur::MustNot,
                            Box::new(TermQuery::new(
                                tantivy::Term::from_field_i64(field, intval),
                                IndexRecordOption::Basic,
                            )),
                        )])),
                        Comparator::Greater => Box::new(RangeQuery::new_i64_bounds(
                            field,
                            Bound::Unbounded,
                            Bound::Excluded(intval),
                        )),
                        Comparator::GreaterEqual => Box::new(RangeQuery::new_i64_bounds(
                            field,
                            Bound::Unbounded,
                            Bound::Included(intval),
                        )),
                        Comparator::Less => Box::new(RangeQuery::new_i64_bounds(
                            field,
                            Bound::Excluded(intval),
                            Bound::Unbounded,
                        )),
                        Comparator::LessEqual => Box::new(RangeQuery::new_i64_bounds(
                            field,
                            Bound::Included(intval),
                            Bound::Unbounded,
                        )),
                        v => todo!("unimplemented operator against integer: {:?}", v),
                    },
                    (FieldType::F64(_), AttrValue::Float(fltval)) => match cmp {
                        Comparator::Equal => Box::new(TermQuery::new(
                            tantivy::Term::from_field_f64(field, fltval),
                            IndexRecordOption::Basic,
                        )),
                        Comparator::NotEqual => Box::new(BooleanQuery::new(vec![(
                            Occur::MustNot,
                            Box::new(TermQuery::new(
                                tantivy::Term::from_field_f64(field, fltval),
                                IndexRecordOption::Basic,
                            )),
                        )])),
                        Comparator::Greater => Box::new(RangeQuery::new_f64_bounds(
                            field,
                            Bound::Unbounded,
                            Bound::Excluded(fltval),
                        )),
                        Comparator::GreaterEqual => Box::new(RangeQuery::new_f64_bounds(
                            field,
                            Bound::Unbounded,
                            Bound::Included(fltval),
                        )),
                        Comparator::Less => Box::new(RangeQuery::new_f64_bounds(
                            field,
                            Bound::Excluded(fltval),
                            Bound::Unbounded,
                        )),
                        Comparator::LessEqual => Box::new(RangeQuery::new_f64_bounds(
                            field,
                            Bound::Included(fltval),
                            Bound::Unbounded,
                        )),
                        v => todo!("unimplemented operator against float: {:?}", v),
                    },
                    (FieldType::Str(_), AttrValue::String(strval)) => match cmp {
                        Comparator::Equal => Box::new(TermQuery::new(
                            tantivy::Term::from_field_text(field, &strval),
                            IndexRecordOption::Basic,
                        )),
                        Comparator::NotEqual => Box::new(BooleanQuery::new(vec![(
                            Occur::MustNot,
                            Box::new(TermQuery::new(
                                tantivy::Term::from_field_text(field, &strval),
                                IndexRecordOption::Basic,
                            )),
                        )])),
                        Comparator::Greater => Box::new(RangeQuery::new_str_bounds(
                            field,
                            Bound::Unbounded,
                            Bound::Excluded(&strval),
                        )),
                        Comparator::GreaterEqual => Box::new(RangeQuery::new_str_bounds(
                            field,
                            Bound::Unbounded,
                            Bound::Included(&strval),
                        )),
                        Comparator::Less => Box::new(RangeQuery::new_str_bounds(
                            field,
                            Bound::Excluded(&strval),
                            Bound::Unbounded,
                        )),
                        Comparator::LessEqual => Box::new(RangeQuery::new_str_bounds(
                            field,
                            Bound::Included(&strval),
                            Bound::Unbounded,
                        )),
                        v => todo!("unimplemented operator against string: {:?}", v),
                    },
                    (FieldType::Date(_), AttrValue::String(strval)) => {
                        //let strval = htp::parse(&strval, chrono::Utc::now())?;
                        let strval = chrono_english::parse_date_string(
                            &strval,
                            chrono::Utc::now(),
                            chrono_english::Dialect::Uk,
                        )?;
                        let strval = tantivy::DateTime::from_timestamp_secs(strval.timestamp());
                        let strval_term = Term::from_field_date(field, strval);
                        match cmp {
                            Comparator::Equal => Box::new(TermQuery::new(
                                tantivy::Term::from_field_date(field, strval),
                                IndexRecordOption::Basic,
                            )),
                            Comparator::NotEqual => Box::new(BooleanQuery::new(vec![(
                                Occur::MustNot,
                                Box::new(TermQuery::new(
                                    tantivy::Term::from_field_date(field, strval),
                                    IndexRecordOption::Basic,
                                )),
                            )])),
                            Comparator::Greater => Box::new(RangeQuery::new_term_bounds(
                                field,
                                tantivy::schema::Type::Date,
                                &Bound::Excluded(strval_term),
                                &Bound::Unbounded,
                            )),
                            Comparator::GreaterEqual => Box::new(RangeQuery::new_term_bounds(
                                field,
                                tantivy::schema::Type::Date,
                                &Bound::Included(strval_term),
                                &Bound::Unbounded,
                            )),
                            Comparator::Less => Box::new(RangeQuery::new_term_bounds(
                                field,
                                tantivy::schema::Type::Date,
                                &Bound::Unbounded,
                                &Bound::Excluded(strval_term),
                            )),
                            Comparator::LessEqual => Box::new(RangeQuery::new_term_bounds(
                                field,
                                tantivy::schema::Type::Date,
                                &Bound::Unbounded,
                                &Bound::Included(strval_term),
                            )),
                            v => todo!("unimplemented operator against datetime: {:?}", v),
                        }
                    }
                    (v, w) => {
                        todo!("invalid type/field combination: {:?}, {:?}", v, w);
                    }
                }
            }
            Query::True => Box::new(AllQuery),
        })
    }
}

#[cfg(test)]
mod test {
    use super::Query;
    #[test]
    fn test_death_query() -> anyhow::Result<()> {
        let query = "sg AND (-pony-,(:),human (eqg)))";
        let fs = crate::tokenizer::parse(query);
        let exp = r#"{"sg" AND {{NOT "pony-"} AND {":)" AND "human (eqg)"}}}"#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}", e, q),
        };
        println!("Got: {}", q);
        let q = format!("{}", q);
        assert_eq!(exp, q);
        Ok(())
    }

    #[test]
    fn test_long_query() -> anyhow::Result<()> {
        let query = "pride flag, -oc, -twilight sparkle, -fluttershy, -pinkie pie, -rainbow dash, -applejack, -rarity";
        let fs = crate::tokenizer::parse(query);
        let exp = r#"{{{{{{{"pride flag" AND {NOT "oc"}} AND {NOT "twilight sparkle"}} AND {NOT "fluttershy"}} AND {NOT "pinkie pie"}} AND {NOT "rainbow dash"}} AND {NOT "applejack"}} AND {NOT "rarity"}}"#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}, Query: {}", e, q, query),
        };
        println!("Got: {}", q);
        let q = format!("{}", q);
        assert_eq!(exp, q);
        Ok(())
    }

    #[test]
    fn test_simplest_query() -> anyhow::Result<()> {
        let query = "sg";
        let fs = crate::tokenizer::parse(query);
        let exp = r#""sg""#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}, Query: {}", e, q, query),
        };
        println!("Got: {}", q);
        let q = format!("{}", q);
        assert_eq!(exp, q);
        Ok(())
    }

    #[test]
    fn test_logic_group() -> anyhow::Result<()> {
        let query = "pony OR human";
        let fs = crate::tokenizer::parse(query);
        let exp = r#"{"pony" OR "human"}"#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}, Query: {}", e, q, query),
        };
        println!("Got: {}", q);
        let q = format!("{}", q);
        assert_eq!(exp, q);
        Ok(())
    }

    #[cfg(feature = "search-with-tantivy")]
    #[test]
    fn test_death_query_tantivy() -> anyhow::Result<()> {
        let query = "sg AND (-pony-,(:),human (eqg)))";
        let fs = crate::tokenizer::parse(query);
        let exp = r#"BooleanQuery { subqueries: [(Must, TermQuery(Term(type=Str, field=0, "sg"))), (Must, BooleanQuery { subqueries: [(Must, BooleanQuery { subqueries: [(MustNot, TermQuery(Term(type=Str, field=0, "pony-")))] }), (Must, BooleanQuery { subqueries: [(Must, TermQuery(Term(type=Str, field=0, ":)"))), (Must, TermQuery(Term(type=Str, field=0, "human (eqg)")))] })] })] }"#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}", e, q),
        };
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("tag", tantivy::schema::STRING);
        let schema = schema_builder.build();
        let q = q.into_tantivy_search(&schema)?;
        println!("Got: {:?}", q);
        let q = format!("{:?}", q);
        assert_eq!(exp, q);
        Ok(())
    }

    #[test]
    fn test_attrcomp_query() -> anyhow::Result<()> {
        let query = "width.gte:1024,aspect_ratio.lte:2.0,created.lte:3 days ago";
        let fs = crate::tokenizer::parse(query);
        let exp = r#"{{?"width" GreaterEqual Integer(1024)? AND ?"aspect_ratio" LessEqual Float(2.0)?} AND ?"created" LessEqual String("3 days ago")?}"#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}, Query: {}", e, q, query),
        };
        println!("Got: {}", q);
        let q = format!("{}", q);
        assert_eq!(exp, q);
        Ok(())
    }

    #[cfg(feature = "search-with-tantivy")]
    #[test]
    fn test_attrcomp_query_tantivy() -> anyhow::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let query = "width.gte:1024,aspect_ratio.lte:2.0,created.lte:3 days ago";
        let predate = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("couldn't compute datetime");
        let fs = crate::tokenizer::parse(query);
        let date = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("couldn't compute datetime");
        assert_eq!(
            predate.as_secs(),
            date.as_secs(),
            "Must compute within 1 second"
        );
        let exp1 = r#"BooleanQuery { subqueries: [(Must, BooleanQuery { subqueries: [(Must, RangeQuery { field: Field(0), value_type: I64, left_bound: Unbounded, right_bound: Included([128, 0, 0, 0, 0, 0, 4, 0]) }), (Must, RangeQuery { field: Field(1), value_type: F64, left_bound: Included([192, 0, 0, 0, 0, 0, 0, 0]), right_bound: Unbounded })] }), (Must, RangeQuery { field: Field(2), value_type: Date, left_bound: Unbounded, right_bound: Included(["#;
        let exp2 = r#"]) })] }"#;
        let q = Query::from_foldstate(0, fs);
        let q = match q {
            Ok(v) => v,
            Err((q, e)) => anyhow::bail!("Error: {}: Got so far: {}", e, q),
        };
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_i64_field("width", tantivy::schema::INDEXED);
        schema_builder.add_f64_field("aspect_ratio", tantivy::schema::INDEXED);
        schema_builder.add_date_field("created", tantivy::schema::INDEXED);
        let schema = schema_builder.build();
        let q = q.into_tantivy_search(&schema)?;
        println!("Got: {:?}", q);
        let q = format!("{:?}", q);
        assert!(q.starts_with(exp1));
        assert!(q.ends_with(exp2));
        Ok(())
    }
}
