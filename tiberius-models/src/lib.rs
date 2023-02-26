//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(deprecated)]

#[macro_use]
extern crate tracing;

mod models;
#[macro_use]
mod macros;
pub mod pluggables;
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
pub mod slug;

//#[cfg(test)]
//mod secret_tests;

use async_std::sync::{RwLock, RwLockWriteGuard};
use maud::PreEscaped;
pub use models::*;

use tiberius_dependencies::chrono::NaiveDateTime;
pub use tantivy::TantivyError;
use tiberius_dependencies::reqwest;
use tiberius_dependencies::{
    moka::future::Cache,
    totp_rs::{self, TotpUrlError},
};
pub use tiberius_search::{QueryError, Queryable};
use tiberius_dependencies::base64;

use async_trait::async_trait;
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tantivy::{IndexReader, IndexWriter};
use tracing::Level;

pub type Tx<'a> = &'a mut TxOwned<'a>;
pub type TxOwned<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
pub type Db = sqlx::pool::PoolConnection<Postgres>;
pub type ClientRef<'a> = &'a mut Client;

#[derive(thiserror::Error, Debug)]
pub enum PhilomenaModelError {
    #[error("Other error: {}", .0)]
    Other(String),
    #[error("Error in underlying datamodel: {}", .0)]
    SQLx(#[from] sqlx::Error),
    #[error("Could not deserialize upstream: {}", .0)]
    SerdeJson(#[from] serde_json::Error),
    #[error("Network Error in upstream API: {}", .0)]
    Reqwest(#[from] reqwest::Error),
    #[error("URL error: {}", .0)]
    Url(#[from] url::ParseError),
    #[error("Error in search parser: {}", .0)]
    Searcher(#[from] tiberius_search::QueryError),
    #[error("Error in search index: {}", .0)]
    Tantivy(#[from] TantivyError),
    #[error("IO Error: {}", .0)]
    IOError(#[from] std::io::Error),
    #[error("Could not find {} in {}", .1, .0)]
    NotFoundInSequence(String, String),
    #[error("Search not configured")]
    NoSearchConfigured,
    #[error("Column {} in {} {} was null", .column, .table, .id)]
    DataWasNull {
        column: String,
        table: String,
        id: String,
    },
    #[error("Could not convert: {}", .0)]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error("Unspecified Cryptographic error")]
    RingUnspec,
    #[error("Decode error: {}", .0)]
    Base64(#[from] base64::DecodeError),
    #[error("BCrypt Error: {}", .0)]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("{:?}", .0)]
    Context(#[from] anyhow::Error),
    #[error("TOTP Error: {:?}", .0)]
    TotpUrlError(totp_rs::TotpUrlError),
    #[error("Error parsing integer: {:?}", .0)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Consumed this TOTP Step already")]
    ConsumedTOTPAlready,
}

impl From<Arc<sqlx::Error>> for PhilomenaModelError {
    fn from(v: Arc<sqlx::Error>) -> Self {
        // TODO: fix the arc deref here
        Self::Other(v.to_string())
    }
}

impl From<ring::error::Unspecified> for PhilomenaModelError {
    fn from(_: ring::error::Unspecified) -> Self {
        Self::RingUnspec
    }
}

impl From<TotpUrlError> for PhilomenaModelError {
    fn from(v: TotpUrlError) -> Self {
        Self::TotpUrlError(v)
    }
}

#[derive(Clone)]
pub struct Client {
    db: PgPool,
    search_dir: Option<std::path::PathBuf>,
    indices: Arc<RwLock<BTreeMap<String, tantivy::Index>>>,
    writers: Arc<RwLock<BTreeMap<String, Arc<RwLock<tantivy::IndexWriter>>>>>,
    cache_users: Cache<i64, Option<User>>,
    cache_tag_assoc: Cache<ImageID, Vec<Tag>>,
}

impl Client {
    pub fn new(db: PgPool, search_dir: Option<&std::path::PathBuf>) -> Self {
        assert!(
            search_dir.map(|x| x.exists()).unwrap_or(true),
            "Search directory {:?} did not exist",
            search_dir
        );
        warn!("Creating new Database Client");
        Self {
            db,
            search_dir: search_dir.cloned(),
            indices: Arc::new(RwLock::new(BTreeMap::new())),
            writers: Arc::new(RwLock::new(BTreeMap::new())),
            cache_users: Cache::new(1000),
            cache_tag_assoc: Cache::new(1000),
        }
    }
    #[deprecated(note = "Use Client directly since it implements the necessary interface")]
    pub(crate) async fn db(&self) -> Result<PoolConnection<Postgres>, PhilomenaModelError> {
        Ok(self.db.acquire().await?)
    }

    /// Returns an instance of the recommendation engine used to show users images they might like
    pub fn recommendation_engine(&self) -> Result<(), ()> {
        todo!()
    }

    pub async fn index_writer<T: Queryable>(
        &mut self,
    ) -> Result<Arc<RwLock<IndexWriter>>, PhilomenaModelError> {
        let search_dir = match &self.search_dir {
            Some(v) => v.clone(),
            None => return Err(PhilomenaModelError::NoSearchConfigured),
        };
        let t = std::any::type_name::<T>();

        trace!("Need index writer {}", t);
        let writer_readg = self.writers.read().await;
        let writer = writer_readg.get(t);
        if let Some(writer) = writer {
            trace!("Index writer exists, cloning existing lock");
            let r = Ok(writer.clone());
            drop(writer_readg);
            r
        } else {
            trace!("Index writer does not exist creating");
            let indexrg = self.indices.read().await;
            trace!("Got read lock on index list");
            let index = indexrg.get(t);
            let writer = if index.is_none() {
                drop(indexrg);
                trace!("Dropped read lock on index, upgrading to setup index reader");
                let mut iwg = self.indices.write().await;
                let i = if let Some(i) = iwg.get(t) {
                    trace!("Someone setup index while checking, using that one");
                    i.clone()
                } else {
                    trace!("Index does not exist, opening");
                    let i = T::open_or_create_index(search_dir.clone())?;
                    iwg.insert(t.to_string(), i.clone());
                    i
                };
                drop(iwg);
                trace!("Creating writer for new index");
                let w = i.writer(10_000_000)?;
                w
            } else {
                let index = index.unwrap();
                trace!("Creating writer for open index");
                let w = index.writer(10_000_000)?;
                drop(indexrg);
                w
            };
            // we need to hold the indexrg until we have the writer while we setup the index in the cache
            let writer = Arc::new(RwLock::new(writer));
            trace!("Inserting writer into cache");
            drop(writer_readg);
            let mut w = self.writers.write().await;
            if let Some(w) = w.get(&t.to_string()) {
                // double check if we didn't setup a write while we upgraded the lock
                return Ok(w.clone());
            }
            w.insert(t.to_string(), writer.clone());
            drop(w);
            Ok(writer.clone())
        }
    }
    pub fn index_reader<T: Queryable>(&mut self) -> Result<IndexReader, PhilomenaModelError> {
        let search_dir = match &self.search_dir {
            Some(v) => v.clone(),
            None => return Err(PhilomenaModelError::NoSearchConfigured),
        };
        let i = T::open_or_create_index(search_dir.clone())?;
        Ok(i.reader()?)
    }
    pub(crate) async fn clone_new_conn(&self, pool: &PgPool) -> Result<Self, PhilomenaModelError> {
        Ok(Self {
            db: pool.clone(),
            search_dir: self.search_dir.clone(),
            indices: self.indices.clone(),
            writers: self.writers.clone(),
            cache_users: Cache::new(1000),
            cache_tag_assoc: Cache::new(1000),
        })
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("db", &self.db)
            .field("search_dir", &self.search_dir)
            .finish()
    }
}

impl From<PgPool> for Client {
    fn from(p: PgPool) -> Self {
        Client::new(p, None)
    }
}

impl From<&PgPool> for Client {
    fn from(p: &PgPool) -> Self {
        Client::new(p.clone(), None)
    }
}

impl From<&mut PgPool> for Client {
    fn from(p: &mut PgPool) -> Self {
        Client::new(p.clone(), None)
    }
}

impl<'c> sqlx::Executor<'c> for &mut Client {
    type Database = sqlx::Postgres;

    #[instrument(skip(query), fields(query = query.sql()))]
    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<
        'e,
        Result<
            itertools::Either<
                <Self::Database as sqlx::Database>::QueryResult,
                <Self::Database as sqlx::Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        use tiberius_dependencies::tracing_futures::{Instrument, WithSubscriber};
        Box::pin(self.db.fetch_many(query).instrument(tracing::span::Span::current()))
        //Box::pin(self.db.fetch_many(query).instrument(tracing::debug_span!("fetch_many")))
    }

    #[instrument(skip(query), fields(query = query.sql()))]
    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> futures::future::BoxFuture<
        'e,
        Result<Option<<Self::Database as sqlx::Database>::Row>, sqlx::Error>,
    >
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        use tiberius_dependencies::tracing_futures::Instrument;
        Box::pin(self.db.fetch_optional(query).instrument(tracing::span::Span::current()))
    }

    #[instrument(skip(parameters))]
    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> futures::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::database::HasStatement<'q>>::Statement, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        use tiberius_dependencies::tracing_futures::Instrument;
        Box::pin(self.db.prepare_with(sql, parameters).instrument(tracing::span::Span::current()))
    }

    #[instrument]
    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> futures::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        use tiberius_dependencies::tracing_futures::Instrument;
        Box::pin(self.db.describe(sql).instrument(tracing::span::Span::current()))
    }
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Adverts {
    pub id: i64,
    pub image: Option<String>,
    pub link: Option<String>,
    pub title: Option<String>,
    pub clicks: Option<i32>,
    pub impressions: Option<i32>,
    pub live: Option<bool>,
    pub start_date: Option<NaiveDateTime>,
    pub finish_date: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub restrictions: Option<String>,
    pub notes: Option<String>,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct BadgeAwards {
    pub id: i64,
    pub label: Option<String>,
    pub awarded_on: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: i64,
    pub badge_id: i64,
    pub awarded_by_id: i64,
    pub reason: Option<String>,
    pub badge_name: Option<String>,
}

/// This trait must only be implemented if *ALL* fields of a struct can be pushed into the public API as a response
pub trait SafeSerialize {
    type Target: serde::Serialize;

    fn into_safe(&self) -> Self::Target;
}

pub trait DirectSafeSerialize: serde::Serialize {}

pub trait Identifiable {
    fn id(&self) -> i64;
}

#[async_trait]
pub trait IdentifiesUser {
    async fn best_user_identifier(
        &self,
        client: &mut Client,
    ) -> Result<String, PhilomenaModelError>;
    fn user_id(&self) -> Option<i64>;
    fn is_anonymous(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}
