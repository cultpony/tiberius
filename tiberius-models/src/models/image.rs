use std::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::{ops::DerefMut, path::PathBuf, pin::Pin};

use async_std::prelude::*;
use async_std::sync::RwLock;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::TryStreamExt;
use ipnetwork::IpNetwork;
use itertools::Itertools;
use sqlx::{postgres::PgRow, query_as, Executor, FromRow, PgPool};
use tantivy::IndexWriter;
use tiberius_search::Queryable;
use tracing::trace;

use crate::pluggables::{Hashable, Representations, Intensities, ImageInteractionMetadata, ImageFileMetadata, ImageUrls};
use crate::{
    tantivy_bool_text_field, tantivy_date_field, tantivy_raw_text_field, tantivy_text_field,
    tantivy_u64_field, Client, ImageFeature, ImageTag, PhilomenaModelError, Tag,
};
#[cfg(feature = "verify-db")]
use crate::VerifiableTable;

pub struct VerifierImage {
    pool: PgPool,
    client: Client,
    start_id: u64,
    end_id: u64,
    subbatching: u64,
}

#[cfg(feature = "verify-db")]
#[async_trait]
impl VerifiableTable for VerifierImage {
    async fn verify(&mut self) -> Result<(), PhilomenaModelError> {
        tracing::warn!("Verifying consistency of image database");
        let mut images =
            Image::get_all(self.pool.clone(), Some(self.start_id), Some(self.end_id)).await?;
        let mut images_scanned: u64 = 0;
        let size = Image::count(&mut self.client, Some(self.start_id), Some(self.end_id)).await?;
        tracing::warn!("Got image stream (expecting {} items), scanning...", size);

        let (tag_scanner_send, tag_scanner_recv) = async_std::channel::bounded(32);
        let (user_scanner_send, user_scanner_recv) = async_std::channel::bounded(32);
        let (image_scanner_send, image_scanner_recv) = async_std::channel::bounded(32);
        let tag_scanner = {
            let client = self.client.clone_new_conn(&mut self.pool).await?;
            tokio::spawn(async {
                let recv: async_std::channel::Receiver<Vec<i64>> = tag_scanner_recv;
                let mut client = client;
                loop {
                    let ids = recv.recv().await;
                    let ids = match ids {
                        Ok(v) => v,
                        Err(_) => {
                            tracing::warn!("tag scanner closing...");
                            return;
                        }
                    };
                    let ids = query_as!(
                        ImageTag,
                        "SELECT * FROM image_taggings WHERE image_id = ANY($1)",
                        &ids
                    )
                    .fetch_all(client.db().await.unwrap().deref_mut())
                    .await
                    .unwrap();
                    let tags: Vec<i64> = ids
                        .iter()
                        .map(|x| &x.tag_id)
                        .map(|x| *x as i64)
                        .sorted()
                        .unique()
                        .collect();
                    let tags = Tag::get_many(&mut client, tags).await;
                    let tags = match tags {
                        Ok(v) => v,
                        Err(v) => {
                            tracing::error!("Error while processing tag batch: {:?}", v);
                            return;
                        }
                    };
                    let tags: Vec<i64> = tags.iter().map(|x| x.id as i64).collect();
                    let missing_tags: Vec<&ImageTag> =
                        ids.iter().filter(|x| !tags.contains(&x.tag_id)).collect();
                    if !missing_tags.is_empty() {
                        tracing::warn!(
                            "Following tags ({}) not found in database: ",
                            missing_tags.len()
                        );
                        for tag in missing_tags {
                            tracing::error!(" - Image {} missing ({:?})", tag.image_id, tag.tag_id);
                        }
                    }
                }
            })
        };
        let image_scanner = {
            let client = self.client.clone_new_conn(&mut self.pool).await?;
            tokio::spawn(async {
                let recv: async_std::channel::Receiver<Vec<i32>> = image_scanner_recv;
                let mut client = client;
                loop {
                    let ids = recv.recv().await;
                    let ids = match ids {
                        Ok(v) => v,
                        Err(_) => {
                            tracing::warn!("image scanner closing...");
                            return;
                        }
                    };
                    let images = query_as!(Image, "SELECT * FROM images WHERE id = ANY($1)", &ids)
                        .fetch_all(client.db().await.unwrap().deref_mut())
                        .await
                        .unwrap();
                    let images: Vec<i64> = images
                        .iter()
                        .map(|x| &x.id)
                        .map(|x| *x as i64)
                        .sorted()
                        .unique()
                        .collect();
                    let images = Image::get_many(&mut client, images).await;
                    let images = match images {
                        Ok(v) => v,
                        Err(v) => {
                            tracing::error!("Error while processing image batch: {:?}", v);
                            return;
                        }
                    };
                    let images: Vec<i64> = images.iter().map(|x| x.id as i64).collect();
                    let missing_images: Vec<&i32> = ids
                        .iter()
                        .filter(|x| !images.contains(&(**x as i64)))
                        .collect();
                    if !missing_images.is_empty() {
                        tracing::warn!(
                            "Following images ({}) not found in database: ",
                            missing_images.len()
                        );
                        for image in missing_images {
                            tracing::error!(" - Image {} missing", image);
                        }
                    }
                }
            })
        };
        let user_scanner = {
            let client = self.client.clone_new_conn(&mut self.pool).await?;
            tokio::spawn(async {
                let recv: async_std::channel::Receiver<Vec<(i64, Vec<i32>)>> = user_scanner_recv;
                let client = client;
                loop {
                    let ids = recv.recv().await;
                    let ids = match ids {
                        Ok(v) => v,
                        Err(_) => {
                            tracing::warn!("user scanner closing...");
                            return;
                        }
                    };
                    struct UserIdOnly {
                        id: i32,
                    }
                    let sub_ids: Vec<i32> = ids.iter().map(|x| x.1.clone()).flatten().collect();
                    let users = query_as!(
                        UserIdOnly,
                        "SELECT id FROM users WHERE id = ANY($1)",
                        &sub_ids
                    )
                    .fetch_all(client.db().await.unwrap().deref_mut())
                    .await
                    .unwrap();
                    let users: Vec<i32> = users.iter().map(|x| x.id).sorted().unique().collect();
                    let missing_users: Vec<(&i64, Vec<&i32>)> = ids
                        .iter()
                        .map(|(i, us)| {
                            let us: Vec<&i32> = us.iter().filter(|x| !users.contains(*x)).collect();
                            (i, us)
                        })
                        .filter(|x| !x.1.is_empty())
                        .collect();
                    if !missing_users.is_empty() {
                        tracing::warn!(
                            "Following users ({}) not found in database: ",
                            missing_users.len()
                        );
                        for user in missing_users {
                            tracing::error!(" - Image {} missing users {:?}", user.0, user.1);
                        }
                    }
                }
            })
        };
        use progressing::Baring;
        let mut progress = progressing::bernoulli::Bar::with_goal(size as usize).timed();
        let mut tag_batches = Vec::new();
        let mut image_batches = Vec::new();
        let mut user_batches = Vec::new();
        while let Some(image) = images.try_next().await? {
            images_scanned += 1;
            progress.add(1);
            if progress.has_progressed_significantly() {
                progress.remember_significant_progress();
                tracing::info!("{}", progress);
            }
            let image: Image = Image::from_row(&image)?;
            let mut users = image.watcher_ids;
            if let Some(user_id) = image.user_id {
                users.push(user_id);
            }
            if let Some(deleted_by_id) = image.deleted_by_id {
                users.push(deleted_by_id);
            }
            user_batches.push((image.id as i64, users));
            if let Some(duplicate_id) = image.duplicate_id {
                image_batches.push(duplicate_id);
            }
            tag_batches.push(image.id as i64);

            if tag_batches.len() > self.subbatching as usize {
                tag_scanner_send
                    .send(tag_batches.clone())
                    .await
                    .expect("could not send tag batch");
                tag_batches = Vec::new();
            }
            if image_batches.len() > self.subbatching as usize {
                image_scanner_send
                    .send(image_batches.clone())
                    .await
                    .expect("could not send image batch");
                image_batches = Vec::new();
            }
            if user_batches.len() > self.subbatching as usize {
                user_scanner_send
                    .send(user_batches.clone())
                    .await
                    .expect("could not send user batch");
                user_batches = Vec::new();
            }
        }
        if tag_batches.len() > 0 {
            tag_scanner_send
                .send(tag_batches.clone())
                .await
                .expect("could not send final tag batch");
        }

        if image_batches.len() > 0 {
            image_scanner_send
                .send(image_batches.clone())
                .await
                .expect("could not send final image batch");
        }

        if user_batches.len() > 0 {
            user_scanner_send
                .send(user_batches.clone())
                .await
                .expect("could not send final user batch");
        }
        tracing::warn!("Waiting for remaining data batches to be verified");
        image_scanner_send.close();
        tag_scanner_send.close();
        user_scanner_send.close();
        image_scanner.await.unwrap();
        tag_scanner.await.unwrap();
        user_scanner.await.unwrap();
        tracing::warn!("Scanned {} images (expected {})", images_scanned, size);
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct APIImage {
    pub id: u64,
    #[serde(flatten)]
    pub hash: Hashable,
    #[serde(flatten)]
    pub urls: ImageUrls,
    #[serde(flatten)]
    pub image_file_metadata: ImageFileMetadata,
    #[serde(flatten)]
    pub image_interaction_metadata: ImageInteractionMetadata,
    pub first_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub uploader_id: u64,
    pub uploader: String,
    pub description: String,
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

impl Image {
    #[cfg(feature = "verify-db")]
    pub fn verifier(
        client: Client,
        pool: PgPool,
        start_id: u64,
        end_id: u64,
        subbatching: u64,
    ) -> Box<dyn VerifiableTable> {
        Box::new(VerifierImage {
            client,
            pool,
            start_id,
            end_id,
            subbatching,
        })
    }
    pub async fn upload(self, client: &mut Client) -> Result<Image, PhilomenaModelError> {
        assert!(self.id == 0, "New images must have ID == 0");
        // Some sanity asserts here, then store in DB
        todo!()
    }
    pub fn filename(&self) -> String {
        format!(
            "{}.{}",
            self.id,
            self.image_format.as_ref().unwrap_or(&"png".to_string())
        )
    }
    pub fn filetypef<S: Display>(&self, s: S) -> String {
        format!(
            "{}.{}",
            s,
            self.image_format.as_ref().unwrap_or(&"png".to_string())
        )
    }
    pub async fn tags(&self, client: &mut Client) -> Result<Vec<Tag>, PhilomenaModelError> {
        Ok(query_as!(
            crate::Tag,
            "SELECT * FROM tags WHERE id IN (SELECT tag_id FROM image_taggings WHERE image_id = $1)",
            self.id as i64,
        )
        .fetch_all(client)
        .await?)
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
        .fetch_one(client.db().await?.deref_mut())
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
            .execute(client.db().await?.deref_mut())
            .await?;
        }
        Ok(Image::get(client, self.id as i64)
            .await?
            .expect("we just uploaded this"))
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
        .fetch_one(client.db().await?.deref_mut())
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
            .execute(client.db().await?.deref_mut())
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
        .fetch_all(client.db().await?.deref_mut())
        .await?)
    }
    pub async fn get(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(
            query_as!(Image, "SELECT * FROM images WHERE id = $1", (id as i32),)
                .fetch_optional(client.db().await?.deref_mut())
                .await?,
        )
    }
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
            .fetch_one(client.db().await?.deref_mut())
            .await?
            .cnt
            .unwrap_or_default() as u64),
            (Some(start_id), None) => Ok(query_as!(
                Count,
                "SELECT COUNT(*) AS cnt FROM images WHERE id > $1",
                start_id as i64
            )
            .fetch_one(client.db().await?.deref_mut())
            .await?
            .cnt
            .unwrap_or_default() as u64),
            (None, Some(end_id)) => Ok(query_as!(
                Count,
                "SELECT COUNT(*) AS cnt FROM images WHERE id <= $1",
                end_id as i64
            )
            .fetch_one(client.db().await?.deref_mut())
            .await?
            .cnt
            .unwrap_or_default() as u64),
            (None, None) => Ok(query_as!(Count, "SELECT COUNT(*) AS cnt FROM images")
                .fetch_one(client.db().await?.deref_mut())
                .await?
                .cnt
                .unwrap_or_default() as u64),
        }
    }
    pub async fn get_many(
        client: &mut Client,
        ids: Vec<i64>,
    ) -> Result<Vec<Self>, PhilomenaModelError> {
        let ids: Vec<i32> = ids.iter().map(|x| *x as i32).collect();
        Ok(query_as!(
            Image,
            "SELECT * FROM images WHERE id = ANY($1) LIMIT 100",
            &ids
        )
        .fetch_all(client.db().await?.deref_mut())
        .await?)
    }
    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        Ok(query_as!(
            Image,
            "SELECT * FROM images WHERE id = $1 LIMIT 1",
            id as i32
        )
        .fetch_optional(client.db().await?.deref_mut())
        .await?)
    }
    pub async fn get_all(
        pool: PgPool,
        start_id: Option<u64>,
        end_id: Option<u64>,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PgRow, sqlx::Error>>>>, PhilomenaModelError>
    {
        match (start_id, end_id) {
            (Some(start_id), Some(end_id)) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM images WHERE id BETWEEN $1 AND $2",
                start_id as i64,
                end_id as i64
            ))),
            (Some(start_id), None) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM images WHERE id > $1",
                start_id as i64
            ))),
            (None, Some(end_id)) => Ok(pool.fetch(sqlx::query!(
                "SELECT * FROM images WHERE id <= $1",
                end_id as i64
            ))),
            (None, None) => Ok(pool.fetch(sqlx::query!("SELECT * FROM images"))),
        }
    }
    pub async fn get_featured(client: &mut Client) -> Result<Option<Self>, PhilomenaModelError> {
        let feature: Option<ImageFeature> = sqlx::query_as!(
            ImageFeature,
            "SELECT * FROM image_features ORDER BY created_at LIMIT 1"
        )
        .fetch_optional(client.db().await?.deref_mut())
        .await?;
        if let Some(feature) = feature {
            Ok(Self::get_id(client, feature.image_id).await?)
        } else {
            Ok(None)
        }
    }
    pub async fn search<
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
        S4: Into<String>,
        S5: Into<String>,
    >(
        client: &mut Client,
        query: S1,
        aqueries: Vec<S4>,
        anqueries: Vec<S5>,
        _sort_by: Option<S2>,
        _order_by: Option<S3>,
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
        );
        let (total, ids) = match ids {
            Ok((total, v)) => (total, v.iter().map(|x| x.1 as i64).collect()),
            Err(e) => return Err(PhilomenaModelError::Searcher(e)),
        };
        Ok((total as u64, Self::get_many(client, ids).await?))
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
}

