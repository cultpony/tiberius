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
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

#[cfg(test)]
mod secret_tests;

use async_std::sync::{RwLock, RwLockWriteGuard};
pub use models::*;

use chrono::NaiveDateTime;
pub use tantivy::TantivyError;
pub use tiberius_search::QueryError;
pub use tiberius_search::Queryable;

use async_trait::async_trait;
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tantivy::{IndexReader, IndexWriter};

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
    #[error("{}", .0)]
    Context(#[from] anyhow::Error),
}

impl From<ring::error::Unspecified> for PhilomenaModelError {
    fn from(_: ring::error::Unspecified) -> Self {
        Self::RingUnspec
    }
}

#[derive(Clone)]
pub struct Client {
    db: PgPool,
    search_dir: Option<std::path::PathBuf>,
    indices: Arc<RwLock<BTreeMap<String, tantivy::Index>>>,
    writers: Arc<RwLock<BTreeMap<String, Arc<RwLock<tantivy::IndexWriter>>>>>,
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
        }
    }
    #[deprecated(note = "Use Client directly since it implements the necessary interface")]
    pub(crate) async fn db(&self) -> Result<PoolConnection<Postgres>, PhilomenaModelError> {
        Ok(self.db.acquire().await?)
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

impl<'c> sqlx::Executor<'c> for &mut Client {
    type Database = sqlx::Postgres;

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
        self.db.fetch_many(query)
    }

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
        self.db.fetch_optional(query)
    }

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
        self.db.prepare_with(sql, parameters)
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> futures::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        self.db.describe(sql)
    }
}

/// Tables that implement this trait can be verified.
/// Verifying means all table entries are loaded, scanned and it's foreign keys loaded.
/// If any foreign keys are missing, it is noted in the log output.
#[cfg(feature = "verify-db")]
#[async_trait]
pub trait VerifiableTable {
    async fn verify(&mut self) -> Result<(), PhilomenaModelError>;
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
