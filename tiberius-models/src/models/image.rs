use std::{
    fmt::Display,
    fs::File,
    io::BufReader,
    ops::{DerefMut, Range},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};

use async_std::{prelude::*, sync::RwLock};
use async_trait::async_trait;
use axum_extra::routing::TypedPath;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use futures::TryStreamExt;
use itertools::Itertools;
use sqlx::{
    postgres::PgRow, query, query_as, types::ipnetwork::IpNetwork, Executor, FromRow, PgPool,
};
use tantivy::{
    collector::{Collector, TopDocs},
    Document, IndexWriter,
};
use tiberius_dependencies::http::{
    uri::{Authority, Scheme},
    Uri,
};
use tiberius_search::{Queryable, SortIndicator};
use tracing::trace;

use crate::{
    comment::Comment,
    pluggables::{
        Hashable, ImageFileMetadata, ImageInteractionMetadata, ImageUrls, Intensities,
        Representations,
    },
    tantivy_bool_text_field, tantivy_date_field, tantivy_raw_text_field, tantivy_text_field,
    tantivy_u64_field, Client, DirectSafeSerialize, ImageFeature, ImageTag, PhilomenaModelError,
    SafeSerialize, SortDirection, Tag, TagLike, TagView,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ImageID(u64);

impl Into<i64> for ImageID {
    fn into(self) -> i64 {
        self.0 as i64
    }
}

impl Into<u64> for ImageID {
    fn into(self) -> u64 {
        self.0 as u64
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct APIImage {
    pub id: u64,
    #[serde(flatten)]
    pub hash: Hashable,
    #[serde(flatten)]
    pub urls: Option<ImageUrls>,
    #[serde(flatten)]
    pub image_file_metadata: Option<ImageFileMetadata>,
    #[serde(flatten)]
    pub image_interaction_metadata: Option<ImageInteractionMetadata>,
    pub first_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: Option<String>,
    pub uploader_id: Option<u64>,
    pub uploader: Option<String>,
    pub description: String,
}

impl DirectSafeSerialize for APIImage {}

impl SafeSerialize for Image {
    type Target = APIImage;

    fn into_safe(&self) -> Self::Target {
        APIImage {
            id: self.id as u64,
            hash: Hashable {
                sha512_hash: self.image_sha512_hash.clone(),
                orig_sha512_hash: self.image_orig_sha512_hash.clone(),
            },
            urls: None,
            image_file_metadata: None,
            image_interaction_metadata: None,
            first_seen_at: DateTime::from_utc(self.first_seen_at, Utc),
            created_at: DateTime::from_utc(self.created_at, Utc),
            updated_at: DateTime::from_utc(self.updated_at, Utc),
            name: self.image_name.clone(),
            uploader_id: self.user_id.map(|x| x as u64),
            uploader: None,
            description: self.description.clone(),
        }
    }
}

#[derive(sqlx::FromRow, Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct Image {
    pub id: i32,
    pub image: Option<String>,
    pub image_name: Option<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub image_size: Option<i32>,
    pub image_format: Option<String>,
    pub image_mime_type: Option<String>,
    pub image_aspect_ratio: Option<f64>,
    pub ip: Option<IpNetwork>,
    pub fingerprint: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub anonymous: Option<bool>,
    pub score: i32,
    pub faves_count: i32,
    pub upvotes_count: i32,
    pub downvotes_count: i32,
    pub votes_count: i32,
    pub watcher_ids: Vec<i32>,
    pub watcher_count: i32,
    pub source_url: Option<String>,
    pub description: String,
    pub image_sha512_hash: Option<String>,
    pub image_orig_sha512_hash: Option<String>,
    pub deletion_reason: Option<String>,
    pub tag_list_cache: Option<String>,
    pub tag_list_plus_alias_cache: Option<String>,
    pub file_name_cache: Option<String>,
    pub duplicate_id: Option<i32>,
    pub tag_ids: Vec<i32>,
    pub comments_count: i32,
    pub processed: bool,
    pub thumbnails_generated: bool,
    pub duplication_checked: bool,
    pub hidden_from_users: bool,
    pub tag_editing_allowed: bool,
    pub description_editing_allowed: bool,
    pub commenting_allowed: bool,
    pub is_animated: bool,
    pub first_seen_at: NaiveDateTime,
    pub featured_on: Option<NaiveDateTime>,
    pub se_intensity: Option<f64>,
    pub sw_intensity: Option<f64>,
    pub ne_intensity: Option<f64>,
    pub nw_intensity: Option<f64>,
    pub average_intensity: Option<f64>,
    pub user_id: Option<i32>,
    pub deleted_by_id: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub destroyed_content: bool,
    pub hidden_image_key: Option<String>,
    pub scratchpad: Option<String>,
    pub hides_count: i32,
    pub image_duration: Option<f64>,
}

impl Default for Image {
    fn default() -> Self {
        Image {
            id: 0,
            image: None,
            image_name: None,
            image_width: None,
            image_height: None,
            image_size: None,
            image_format: None,
            image_mime_type: None,
            image_aspect_ratio: None,
            ip: None,
            fingerprint: None,
            user_agent: None,
            referrer: None,
            anonymous: None,
            score: 0,
            faves_count: 0,
            upvotes_count: 0,
            downvotes_count: 0,
            votes_count: 0,
            watcher_ids: vec![],
            watcher_count: 0,
            source_url: None,
            description: "".to_string(),
            image_sha512_hash: None,
            image_orig_sha512_hash: None,
            deletion_reason: None,
            tag_list_cache: None,
            tag_list_plus_alias_cache: None,
            file_name_cache: None,
            duplicate_id: None,
            tag_ids: vec![],
            comments_count: 0,
            processed: false,
            thumbnails_generated: false,
            duplication_checked: false,
            hidden_from_users: true,
            tag_editing_allowed: false,
            description_editing_allowed: false,
            commenting_allowed: false,
            is_animated: false,
            first_seen_at: Utc::now().naive_utc(),
            featured_on: None,
            se_intensity: None,
            sw_intensity: None,
            ne_intensity: None,
            nw_intensity: None,
            average_intensity: None,
            user_id: None,
            deleted_by_id: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            destroyed_content: false,
            hidden_image_key: None,
            scratchpad: None,
            hides_count: 0,
            image_duration: None,
        }
    }
}

pub struct ImageWithTags {
    pub image: Image,
    pub tags: Vec<Tag>,
}

#[derive(sqlx::FromRow, Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ImageMeta {
    pub id: i32,
    pub views: i64,
}

impl ImageMeta {
    pub fn views_to_text(s: Option<ImageMeta>) -> String {
        match s {
            Some(ImageMeta { views, .. }) => match views {
                0 => "0".to_string(),
                1 => "1".to_string(),
                v => format!("{v}"),
            },
            None => "0".to_string(),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ImageTagView {
    id: i32,
    tag_ids: Vec<i32>,
    tag_names: Vec<String>,
    tag_namespaces: Vec<Option<String>>,
    tag_name_in_namespaces: Vec<Option<String>>,
    tag_categories: Vec<Option<String>>,
    tag_slugs: Vec<Option<String>>,
    tag_descriptions: Vec<Option<String>>,
    tag_images_counts: Vec<i32>,
}

impl ImageTagView {
    pub fn get_tags(&self) -> Vec<TagView> {
        self.tag_ids
            .iter()
            .zip(self.tag_names.iter())
            .zip(self.tag_namespaces.iter())
            .zip(self.tag_name_in_namespaces.iter())
            .zip(self.tag_categories.iter())
            .zip(self.tag_slugs.iter())
            .zip(self.tag_descriptions.iter())
            .zip(self.tag_images_counts.iter())
            .map(
                |(
                    (
                        (
                            (
                                (((id, tag_name), tag_namespace), tag_name_in_namespace),
                                tag_category,
                            ),
                            tag_slug,
                        ),
                        tag_description,
                    ),
                    tag_images_count,
                )| {
                    TagView {
                        id: *id as u64,
                        name: tag_name.clone(),
                        namespace: tag_namespace.clone(),
                        name_in_namespace: tag_name_in_namespace.clone(),
                        category: tag_category.clone(),
                        slug: tag_slug.clone(),
                        description: tag_description.clone(),
                        images_count: tag_images_count.clone(),
                    }
                },
            )
            .collect_vec()
    }
}

impl Image {
    pub fn id(&self) -> ImageID {
        ImageID(self.id as u64)
    }
    pub async fn get_quick_tags(
        &self,
        client: &mut Client,
    ) -> Result<Option<ImageTagView>, PhilomenaModelError> {
        #[derive(sqlx::FromRow, Clone, Debug)]
        struct ImageTagViewInternal {
            id: Option<i32>,
            tag_ids: Option<Vec<i32>>,
            tag_names: Option<Vec<String>>,
            tag_namespaces: Option<Vec<Option<String>>>,
            tag_name_in_namespaces: Option<Vec<Option<String>>>,
            tag_categories: Option<Vec<Option<String>>>,
            tag_slugs: Option<Vec<Option<String>>>,
            tag_descriptions: Option<Vec<Option<String>>>,
            tag_images_counts: Option<Vec<i32>>,
        }
        let r: Option<ImageTagViewInternal> =
            sqlx::query_as("SELECT * FROM image_tags WHERE id = $1")
                .bind(self.id)
                .fetch_optional(client)
                .await?;
        let r = match r {
            Some(v) => v,
            None => return Ok(None),
        };
        let r = ImageTagView {
            id: match r.id {
                None => return Ok(None),
                Some(v) => v,
            },
            tag_ids: r.tag_ids.unwrap_or_default(),
            tag_names: r.tag_names.unwrap_or_default(),
            tag_namespaces: r.tag_namespaces.unwrap_or_default(),
            tag_name_in_namespaces: r.tag_name_in_namespaces.unwrap_or_default(),
            tag_categories: r.tag_categories.unwrap_or_default(),
            tag_slugs: r.tag_slugs.unwrap_or_default(),
            tag_descriptions: r.tag_descriptions.unwrap_or_default(),
            tag_images_counts: r.tag_images_counts.unwrap_or_default(),
        };
        Ok(Some(r))
    }
    pub async fn upload(self, client: &mut Client) -> Result<Image, PhilomenaModelError> {
        #[cfg(not(test))]
        assert!(self.id == 0, "New images must have ID == 0");
        // Some sanity asserts here, then store in DB
        // TODO: implement this properly???
        self.insert_new(client).await
    }
    pub fn filename(&self) -> String {
        format!(
            "{}.{}",
            self.id,
            self.image_format.as_ref().unwrap_or(&"png".to_string())
        )
    }

    async fn update_fnc(
        &mut self,
        fnc: &String,
        client: &mut Client,
    ) -> Result<(), PhilomenaModelError> {
        self.file_name_cache = Some(fnc.clone());
        sqlx::query!(
            "UPDATE images SET file_name_cache = $1 WHERE id = $2",
            fnc,
            self.id
        )
        .execute(client)
        .await?;
        Ok(())
    }

    pub async fn long_filename(
        &mut self,
        client: &mut Client,
    ) -> Result<String, PhilomenaModelError> {
        Ok(match None::<String> {
            Some(fnc) => format!(
                "{fnc}.{}",
                self.image_format.as_ref().unwrap_or(&"png".to_string())
            ),
            None => {
                let id = self.id.to_string();
                let tag_line = self.tags(client).await?;
                let mut tag_line = tag_line
                    .iter()
                    .sorted()
                    .map(|x| x.path_full_name())
                    .join("_");
                tag_line.truncate(151);
                let res = format!(
                    "{id}__{tag_line}.{}",
                    self.image_format.as_ref().unwrap_or(&"png".to_string())
                );
                self.update_fnc(&res, client).await?;
                res
            }
        })
    }

    pub fn filetypef<S: Display>(&self, s: S) -> String {
        format!(
            "{}.{}",
            s,
            self.image_format.as_ref().unwrap_or(&"png".to_string())
        )
    }
    pub async fn tags(&self, client: &mut Client) -> Result<Vec<Tag>, PhilomenaModelError> {
        let cta = client.cache_tag_assoc.clone();
        Ok(cta.get_or_try_insert_with(
            self.id(),
            query_as!(
                crate::Tag,
                //"SELECT * FROM tags WHERE id IN (SELECT tag_id FROM image_taggings WHERE image_id = $1)",
                // join is more optimal tbh
                "SELECT t.* FROM tags t JOIN image_taggings it ON it.tag_id = t.id WHERE it.image_id = $1",
                self.id as i64,
            )
            .fetch_all(client)
        ).await?)
    }

    #[cfg(test)]
    pub async fn add_tag<S: AsRef<str>>(
        &self,
        tag: S,
        client: &mut Client,
    ) -> Result<(), PhilomenaModelError> {
        let tag = Tag::create_for_test(client, tag).await?;
        sqlx::query!(
            "INSERT INTO image_taggings (image_id, tag_id) VALUES ($1, $2)",
            self.id as i64,
            tag.id as i64
        )
        .execute(client)
        .await?;
        Ok(())
    }

    pub async fn increment_views(&self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        query!(
            "INSERT INTO images_metadata (id, views) VALUES ($1, 0)
                ON CONFLICT (id) DO NOTHING",
            self.id
        )
        .execute(&mut *client)
        .await?;
        query!(
            "UPDATE images_metadata SET views = views + 1 WHERE id = $1",
            self.id
        )
        .execute(&mut *client)
        .await?;
        Ok(())
    }

    pub async fn metadata(
        &self,
        client: &mut Client,
    ) -> Result<Option<ImageMeta>, PhilomenaModelError> {
        Ok(query_as!(
            ImageMeta,
            "SELECT * FROM images_metadata WHERE id = $1",
            self.id,
        )
        .fetch_optional(client)
        .await?)
    }

    /// Updates the row caches of the image
    /// and returns true if the image needs to be updated in the database
    pub async fn update_cache_lines(
        &mut self,
        client: &mut Client,
    ) -> Result<bool, PhilomenaModelError> {
        let tags = self.tags(client).await?;
        let tag_cacheline = Tag::create_cache_tagline(&tags);
        if self.tag_list_cache.as_ref() != Some(&tag_cacheline) {
            debug!(
                "Updating tag cachline on image {} with {} from {:?}",
                self.id, tag_cacheline, self.tag_list_cache
            );
            self.tag_list_cache = Some(tag_cacheline);
            Ok(true)
        } else {
            debug!("Tag Cacheline Update requested but not stale");
            Ok(false)
        }
    }

    pub async fn mark_processed(&self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        query_as!(
            ImageMeta,
            "UPDATE images SET processed = true WHERE id = $1",
            self.id,
        )
        .execute(client)
        .await?;
        Ok(())
    }

    pub async fn mark_thumbnails_generated(&self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        query_as!(
            ImageMeta,
            "UPDATE images SET thumbnails_generated = true WHERE id = $1",
            self.id,
        )
        .execute(client)
        .await?;
        Ok(())
    }

    pub async fn openf(&self, base_dir: &PathBuf) -> Result<BufReader<File>, PhilomenaModelError> {
        let path = self.pathf(base_dir).await?;
        Ok(BufReader::new(File::open(path)?))
    }
    pub async fn statf(
        &self,
        base_dir: &PathBuf,
    ) -> Result<std::fs::Metadata, PhilomenaModelError> {
        let path = self.pathf(base_dir).await?;
        Ok(async_std::fs::metadata(path).await?)
    }
    pub async fn pathf(&self, base_dir: &PathBuf) -> Result<PathBuf, PhilomenaModelError> {
        let image = match &self.image {
            None => {
                return Err(PhilomenaModelError::DataWasNull {
                    column: "image".to_string(),
                    table: "images".to_string(),
                    id: self.id.to_string(),
                })
            }
            Some(i) => i,
        };
        Ok(base_dir.clone().join("images").join(image))
    }
    #[cfg(test)]
    pub async fn new_test_image(client: &mut Client) -> Result<Self, PhilomenaModelError> {
        let image = Image {
            id: 0x5EADBEEFi32,
            image: Some("./res/test-assets/test-image.png".to_string()),
            image_name: Some("test-image.png".to_string()),
            image_height: Some(1980),
            image_width: Some(1238),
            ..Default::default()
        };
        Ok(image.upload(client).await?)
    }

    #[cfg(test)]
    pub async fn new_test_image_from_disk<P: AsRef<std::path::Path>>(
        client: &mut Client,
        id: i32,
        timestamp: u64,
        pid: u64,
        path: P,
    ) -> Result<Self, PhilomenaModelError> {
        use std::path::Path;

        let path: &Path = path.as_ref();
        let timestamp: DateTime<Utc> = (std::time::UNIX_EPOCH + std::time::Duration::from_micros(timestamp))
            .into();
        let true_path = format!("{}{:09}.{}", timestamp.timestamp_micros(), pid, path.extension().unwrap().to_str().unwrap());
        let true_path = format!("{}/{}/{}/{true_path}", timestamp.year(), timestamp.month(), timestamp.day());
        let image = Image {
            id,
            created_at: timestamp.naive_utc(),
            image: Some(true_path),
            image_name: Some(path.file_name().unwrap().to_string_lossy().to_string()),
            image_height: Some(20000),
            image_width: Some(1500),
            image_format: Some(path.extension().unwrap().to_string_lossy().to_string()),
            ..Default::default()
        };
        let mut img = image.upload(client).await?;
        img.id = id;
        Ok(img)
    }

    pub async fn save(mut self, client: &mut Client) -> Result<Self, PhilomenaModelError> {
        #[derive(sqlx::FromRow)]
        struct Returning {
            id: i32,
        }
        let id = query_as!(
            Returning,
            "UPDATE images SET 
                image = $2, image_name = $3, image_width = $4, image_height = $5,
                image_size = $6, image_format = $7, image_mime_type = $8, image_aspect_ratio = $9,
                ip = $10, fingerprint = $11, user_agent = $12, referrer = $13,
                anonymous = $14, score = $15, faves_count = $16, upvotes_count = $17,
                downvotes_count = $18, watcher_ids = $19, watcher_count = $20, source_url = $21,
                description = $22, image_sha512_hash = $23, image_orig_sha512_hash = $24, deletion_reason = $25,
                file_name_cache = $26, duplicate_id = $27,
                comments_count = $28, processed = $29, thumbnails_generated = $30,
                duplication_checked = $31, hidden_from_users = $32, tag_editing_allowed = $33, description_editing_allowed = $34,
                commenting_allowed = $35, is_animated = $36, first_seen_at = $37, featured_on = $38,
                se_intensity = $39, sw_intensity = $40, ne_intensity = $41, nw_intensity = $42,
                average_intensity = $43, user_id = $44, deleted_by_id = $45, created_at = $46,
                updated_at = $47, destroyed_content = $48, hidden_image_key = $49, scratchpad = $50,
                hides_count = $51, image_duration = $52
            WHERE id = $1
            RETURNING id",
            self.id,
            self.image,
            self.image_name,
            self.image_width,
            self.image_height,
            self.image_size,
            self.image_format,
            self.image_mime_type,
            self.image_aspect_ratio,
            self.ip,
            self.fingerprint,
            self.user_agent,
            self.referrer,
            self.anonymous,
            self.score,
            self.faves_count,
            self.upvotes_count,
            self.downvotes_count,
            &self.watcher_ids,
            self.watcher_count,
            self.source_url,
            self.description,
            self.image_sha512_hash,
            self.image_orig_sha512_hash,
            self.deletion_reason,
            self.file_name_cache,
            self.duplicate_id,
            self.comments_count,
            self.processed,
            self.thumbnails_generated,
            self.duplication_checked,
            self.hidden_from_users,
            self.tag_editing_allowed,
            self.description_editing_allowed,
            self.commenting_allowed,
            self.is_animated,
            self.first_seen_at,
            self.featured_on,
            self.se_intensity,
            self.sw_intensity,
            self.ne_intensity,
            self.nw_intensity,
            self.average_intensity,
            self.user_id,
            self.deleted_by_id,
            self.created_at,
            self.updated_at,
            self.destroyed_content,
            self.hidden_image_key,
            self.scratchpad,
            self.hides_count,
            self.image_duration,
        )
        .fetch_one(&mut client.clone())
        .await?;
        self.id = id.id;
        for tag in self.tag_ids {
            sqlx::query!(
                "
                INSERT INTO image_taggings (image_id, tag_id) VALUES ($1, $2)
                ON CONFLICT DO NOTHING
            ",
                self.id as i64,
                tag as i64
            )
            .execute(&mut client.clone())
            .await?;
        }
        Ok(Image::get(client, self.id as i64)
            .await?
            .expect("we just uploaded this"))
    }

    pub async fn source_change_count(&self) -> u64 {
        // TODO: determine actual source change count
        1
    }

    pub async fn thumbnail_path(
        &self,
        thumb_type: ImageThumbType,
    ) -> Result<PathBuf, PhilomenaModelError> {
        let mut thumb_path = self.thumbnail_basepath().await?;
        thumb_path.push(format!(
            "{}.{}",
            thumb_type.to_string(),
            self.image_format
                .as_ref()
                .expect("no image format when processing thumbs")
        ));
        Ok(thumb_path)
    }
    pub async fn thumbnail_basepath(&self) -> Result<PathBuf, PhilomenaModelError> {
        //TODO: add auxiliary table for keeping track of image thumbnails
        let mut img_path: PathBuf = self
            .image
            .as_ref()
            .map(PathBuf::from)
            .expect("image has not path => no thumbs either");
        assert!(
            img_path.pop(),
            "must not be top level path when truncating original file name"
        );
        let thumb_path = PathBuf::from("images/thumbs/")
            .join(img_path)
            .join(self.id.to_string());
        Ok(thumb_path)
    }
    pub async fn insert_new(mut self, client: &mut Client) -> Result<Self, PhilomenaModelError> {
        #[derive(sqlx::FromRow)]
        struct Returning {
            id: i32,
        }
        let id = query_as!(
            Returning,
            "INSERT INTO images (
                image, image_name, image_width, image_height, 
                image_size, image_format, image_mime_type, ip,
                fingerprint, user_agent, referrer, anonymous,
                source_url, description, tag_ids, is_animated,
                created_at, updated_at, first_seen_at
             ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7, $8,
                $9, $10, $11, $12,
                $13, $14, $15, $16,
                $17, $18, $19
            ) RETURNING id",
            self.image,
            self.image_name,
            self.image_width,
            self.image_height,
            self.image_size,
            self.image_format,
            self.image_mime_type,
            self.ip,
            self.fingerprint,
            self.user_agent,
            self.referrer,
            self.anonymous,
            self.source_url,
            self.description,
            &self.tag_ids,
            self.is_animated,
            self.created_at,
            self.updated_at,
            self.first_seen_at,
        )
        .fetch_one(&mut client.clone())
        .await?;
        self.id = id.id;
        for tag in self.tag_ids {
            sqlx::query!(
                "
                INSERT INTO image_taggings (image_id, tag_id) VALUES ($1, $2)
            ",
                self.id as i64,
                tag as i64
            )
            .execute(&mut client.clone())
            .await?;
        }
        Ok(Image::get(client, self.id as i64)
            .await?
            .expect("we just uploaded this"))
    }
    pub async fn tags_text(&self, client: &mut Client) -> Result<String, PhilomenaModelError> {
        let res: Vec<Tag> = self.tags(client).await?;
        let res = res.into_iter().map(|x| x.full_name()).join(", ");
        Ok(res)
    }
    pub async fn get_tag_ids(
        &self,
        client: &mut Client,
    ) -> Result<Vec<ImageTag>, PhilomenaModelError> {
        Ok(query_as!(
            crate::ImageTag,
            "SELECT * FROM image_taggings WHERE image_id = $1",
            self.id as i64,
        )
        .fetch_all(client)
        .await?)
    }
    #[instrument(skip(client))]
    pub async fn random(client: &mut Client) -> Result<Self, PhilomenaModelError> {
        Ok(
            query_as!(Image, "SELECT * FROM images ORDER BY random() LIMIT 1")
                .fetch_one(client)
                .await?,
        )
    }

    pub async fn get_next_from(
        client: &mut Client,
        id: i64,
    ) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(query_as!(
            Image,
            "SELECT * FROM images WHERE id > $1 ORDER BY id LIMIT 1",
            id as i32
        )
        .fetch_optional(client)
        .await?)
    }

    pub async fn get_previous_from(
        client: &mut Client,
        id: i64,
    ) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(query_as!(
            Image,
            "SELECT * FROM images WHERE id < $1 ORDER BY id DESC LIMIT 1",
            id as i32
        )
        .fetch_optional(client)
        .await?)
    }

    #[instrument(skip(client))]
    pub async fn get(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(
            query_as!(Image, "SELECT * FROM images WHERE id = $1", (id as i32),)
                .fetch_optional(client)
                .await?,
        )
    }
    #[instrument(skip(client))]
    pub async fn count(
        client: &mut Client,
        start_id: Option<u64>,
        end_id: Option<u64>,
    ) -> Result<u64, PhilomenaModelError> {
        struct Count {
            cnt: Option<i64>,
        }
        match (start_id, end_id) {
            (Some(start_id), Some(end_id)) => Ok(query_as!(
                Count,
                "SELECT COUNT(*) AS cnt FROM images WHERE id BETWEEN $1 AND $2",
                start_id as i64,
                end_id as i64
            )
            .fetch_one(client)
            .await?
            .cnt
            .unwrap_or_default() as u64),
            (Some(start_id), None) => Ok(query_as!(
                Count,
                "SELECT COUNT(*) AS cnt FROM images WHERE id > $1",
                start_id as i64
            )
            .fetch_one(client)
            .await?
            .cnt
            .unwrap_or_default() as u64),
            (None, Some(end_id)) => Ok(query_as!(
                Count,
                "SELECT COUNT(*) AS cnt FROM images WHERE id <= $1",
                end_id as i64
            )
            .fetch_one(client)
            .await?
            .cnt
            .unwrap_or_default() as u64),
            (None, None) => Ok(query_as!(Count, "SELECT COUNT(*) AS cnt FROM images")
                .fetch_one(client)
                .await?
                .cnt
                .unwrap_or_default() as u64),
        }
    }
    #[instrument(skip(client))]
    pub async fn get_many(
        client: &mut Client,
        ids: Vec<i64>,
        sort_by: ImageSortBy,
    ) -> Result<Vec<Self>, PhilomenaModelError> {
        let ids: Vec<i32> = ids.iter().map(|x| *x as i32).collect();
        if sort_by.random() {
            Ok(query_as!(
                Image,
                "SELECT * FROM images WHERE id = ANY($1) LIMIT 100",
                &ids,
            )
            .fetch_all(client)
            .await?)
        } else if sort_by.invert_sort() {
            Ok(query_as!(
                Image,
                "SELECT * FROM images WHERE id = ANY($1) ORDER BY $2 ASC LIMIT 100",
                &ids,
                sort_by.to_sql(),
            )
            .fetch_all(client)
            .await?)
        } else {
            Ok(query_as!(
                Image,
                "SELECT * FROM images WHERE id = ANY($1) ORDER BY $2 DESC LIMIT 100",
                &ids,
                sort_by.to_sql(),
            )
            .fetch_all(client)
            .await?)
        }
    }
    #[instrument(skip(client))]
    pub async fn get_range(
        client: &mut Client,
        range: Range<u64>,
    ) -> Result<Vec<Self>, PhilomenaModelError> {
        Ok(query_as!(
            Image,
            "SELECT * FROM images WHERE id BETWEEN $1 and $2 LIMIT 100",
            range.start as i32,
            range.end as i32
        )
        .fetch_all(client)
        .await?)
    }
    #[instrument(skip(client))]
    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(query_as!(
            Image,
            "SELECT * FROM images WHERE id = $1 LIMIT 1",
            id as i32
        )
        .fetch_optional(client)
        .await?)
    }
    pub async fn get_newest(client: &mut Client) -> Result<Option<Self>, PhilomenaModelError> {
        let id = query!("SELECT id FROM images ORDER BY created_at DESC LIMIT 1",)
            .fetch_one(&mut *client)
            .await?
            .id;
        Self::get_id(client, id as i64).await
    }
    pub async fn get_all(
        pool: PgPool,
        start_id: Option<u64>,
        end_id: Option<u64>,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PgRow, sqlx::Error>>>>, PhilomenaModelError>
    {
        match (start_id, end_id) {
            (Some(start_id), Some(end_id)) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM images WHERE id BETWEEN $1 AND $2 ORDER BY id",
                start_id as i64,
                end_id as i64
            ))),
            (Some(start_id), None) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM images WHERE id > $1 ORDER BY id",
                start_id as i64
            ))),
            (None, Some(end_id)) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM images WHERE id <= $1 ORDER BY id",
                end_id as i64
            ))),
            (None, None) => Ok(pool.fetch(sqlx::query!("SELECT * FROM images ORDER BY id"))),
        }
    }

    pub async fn get_featured(client: &mut Client) -> Result<Option<Self>, PhilomenaModelError> {
        let feature: Option<ImageFeature> = sqlx::query_as!(
            ImageFeature,
            "SELECT * FROM image_features ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&mut client.clone())
        .await?;
        if let Some(feature) = feature {
            Ok(Self::get_id(client, feature.image_id).await?)
        } else {
            Ok(None)
        }
    }
    #[instrument(skip(client))]
    pub async fn search<
        S1: Into<String> + std::fmt::Debug,
        S4: Into<String> + std::fmt::Debug,
        S5: Into<String> + std::fmt::Debug,
    >(
        client: &mut Client,
        query: S1,
        aqueries: Vec<S4>,
        anqueries: Vec<S5>,
        sort_by: ImageSortBy,
        page: u64,
        page_size: u64,
    ) -> Result<(u64, Vec<Self>), PhilomenaModelError> {
        let query: String = query.into();
        let i: tiberius_search::tantivy::IndexReader = client.index_reader::<Image>()?;
        let ids = Image::search_item_with_str(
            &i,
            &query,
            aqueries,
            anqueries,
            page_size as usize,
            (page * page_size) as usize,
            sort_by,
        );
        let (total, ids): (usize, Vec<i64>) = match ids {
            Ok((total, v)) => (total, v.iter().map(|x| x.1 as i64).collect()),
            Err(e) => return Err(PhilomenaModelError::Searcher(e)),
        };
        Ok((total as u64, Self::get_many(client, ids, sort_by).await?))
    }
    pub fn hidden(&self, _client: &mut Client) -> Result<bool, PhilomenaModelError> {
        //TODO: check if hidden properly, trust staff for now
        Ok(false)
    }
    pub async fn filter_or_spoiler_hits(
        &self,
        _client: &mut Client,
    ) -> Result<bool, PhilomenaModelError> {
        //TODO: check if filtered/spoilered
        Ok(false)
    }
    pub async fn title_text(&self, client: &mut Client) -> Result<String, PhilomenaModelError> {
        let tags = self.tags(client).await?;
        let tags: Vec<String> = tags.iter().map(|x| x.full_name()).collect();
        trace!("Got tags for image: {:?}", tags);
        let tags = tags.join(", ");

        Ok(format!(
            "Size: {}x{} | Tagged: {}",
            self.image_width.unwrap_or_default(),
            self.image_height.unwrap_or_default(),
            tags
        ))
    }

    pub async fn comments(&self, client: &mut Client) -> Result<Vec<Comment>, PhilomenaModelError> {
        Ok(query_as!(
            Comment,
            "SELECT * FROM comments WHERE image_id = $1 ORDER BY created_at DESC",
            self.id
        )
        .fetch_all(client)
        .await?)
    }

    pub async fn image_thumb_urls(&self) -> Result<ImageThumbUrl, PhilomenaModelError> {
        Ok(ImageThumbUrl {
            full: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageGetFull {
                            filename: self.filename(),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            full_thumbnail: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("full"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            large: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("large"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            rendered: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("rendered"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            tall: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("tall"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            medium: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("medium"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            small: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("small"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            thumb: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("thumb"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            thumb_small: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("thumb_small"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
            thumb_tiny: Uri::builder()
                .path_and_query(
                    PathBuf::from(
                        PathImageThumbGet {
                            id: self.id as u64,
                            filename: self.filetypef("thumb_tiny"),
                            year: self.created_at.year() as u16,
                            month: self.created_at.month() as u8,
                            day: self.created_at.day() as u8,
                        }
                        .to_uri()
                        .to_string(),
                    )
                    .to_string_lossy()
                    .to_string(),
                )
                .build()
                .unwrap(),
        })
    }

    pub async fn storage_path(&self) -> Result<Option<PathBuf>, PhilomenaModelError> {
        match self.image {
            Some(ref image) => Ok(Some(PathBuf::from(image))),
            None => Ok(None),
        }
    }
}

impl ImageSortBy {
    pub const fn to_sql(&self) -> &'static str {
        match self {
            ImageSortBy::Random => "",
            ImageSortBy::ID(_) => "id",
            ImageSortBy::CreatedAt(_) => "created_at",
            ImageSortBy::Score(_) => "score",
            // TODO: compute wilson score and allow sorting by
            ImageSortBy::WilsonScore(_) => "score",
        }
    }
}

impl SortIndicator for ImageSortBy {
    fn field(&self) -> &'static str {
        match self {
            ImageSortBy::Random => "id",
            ImageSortBy::ID(_) => "id",
            ImageSortBy::CreatedAt(_) => "created_at_ts",
            ImageSortBy::Score(_) => "score",
            // TODO: compute wilson score and allow sorting by
            ImageSortBy::WilsonScore(_) => "score",
        }
    }

    fn invert_sort(&self) -> bool {
        let dir = match self {
            ImageSortBy::Random => return false,
            ImageSortBy::ID(dir) => dir,
            ImageSortBy::CreatedAt(dir) => dir,
            ImageSortBy::Score(dir) => dir,
            ImageSortBy::WilsonScore(dir) => dir,
        };
        match dir {
            SortDirection::Ascending => true,
            SortDirection::Descending => false,
        }
    }

    fn random(&self) -> bool {
        match self {
            ImageSortBy::Random => true,
            _ => false,
        }
    }
}

#[async_trait::async_trait]
impl Queryable for Image {
    type Group = String;
    type DBClient = Client;
    type IndexError = PhilomenaModelError;
    type SortIndicator = ImageSortBy;

    fn identifier(&self) -> u64 {
        self.id as u64
    }

    fn group() -> Self::Group {
        "images".to_string()
    }

    fn schema() -> tantivy::schema::Schema {
        use schema::*;
        use tantivy::*;
        let mut builder = Schema::builder();
        tantivy_date_field!(builder, created_at);
        tantivy_u64_field!(builder, id);
        tantivy_u64_field!(builder, score);
        tantivy_raw_text_field!(builder, tag);
        tantivy_text_field!(builder, description);
        tantivy_bool_text_field!(builder, processed);
        tantivy_bool_text_field!(builder, deleted);
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
        doc.add_date(
            schema.get_field("created_at").unwrap(),
            tantivy::DateTime::from_timestamp_secs(
                chrono::DateTime::<chrono::Utc>::from_utc(self.created_at, chrono::Utc).timestamp(),
            ),
        );
        doc.add_u64(
            schema.get_field("created_at_ts").unwrap(),
            chrono::DateTime::<chrono::Utc>::from_utc(self.created_at, chrono::Utc).timestamp()
                as u64,
        );
        doc.add_u64(schema.get_field("id").unwrap(), self.id as u64);
        if !omit_index_only {
            doc.add_text(schema.get_field("description").unwrap(), &self.description);
        }
        doc.add_text(
            schema.get_field("processed").unwrap(),
            self.processed.to_string(),
        );
        doc.add_text(
            schema.get_field("deleted").unwrap(),
            self.deleted_by_id.is_some().to_string(),
        );
        let tag_field = schema.get_field("tag").unwrap();
        for tag in self.tags(client).await? {
            doc.add_text(tag_field, tag.full_name());
        }
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
        writer.write().await.delete_term(Term::from_field_u64(
            Self::schema().get_field("id").unwrap(),
            self.id as u64,
        ));
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ImageThumbType {
    Rendered,
    Full,
    Tall,
    Large,
    Medium,
    Small,
    Thumb,
    ThumbSmall,
    ThumbTiny,
}

#[derive(serde::Serialize)]
pub struct ImageThumbUrl {
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub rendered: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub full: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub full_thumbnail: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub tall: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub large: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub medium: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub small: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub thumb: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub thumb_small: Uri,
    #[serde(with = "tiberius_dependencies::http_serde::uri")]
    pub thumb_tiny: Uri,
}

impl ImageThumbUrl {
    pub fn with_host(self, host: Option<String>) -> Self {
        let host = match host {
            None => return self,
            Some(host) => host,
        };
        let base = Uri::try_from(host).unwrap();
        let host = base.authority().unwrap();
        let scheme = base.scheme().unwrap();
        Self {
            rendered: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.rendered.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            full: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.full.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            full_thumbnail: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.full_thumbnail.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            tall: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.tall.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            large: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.large.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            medium: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.medium.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            small: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.small.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            thumb: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.thumb.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            thumb_small: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.thumb_small.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
            thumb_tiny: {
                Uri::builder()
                    .scheme(scheme.clone())
                    .authority(host.clone())
                    .path_and_query(self.thumb_tiny.path_and_query().unwrap().clone())
                    .build()
                    .expect("was already valid URI")
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSortBy {
    Random,
    ID(SortDirection),
    CreatedAt(SortDirection),
    Score(SortDirection),
    WilsonScore(SortDirection),
}

#[derive(Clone, Copy, Debug)]
pub struct ResolutionLimit {
    pub height: u32,
    pub width: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Resolution {
    pub height: u32,
    pub width: u32,
}

impl From<(u32, u32)> for ResolutionLimit {
    fn from(f: (u32, u32)) -> Self {
        Self {
            width: f.1,
            height: f.0,
        }
    }
}

impl ResolutionLimit {
    pub fn new(height: u32, width: u32) -> Self {
        (height, width).into()
    }
    pub fn clamp_resolution(&self, height: u32, width: u32) -> Resolution {
        let aspect = {
            assert!(
                width > 0,
                "Width of image was zero, cannot compute aspect ratio"
            );
            let height = height as f64;
            let width = width as f64;
            height / width
        };
        if height <= self.height && width <= self.width {
            Resolution { height, width }
        } else if aspect < 1.0 {
            let ratio = self.width as f64 / width as f64;
            let nheight = height as f64 * ratio;
            let height = nheight.round() as u32;
            Resolution {
                height,
                width: self.width,
            }
        } else if aspect > 1.0 {
            let ratio = self.height as f64 / height as f64;
            let nwidth = width as f64 * ratio;
            let width = nwidth.round() as u32;
            Resolution {
                height: self.height,
                width,
            }
        } else {
            warn!("WARN: Using full-float resize, this indicates bad aspect compute");
            let aratio = self.height as f64 / height as f64;
            let bratio = self.width as f64 / width as f64;
            let ratio = aratio.min(bratio);
            let nwidth = width as f64 * ratio;
            let nheight = height as f64 * ratio;
            let height = nheight.round() as u32;
            let width = nwidth.round() as u32;
            Resolution { height, width }
        }
    }
}

impl ImageThumbType {
    pub fn to_resolution_limit(self) -> Option<ResolutionLimit> {
        use ImageThumbType::*;
        match self {
            Rendered => ImageThumbType::Full.to_resolution_limit(),
            Full => None,
            Tall => Some((4096, 1024).into()),
            Large => Some((1280, 1024).into()),
            Medium => Some((800, 600).into()),
            Small => Some((320, 240).into()),
            Thumb => Some((250, 250).into()),
            ThumbSmall => Some((150, 150).into()),
            ThumbTiny => Some((50, 50).into()),
        }
    }
}

impl ToString for ImageThumbType {
    fn to_string(&self) -> String {
        use ImageThumbType::*;
        match self {
            Rendered => "rendered",
            Full => "full",
            Tall => "tall",
            Large => "large",
            Medium => "medium",
            Small => "small",
            Thumb => "thumb",
            ThumbSmall => "thumb_small",
            ThumbTiny => "thumb_tiny",
        }
        .to_string()
    }
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/img/:year/:month/:day/:id/:filename")]
pub struct PathImageThumbGet {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub id: u64,
    pub filename: String,
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/img/view/:year/:month/:day/:filename")]
pub struct PathImageGetFull {
    pub filename: String,
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl PathImageGetFull {
    pub async fn from_image(
        i: &mut Image,
        client: &mut Client,
    ) -> Result<Self, PhilomenaModelError> {
        Ok(Self {
            filename: i.long_filename(client).await?,
            year: i.created_at.year() as u16,
            month: i.created_at.month() as u8,
            day: i.created_at.day() as u8,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::TimeZone;

    /// Test Philo Compat based on image 4020561 image file
    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "../migrations"))]
    async fn test_long_filename_ex4020561() -> Result<(), PhilomenaModelError> {
        let mut client = Client::new(pool, None);
        let mut image = Image::new_test_image(&mut client).await?;
        image.add_tag("artist:test_artist", &mut client).await?;

        assert_eq!(
            "1__artist-colon-test_artist.png",
            image.long_filename(&mut client).await?
        );
        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "../migrations"))]
    async fn test_filepath_generation_oldstyle() -> Result<(), PhilomenaModelError> {
        let mut client = Client::new(pool, None);
        let image = Image::new_test_image_from_disk(
            &mut client,
            4025092,
            1666222412_405_906,
            010_826_188,
            "../test_data/very_tall_image_conversion.jpg",
        )
        .await?;

        let image_thumb_url: ImageThumbUrl = image.image_thumb_urls().await?;

        assert_eq!(
            "/img/view/2022/10/19/4025092.jpg",
            image_thumb_url.full.to_string(),
            "Rendered Full-Size Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/large.jpg",
            image_thumb_url.large.to_string(),
            "Large Thumb Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/medium.jpg",
            image_thumb_url.medium.to_string(),
            "Medium Thumb Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/small.jpg",
            image_thumb_url.small.to_string(),
            "Small Thumb Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/tall.jpg",
            image_thumb_url.tall.to_string(),
            "Tall Thumb Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/thumb.jpg",
            image_thumb_url.thumb.to_string(),
            "Thumb Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/thumb_small.jpg",
            image_thumb_url.thumb_small.to_string(),
            "Small Thumb Path"
        );
        assert_eq!(
            "/img/2022/10/19/4025092/thumb_tiny.jpg",
            image_thumb_url.thumb_tiny.to_string(),
            "Tiny Thumb Path"
        );

        assert_eq!(
            Some(PathBuf::from("2022/10/19/1666222412405906010826188.jpg")),
            image.storage_path().await?,
        );

        Ok(())
    }
}
