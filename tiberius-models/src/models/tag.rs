use std::{cmp::Ordering, ops::DerefMut, pin::Pin, sync::Arc};

use async_std::sync::RwLock;
use tiberius_dependencies::chrono::{NaiveDate, NaiveDateTime, Utc};
use futures::Stream;
use itertools::Itertools;
use sqlx::{postgres::PgRow, query_as, Executor, PgPool};
use tantivy::{Document, IndexWriter};
use tiberius_search::{Queryable, SortIndicator};

use crate::{
    doc_add_, slug::sluggify, tantivy_date_field, tantivy_indexed_text_field,
    tantivy_raw_text_field, tantivy_text_field, tantivy_u64_field, Client, PhilomenaModelError,
    SortDirection,
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

/// A reduced version of tag often obtained from views or queries
#[derive(Clone, Debug)]
pub struct TagView {
    pub id: u64,
    pub name: String,
    pub namespace: Option<String>,
    pub name_in_namespace: Option<String>,
    pub category: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub images_count: i32,
}

impl Into<TagView> for Tag {
    fn into(self) -> TagView {
        TagView {
            id: self.id as u64,
            name: self.name,
            namespace: self.namespace,
            name_in_namespace: self.name_in_namespace,
            category: self.category,
            slug: Some(self.slug),
            description: self.description,
            images_count: self.images_count,
        }
    }
}

pub trait TagLike: Ord {
    fn full_name(&self) -> String;

    fn path_full_name(&self) -> String {
        let full_name = self.full_name();
        sluggify(full_name)
    }
}

impl TagLike for Tag {
    fn full_name(&self) -> String {
        match &self.namespace {
            Some(namespace) => match &self.name_in_namespace {
                Some(name_in_namespace) => format!("{}:{}", namespace, name_in_namespace),
                None => todo!(),
            },
            None => self.name.clone(),
        }
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self {
            id: i32::MAX,
            name: Default::default(),
            slug: Default::default(),
            description: None,
            short_description: None,
            namespace: None,
            name_in_namespace: None,
            images_count: 0,
            image: Default::default(),
            image_format: Default::default(),
            image_mime_type: Default::default(),
            aliased_tag_id: Default::default(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            category: None,
            mod_notes: None,
        }
    }
}

fn category_priority(cat: &str) -> i8 {
    match cat {
        "error" => 0,
        "rating" => 1,
        "origin" => 2,
        "character" => 3,
        "oc" => 4,
        "species" => 5,
        "content-fanmade" => 6,
        "content-official" => 7,
        "spoiler" => 8,
        _ => i8::MAX,
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.slug == other.slug
            && self.description == other.description
            && self.short_description == other.short_description
            && self.namespace == other.namespace
            && self.name_in_namespace == other.name_in_namespace
            && self.image == other.image
            && self.image_format == other.image_format
            && self.image_mime_type == other.image_mime_type
            && self.aliased_tag_id == other.aliased_tag_id
            && self.category == other.category
            && self.mod_notes == other.mod_notes
    }
}

impl Eq for Tag {}
impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (scat, ocat) = match (self.category.as_ref(), other.category.as_ref()) {
            (None, None) => return Some(self.full_name().cmp(&other.full_name())),
            (None, Some(_)) => return Some(Ordering::Greater),
            (Some(_), None) => return Some(Ordering::Less),
            (Some(v), Some(w)) => (v, w),
        };
        if scat == ocat {
            return Some(self.full_name().cmp(&other.full_name()));
        }
        let scat = category_priority(&*scat);
        let ocat = category_priority(&*ocat);
        match scat.cmp(&ocat) {
            Ordering::Less => return Some(Ordering::Less),
            Ordering::Equal => return Some(self.full_name().cmp(&other.full_name())),
            Ordering::Greater => return Some(Ordering::Greater),
        }
    }
}

