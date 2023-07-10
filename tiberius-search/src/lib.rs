use std::{str::FromStr, sync::Arc};

pub(crate) mod query;
pub(crate) mod tokenizer;

use async_std::sync::RwLock;
pub use query::{Match, Query, QueryError};
use tracing::*;

#[cfg(feature = "search-with-tantivy")]
pub use tantivy;
#[cfg(feature = "search-with-tantivy")]
use tantivy::query::Occur;
#[cfg(feature = "search-with-tantivy")]
use tantivy::*;

pub fn parse<S: Into<String>>(s: S) -> std::result::Result<Query, (Query, QueryError)> {
    let s: String = s.into();
    Query::from_str(&s)
}

pub enum SortFieldType {
    /// U64 Field
    Integer,
    /// I64 Field
    SignedInteger,
    /// F32 Field
    Float,
    /// String Field
    String,
}

pub trait SortIndicator: std::fmt::Debug {
    /// Indicate that search is to randomize the score
    fn random(&self) -> bool;
    fn field(&self) -> &'static str;
    /// Indicate that search order is reversed, this does not affect anything if random returns true
    fn invert_sort(&self) -> bool;
    fn field_type(&self) -> SortFieldType {
        SortFieldType::Integer
    }
}

#[async_trait::async_trait]
pub trait Queryable {
    type Group: Into<String>;
    type DBClient;
    type IndexError: std::error::Error;
    type SortIndicator: SortIndicator;

    /// Return a unique identifer (UUID as u128 or Integer) for the object
    fn identifier(&self) -> u64;
    /// Must return the group the object belongs to, such as "tags" or "images"
    fn group() -> Self::Group;

    #[cfg(feature = "search-with-tantivy")]
    /// Provide a standard schema (all text fields and a "tags" and "id" field)
    /// If you require a different schema, override
    /// This will be executed once per search, caching is recommended, keep this cheap
    /// The "id" field **MUST** be present.
    fn schema() -> tantivy::schema::Schema;

    #[cfg(feature = "search-with-tantivy")]
    async fn index(
        &self,
        writer: Arc<RwLock<IndexWriter>>,
        db: &mut Self::DBClient,
    ) -> std::result::Result<(), Self::IndexError>;

    /// This returns the raw document used for indexing
    /// Returning the document helps identify if an image requires a reindex or not.
    /// If omit_index_only is set, index only fields must be skipped
    #[cfg(feature = "search-with-tantivy")]
    async fn get_doc(
        &self,
        db: &mut Self::DBClient,
        omit_index_only: bool,
    ) -> std::result::Result<Document, Self::IndexError>;

    #[cfg(feature = "search-with-tantivy")]
    async fn get_from_index(
        reader: crate::IndexReader,
        id: u64,
    ) -> std::result::Result<Option<Document>, Self::IndexError>;

    #[cfg(feature = "search-with-tantivy")]
    async fn delete_from_index(
        &self,
        writer: Arc<RwLock<IndexWriter>>,
    ) -> std::result::Result<(), Self::IndexError>;

