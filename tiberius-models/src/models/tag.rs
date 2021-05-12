use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;

use async_std::sync::RwLock;
use chrono::NaiveDateTime;
use futures::Stream;
use sqlx::postgres::PgRow;
use sqlx::{query_as, Executor, PgPool};
use tantivy::IndexWriter;
use tiberius_search::Queryable;

use crate::{
    doc_add_, tantivy_date_field, tantivy_indexed_text_field, tantivy_raw_text_field,
    tantivy_text_field, tantivy_u64_field, Client, PhilomenaModelError,
};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub namespace: Option<String>,
    pub name_in_namespace: Option<String>,
    pub images_count: i32,
    pub image: Option<String>,
    pub image_format: Option<String>,
    pub image_mime_type: Option<String>,
    pub aliased_tag_id: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub category: Option<String>,
    pub mod_notes: Option<String>,
}

impl Tag {
    pub fn full_name(&self) -> String {
        match &self.namespace {
            Some(namespace) => format!("{}:{}", namespace, self.name),
            None => self.name.clone(),
        }
    }
    pub async fn get_many(
        client: &mut Client,
        ids: Vec<i64>,
    ) -> Result<Vec<Self>, PhilomenaModelError> {
        let ids: Vec<i32> = ids.iter().map(|x| *x as i32).collect();
        Ok(
            query_as!(Self, "SELECT * FROM tags WHERE id = ANY($1)", &ids)
                .fetch_all(client.db().await?.deref_mut())
                .await?,
        )
    }
    pub async fn get_many_by_name(
        client: &mut Client,
        names: Vec<(String, Option<String>)>,
        allow_missing: bool,
    ) -> Result<Vec<Self>, PhilomenaModelError> {
        //TODO: optimize this!
        let mut tags = Vec::new();
        for (name, namespace) in &names {
            let tag = Self::get_by_name(client, namespace.clone(), name.clone()).await?;
            match tag {
                None => {
                    if !allow_missing {
                        return Err(PhilomenaModelError::NotFoundInSequence(
                            "tags".to_string(),
                            format!("{:?}:{:?}", namespace, name),
                        ));
                    }
                }
                Some(tag) => tags.push(tag),
            }
        }
        Ok(tags)
    }
    pub async fn get(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(
            query_as!(Self, "SELECT * FROM tags WHERE id = $1", id as i32)
                .fetch_optional(client.db().await?.deref_mut())
                .await?,
        )
    }
    pub async fn get_by_name(
        client: &mut Client,
        namespace: Option<String>,
        name: String,
    ) -> Result<Option<Self>, PhilomenaModelError> {
        tracing::debug!("Grabbing by_name tag {:?}:{:?}", namespace, name);
        match namespace {
            Some(namespace) => Ok(query_as!(
                Self,
                "SELECT * FROM tags WHERE namespace = $1 AND name_in_namespace = $2",
                namespace,
                name,
            )
            .fetch_optional(client.db().await?.deref_mut())
            .await?),
            None => Ok(query_as!(
                Self,
                "SELECT * FROM tags WHERE namespace IS NULL AND name_in_namespace = $1",
                name,
            )
            .fetch_optional(client.db().await?.deref_mut())
            .await?),
        }
    }
    pub async fn get_all(
        pool: PgPool,
        start_id: Option<u64>,
        end_id: Option<u64>,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PgRow, sqlx::Error>>>>, PhilomenaModelError>
    {
        match (start_id, end_id) {
            (Some(start_id), Some(end_id)) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM tags WHERE id BETWEEN $1 AND $2",
                start_id as i64,
                end_id as i64
            ))),
            (Some(start_id), None) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM tags WHERE id > $1",
                start_id as i64
            ))),
            (None, Some(end_id)) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM tags WHERE id <= $1",
                end_id as i64
            ))),
            (None, None) => Ok(pool.fetch(sqlx::query!("SELECT * FROM tags"))),
        }
    }
    pub async fn autocomplete<S: Into<String>>(
        client: &mut Client,
        term: S,
    ) -> Result<(usize, Vec<Tag>), PhilomenaModelError> {
        use tiberius_search::tantivy::{query::TermQuery, schema::IndexRecordOption, schema::Term};
        let query = TermQuery::new(
            Term::from_field_text(Self::schema().get_field("full_name").unwrap(), &term.into()),
            IndexRecordOption::Basic,
        );
        let i: tiberius_search::tantivy::IndexReader = client.index_reader::<Tag>()?;
        let res = Self::search_tantivy_query(&i, query, 10, 0)?;
        let res = {
            let tags: Vec<i64> = res.1.iter().map(|(score, id)| *id as i64).collect();
            let tags = Self::get_many(client, tags).await?;
            (res.0, tags)
        };
        Ok(res)
    }
}