impl PartialEq for TagView {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.slug == other.slug
            && self.description == other.description
            && self.namespace == other.namespace
            && self.name_in_namespace == other.name_in_namespace
            && self.category == other.category
    }
}

impl Eq for TagView {}
impl Ord for TagView {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for TagView {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (scat, ocat) = match (self.category.as_ref(), other.category.as_ref()) {
            (None, None) => return Some(self.full_name().cmp(&other.full_name())),
            (None, Some(_)) => return Some(Ordering::Greater),
            (Some(_), None) => return Some(Ordering::Less),
            (Some(v), Some(w)) => (v, w),
        };
        if scat == ocat {
            return Some(self.full_name().cmp(&other.full_name()));
        }
        let scat = category_priority(&*scat);
        let ocat = category_priority(&*ocat);
        match scat.cmp(&ocat) {
            Ordering::Less => return Some(Ordering::Less),
            Ordering::Equal => return Some(self.full_name().cmp(&other.full_name())),
            Ordering::Greater => return Some(Ordering::Greater),
        }
    }
}

impl TagLike for TagView {
    fn full_name(&self) -> String {
        match &self.namespace {
            Some(namespace) => match &self.name_in_namespace {
                Some(name_in_namespace) => format!("{}:{}", namespace, name_in_namespace),
                None => todo!(),
            },
            None => self.name.clone(),
        }
    }
}

impl Tag {
    #[cfg(test)]
    pub async fn create_for_test<S: AsRef<str>>(
        client: &mut Client,
        tag: S,
    ) -> Result<Tag, PhilomenaModelError> {
        use sqlx::query;
        let tag: &str = tag.as_ref();
        let slug = sluggify(tag);
        let tag_id = query!("INSERT INTO tags (name, slug, created_at, updated_at) VALUES ($1, $2, NOW(), NOW()) ON CONFLICT DO NOTHING RETURNING id", tag, slug).fetch_one(&mut client.clone()).await?;
        let tag_id = tag_id.id;
        Ok(Tag::get(client, tag_id as i64)
            .await?
            .expect("tag created but doesn't exist?"))
    }

    pub fn create_cache_tagline(tags: &[Tag]) -> String {
        tags.iter().sorted().map(|x| x.full_name()).join(", ")
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
        use tiberius_search::tantivy::{
            query::TermQuery,
            schema::{IndexRecordOption, Term},
        };
        let query = TermQuery::new(
            Term::from_field_text(Self::schema().get_field("full_name").unwrap(), &term.into()),
            IndexRecordOption::Basic,
        );
        let i: tiberius_search::tantivy::IndexReader = client.index_reader::<Tag>()?;
        let res = Self::search_tantivy_query(
            &i,
            query,
            10,
            0,
            TagSortBy::ImageCount(SortDirection::Descending),
        )?;
        let res = {
            let tags: Vec<i64> = res.1.iter().map(|(score, id)| *id as i64).collect();
            let tags = Self::get_many(client, tags).await?;
            (res.0, tags)
        };
        Ok(res)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSortBy {
    Random,
    ImageCount(SortDirection),
    Alphabetical(SortDirection),
}

impl SortIndicator for TagSortBy {
    fn random(&self) -> bool {
        match self {
            TagSortBy::Random => true,
            _ => false,
        }
    }

    fn field(&self) -> &'static str {
        match self {
            TagSortBy::Random => "id",
            TagSortBy::ImageCount(_) => "image_count",
            TagSortBy::Alphabetical(_) => "name",
        }
    }

    fn invert_sort(&self) -> bool {
        let dir = match self {
            TagSortBy::Random => return false,
            TagSortBy::ImageCount(dir) => dir,
            TagSortBy::Alphabetical(dir) => dir,
        };
        match dir {
            SortDirection::Ascending => true,
            SortDirection::Descending => false,
        }
    }

    fn field_type(&self) -> tiberius_search::SortFieldType {
        match self {
            TagSortBy::Alphabetical(_) => tiberius_search::SortFieldType::String,
            _ => tiberius_search::SortFieldType::Integer,
        }
    }
}

#[async_trait::async_trait]
impl Queryable for Tag {
    type Group = String;
    type DBClient = Client;
    type IndexError = PhilomenaModelError;
    type SortIndicator = TagSortBy;

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
        client: &mut Self::DBClient,
    ) -> std::result::Result<(), Self::IndexError> {
        let mut client = client.clone();
        let doc = self.get_doc(&mut client, false).await?;
        //debug!("Sending {:?} to index", doc);
        writer.write().await.add_document(doc)?;
        Ok(())
    }