    #[cfg(feature = "search-with-tantivy")]
    /// This should not be overridden by implementing objects
    /// The given path must be a directory, if it does not exist, return an error
    /// The directory will have the object's group appended to it
    fn open_or_create_index(path: std::path::PathBuf) -> std::result::Result<Index, TantivyError> {
        if !path.exists() {
            return Err(TantivyError::SystemError("path does not exist".to_string()));
        }
        let group: String = Self::group().into();
        let path = path.join(group);
        debug!("Opening Index {}", path.display());
        if !path.exists() {
            debug!("Creating Index {}", path.display());
            std::fs::create_dir(path.clone())?;
        }
        let path = directory::MmapDirectory::open(path)?;
        let path = directory::ManagedDirectory::wrap(Box::new(path))?;
        let index = Index::open_or_create(path, Self::schema())?;
        let autocomplete_tokenizer = {
            use tantivy::tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer};
            let tokenizer = NgramTokenizer::new(2, 50, true);
            TextAnalyzer::builder(tokenizer).filter(LowerCaser).build()
        };
        index
            .tokenizers()
            .register("autocomplete", autocomplete_tokenizer);
        Ok(index)
    }

    #[cfg(feature = "search-with-tantivy")]
    /// Returns a list of IDs of documents that matched
    /// q is populated by the query the user supplies
    /// aq is a list of queries that any result must also match
    /// anq is a list of queries that any result must not match
    fn search_item(
        i: &IndexReader,
        q: crate::query::Query,
        aq: Vec<crate::query::Query>,
        anq: Vec<crate::query::Query>,
        limit: usize,
        offset: usize,
        dir: Self::SortIndicator,
    ) -> std::result::Result<(usize, Vec<(f32, u64)>), QueryError> {
        debug!(
            "Converting query: {}, offset: {}, limit: {}",
            q, offset, limit
        );
        let schema = Self::schema();
        let q = q.into_tantivy_search(&schema)?;
        let aq_len = aq.len();
        let mut aq: Vec<Box<dyn tantivy::query::Query>> = aq
            .into_iter()
            .flat_map(|x| x.into_tantivy_search(&schema))
            .collect();
        if aq.len() != aq_len {
            return Err(QueryError::AuxQueryError(
                "one more auxiliary queries did not parse".to_string(),
            ));
        }
        let anq_len = anq.len();
        let anq: Vec<Box<dyn tantivy::query::Query>> = anq
            .into_iter()
            .flat_map(|x| x.into_tantivy_search(&schema))
            .collect();
        if anq.len() != anq_len {
            return Err(QueryError::AuxQueryError(
                "one more auxiliary inverted queries did not parse".to_string(),
            ));
        }
        debug!("Merging in auxiliary queries");
        aq.push(q);
        let mut aq: Vec<(Occur, Box<dyn tantivy::query::Query>)> =
            aq.into_iter().map(|x| (Occur::Must, x)).collect();
        let anq: Vec<(Occur, Box<dyn tantivy::query::Query>)> =
            anq.into_iter().map(|x| (Occur::MustNot, x)).collect();
        aq.extend(anq);
        let q = tantivy::query::BooleanQuery::new(aq);
        Self::search_tantivy_query(i, q, limit, offset, dir)
    }

    #[cfg(feature = "search-with-tantivy")]
    fn search_tantivy_query<T: tantivy::query::Query>(
        i: &IndexReader,
        q: T,
        limit: usize,
        offset: usize,
        dir: Self::SortIndicator,
    ) -> std::result::Result<(usize, Vec<(f32, u64)>), QueryError> {
        debug!(
            "Beginning query: {:?}, offset: {}, limit: {}",
            q, offset, limit
        );
        let schema = Self::schema();
        use tantivy::collector::*;
        let coll = TopDocs::with_limit(limit).and_offset(offset);
        let coll = {
            let field_type = dir.field_type();
            let field = dir.field();
            let dir: f64 = if dir.invert_sort() { -1.0 } else { 1.0 };
            match field_type {
                SortFieldType::Integer => {
                    coll.custom_score(move |segment_reader: &SegmentReader| {
                        let pop_reader = segment_reader.fast_fields().u64(field).unwrap();
                        move |doc: DocId| {
                            let pop = pop_reader.values.get_val(doc);
                            (if dir.is_sign_negative() {
                                u64::MAX - pop
                            } else {
                                pop
                            }) as f32
                        }
                    })
                }
                SortFieldType::SignedInteger => todo!(),
                SortFieldType::Float => todo!(),
                SortFieldType::String => todo!(),
            }
        };
        let searcher = i.searcher();
        let field = schema.get_field("id")?;
        debug!("Counting Documents matching query");
        //let count = searcher.search(&q, &Count)?;
        let count = q.count(&searcher)?;
        debug!("Retrieving page window");
        let res = searcher.search(&q, &coll)?;

        debug!("Producing output vector with data");
        let mut out = Vec::new();
        for (score, addr) in res.iter() {
            let doc = searcher.doc(*addr)?;
            trace!("Got document: {:?}", doc);
            let value = doc.get_first(field);
            if let Some(v) = value.and_then(|x| x.as_u64()) {
                out.push((*score, v))
            }
        }

        debug!("Completed query, got {} results", count);

        Ok((count, out))
    }

    #[cfg(feature = "search-with-tantivy")]
    fn search_item_with_str<S: Into<String>, S1: Into<String>, S2: Into<String>>(
        i: &IndexReader,
        s: S,
        aq: Vec<S1>,
        anq: Vec<S2>,
        limit: usize,
        offset: usize,
        dir: Self::SortIndicator,
    ) -> std::result::Result<(usize, Vec<(f32, u64)>), QueryError> {
        let s: String = s.into();
        let q = crate::query::Query::from_str(&s);
        let q = match q {
            Err((q, qe)) => {
                return Err(QueryError::OperatorError(format!(
                    "Error in query: {} -> {}",
                    qe, q
                )))
            }
            Ok(v) => v,
        };
        let aq: Vec<Query> = aq
            .into_iter()
            .flat_map(|s| crate::query::Query::from_str(&s.into()))
            .collect();
        let anq: Vec<Query> = anq
            .into_iter()
            .flat_map(|s| crate::query::Query::from_str(&s.into()))
            .collect();
        Self::search_item(i, q, aq, anq, limit, offset, dir)
    }
}