#[async_trait::async_trait]
impl Queryable for Tag {
    type Group = String;
    type DBClient = Client;
    type IndexError = PhilomenaModelError;

    fn identifier(&self) -> u64 {
        self.id as u64
    }

    fn group() -> Self::Group {
        "tags".to_string()
    }

    fn schema() -> tantivy::schema::Schema {
        use schema::*;
        use tantivy::*;
        let mut builder = Schema::builder();
        tantivy_date_field!(builder, created_at);
        tantivy_u64_field!(builder, id);
        tantivy_raw_text_field!(builder, name);
        tantivy_raw_text_field!(builder, slug);
        tantivy_indexed_text_field!(builder, description);
        tantivy_indexed_text_field!(builder, short_description);
        tantivy_raw_text_field!(builder, namespace);
        tantivy_raw_text_field!(builder, name_in_namespace);
        tantivy_indexed_text_field!(builder, full_name);
        tantivy_u64_field!(builder, images_count);
        tantivy_u64_field!(builder, aliased_tag_id);
        tantivy_raw_text_field!(builder, category);
        builder.build()
    }

    async fn index(
        &self,
        writer: Arc<RwLock<IndexWriter>>,
        _: &mut Self::DBClient,
    ) -> std::result::Result<(), Self::IndexError> {
        let mut doc = tantivy::Document::new();
        let schema = Self::schema();
        doc_add_!(
            doc,
            schema,
            date,
            created_at,
            &chrono::DateTime::<chrono::Utc>::from_utc(self.created_at, chrono::Utc)
        );
        doc_add_!(doc, schema, u64, id, self.id as u64);
        doc_add_!(doc, schema, text, name, &self.name);
        doc_add_!(doc, schema, text, slug, &self.slug);
        doc_add_!(doc, schema, option<text>, description, &self.description);
        doc_add_!(
            doc,
            schema,
            option<text>,
            short_description,
            &self.short_description
        );
        doc_add_!(doc, schema, option<text>, namespace, &self.namespace);
        doc_add_!(
            doc,
            schema,
            option<text>,
            name_in_namespace,
            &self.name_in_namespace
        );
        doc_add_!(doc, schema, text, full_name, &self.full_name());
        doc_add_!(doc, schema, u64, images_count, self.images_count as u64);
        doc_add_!(
            doc,
            schema,
            option<u64>,
            aliased_tag_id,
            self.aliased_tag_id.map(|x| x as u64)
        );
        doc_add_!(doc, schema, option<text>, category, &self.category);
        let writer = writer.write().await;
        writer.add_document(doc);
        drop(writer);
        Ok(())
    }

    async fn delete_from_index(
        &self,
        writer: Arc<RwLock<IndexWriter>>,
    ) -> std::result::Result<(), Self::IndexError> {
        use tantivy::Term;
        let writer = writer.write().await;
        writer.delete_term(Term::from_field_u64(
            Self::schema().get_field("id").unwrap(),
            self.id as u64,
        ));
        drop(writer);
        Ok(())
    }
}