    async fn get_doc(
        &self,
        client: &mut Self::DBClient,
        omit_index_only: bool,
    ) -> std::result::Result<Document, Self::IndexError> {
        let mut doc = tantivy::Document::new();
        let schema = Self::schema();
        doc_add_!(
            doc,
            schema,
            date,
            created_at,
            tantivy::DateTime::from_timestamp_secs(
                tiberius_dependencies::chrono::DateTime::<tiberius_dependencies::chrono::Utc>::from_utc(self.created_at, tiberius_dependencies::chrono::Utc).timestamp()
            )
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
        Ok(doc)
    }

    async fn get_from_index(
        reader: crate::IndexReader,
        id: u64,
    ) -> std::result::Result<Option<Document>, Self::IndexError> {
        let term =
            tantivy::Term::from_field_u64(Self::schema().get_field("id").unwrap(), id as u64);
        let coll = tantivy::collector::TopDocs::with_limit(1).and_offset(0);
        let query = tantivy::query::TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);
        let res = reader.searcher().search(&query, &coll)?;
        let res = res.get(0);
        let res = match res {
            Some(res) => res.1,
            None => return Ok(None),
        };
        let doc = reader.searcher().doc(res)?;
        Ok(Some(doc))
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

#[cfg(test)]
mod test {
    use super::TagLike;
    use crate::Tag;
    use anyhow::Result;

    #[test]
    pub fn fullname_forming() {
        assert_eq!(
            "artist:test",
            Tag {
                name: "artist:test".to_string(),
                namespace: Some("artist".to_string()),
                name_in_namespace: Some("test".to_string()),
                ..Default::default()
            }
            .full_name()
        )
    }

    #[test]
    pub fn tag_sorting() -> Result<()> {
        let tag_list = vec![
            Tag {
                name: "testA".to_string(),
                category: Some("spoiler".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testB".to_string(),
                category: Some("spoiler".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testC".to_string(),
                category: Some("content-official".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testD".to_string(),
                category: Some("content-official".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testE".to_string(),
                category: Some("content-fanmade".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testF".to_string(),
                category: Some("content-fanmade".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testG".to_string(),
                category: Some("species".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testH".to_string(),
                category: Some("species".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testI".to_string(),
                category: Some("oc".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testJ".to_string(),
                category: Some("oc".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testK".to_string(),
                category: Some("character".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testL".to_string(),
                category: Some("character".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testM".to_string(),
                category: Some("origin".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testN".to_string(),
                category: Some("origin".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testO".to_string(),
                category: Some("rating".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testP".to_string(),
                category: Some("rating".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testQ".to_string(),
                category: Some("error".to_string()),
                ..Default::default()
            },
            Tag {
                name: "testR".to_string(),
                category: Some("error".to_string()),
                ..Default::default()
            },
            Tag {
                name: "test0".to_string(),
                category: None,
                ..Default::default()
            },
            Tag {
                name: "test1".to_string(),
                category: None,
                ..Default::default()
            },
        ];
        let out = Tag::create_cache_tagline(&tag_list);
        assert_eq!("testQ, testR, testO, testP, testM, testN, testK, testL, testI, testJ, testG, testH, testE, testF, testC, testD, testA, testB, test0, test1", out, "Cache Tagline Wrong");
        Ok(())
    }
}