#[async_trait::async_trait]
impl Queryable for Image {
    type Group = String;
    type DBClient = Client;
    type IndexError = PhilomenaModelError;

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
        let mut doc = tantivy::Document::new();
        let schema = Self::schema();
        doc.add_date(
            schema.get_field("created_at").unwrap(),
            &chrono::DateTime::<chrono::Utc>::from_utc(self.created_at, chrono::Utc),
        );
        doc.add_u64(schema.get_field("id").unwrap(), self.id as u64);
        doc.add_text(schema.get_field("description").unwrap(), &self.description);
        doc.add_text(
            schema.get_field("processed").unwrap(),
            self.processed.to_string(),
        );
        doc.add_text(
            schema.get_field("deleted").unwrap(),
            self.deleted_by_id.is_some().to_string(),
        );
        let tag_field = schema.get_field("tag").unwrap();
        for tag in self.tags(&mut client).await? {
            doc.add_text(tag_field, tag.full_name());
        }
        //debug!("Sending {:?} to index", doc);
        writer.write().await.add_document(doc);
        Ok(())
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

#[derive(Clone, Copy)]
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
    pub rendered: PathBuf,
    pub full: PathBuf,
    pub tall: PathBuf,
    pub large: PathBuf,
    pub medium: PathBuf,
    pub small: PathBuf,
    pub thumb: PathBuf,
    pub thumb_small: PathBuf,
    pub thumb_tiny: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolutionLimit {
    height: u32,
    width: u32,
}

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
        if height <= self.height && width <= self.width {
            Resolution { height, width }
        } else if height < self.height && width > self.width {
            let ratio = self.width as f64 / width as f64;
            let nwidth = width as f64 * ratio;
            let nheight = height as f64 * ratio;
            let height = nheight.floor() as u32;
            let width = nwidth.floor() as u32;
            Resolution { height, width }
        } else if height > self.height && width < self.width {
            let ratio = self.height as f64 / height as f64;
            let nwidth = width as f64 * ratio;
            let nheight = height as f64 * ratio;
            let height = nheight.floor() as u32;
            let width = nwidth.floor() as u32;
            Resolution { height, width }
        } else {
            let aratio = self.height as f64 / height as f64;
            let bratio = self.width as f64 / width as f64;
            let ratio = aratio.min(bratio);
            let nwidth = width as f64 * ratio;
            let nheight = height as f64 * ratio;
            let height = nheight.floor() as u32;
            let width = nwidth.floor() as u32;
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
            Tall => Some((1024, 4096).into()),
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
