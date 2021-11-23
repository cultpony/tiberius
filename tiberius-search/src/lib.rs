use std::str::FromStr;
use std::sync::Arc;

pub(crate) mod query;
pub(crate) mod tokenizer;

use async_std::sync::RwLock;
pub use query::Match;
pub use query::Query;
pub use query::QueryError;
use tracing::info;

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

#[async_trait::async_trait]
pub trait Queryable {
    type Group: Into<String>;
    type DBClient;
    type IndexError: std::error::Error;

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
        info!("Opening Index {}", path.display());
        if !path.exists() {
            info!("Creating Index {}", path.display());
            std::fs::create_dir(path.clone())?;
        }
        let path = directory::MmapDirectory::open(path)?;
        let path = directory::ManagedDirectory::wrap(path)?;
        let index = Index::open_or_create(path, Self::schema())?;
        let autocomplete_tokenizer = {
            use tantivy::tokenizer::LowerCaser;
            use tantivy::tokenizer::NgramTokenizer;
            use tantivy::tokenizer::TextAnalyzer;
            let tokenizer = NgramTokenizer::new(2, 50, true);
            TextAnalyzer::from(tokenizer).filter(LowerCaser)
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
    ) -> std::result::Result<(usize, Vec<(f32, u64)>), QueryError> {
        info!(
            "Converting query: {}, offset: {}, limit: {}",
            q, offset, limit
        );
        let schema = Self::schema();
        let q = q.into_tantivy_search(&schema)?;
        let aq_len = aq.len();
        let mut aq: Vec<Box<dyn tantivy::query::Query>> = aq
            .into_iter()
            .map(|x| x.into_tantivy_search(&schema))
            .flatten()
            .collect();
        if aq.len() != aq_len {
            return Err(QueryError::AuxQueryError(
                "one more auxiliary queries did not parse".to_string(),
            ));
        }
        let anq_len = anq.len();
        let anq: Vec<Box<dyn tantivy::query::Query>> = anq
            .into_iter()
            .map(|x| x.into_tantivy_search(&schema))
            .flatten()
            .collect();
        if anq.len() != anq_len {
            return Err(QueryError::AuxQueryError(
                "one more auxiliary inverted queries did not parse".to_string(),
            ));
        }
        info!("Merging in auxiliary queries");
        aq.push(q);
        let mut aq: Vec<(Occur, Box<dyn tantivy::query::Query>)> =
            aq.into_iter().map(|x| (Occur::Must, x)).collect();
        let anq: Vec<(Occur, Box<dyn tantivy::query::Query>)> =
            anq.into_iter().map(|x| (Occur::MustNot, x)).collect();
        aq.extend(anq);
        let q = tantivy::query::BooleanQuery::new(aq);
        Self::search_tantivy_query(i, q, limit, offset)
    }

    #[cfg(feature = "search-with-tantivy")]
    fn search_tantivy_query<T: tantivy::query::Query>(
        i: &IndexReader,
        q: T,
        limit: usize,
        offset: usize,
    ) -> std::result::Result<(usize, Vec<(f32, u64)>), QueryError> {
        info!(
            "Beginning query: {:?}, offset: {}, limit: {}",
            q, offset, limit
        );
        let schema = Self::schema();
        use tantivy::collector::*;
        let coll = TopDocs::with_limit(limit).and_offset(offset);
        let searcher = i.searcher();
        let field = schema.get_field("id");
        let field = match field {
            Some(f) => f,
            None => panic!("could not find ID field"),
        };
        let count = searcher.search(&q, &Count)?;
        let res = searcher.search(&q, &coll)?;

        let mut out = Vec::new();
        for (score, addr) in res.iter() {
            let doc = searcher.doc(*addr)?;
            let value = doc.get_first(field);
            let value = value.map(|x| x.u64_value()).flatten();
            match value {
                Some(v) => out.push((*score, v)),
                None => continue,
            }
        }

        info!("Completed query, got {} results", count);

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
            .map(|s| crate::query::Query::from_str(&s.into()))
            .flatten()
            .collect();
        let anq: Vec<Query> = anq
            .into_iter()
            .map(|s| crate::query::Query::from_str(&s.into()))
            .flatten()
            .collect();
        Self::search_item(i, q, aq, anq, limit, offset)
    }
}
