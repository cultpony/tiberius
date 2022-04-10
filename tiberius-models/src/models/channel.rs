use std::io::Write;
use std::ops::DerefMut;
use std::str::FromStr;

use crate::{Client, PhilomenaModelError, Tag};
use chrono::NaiveDateTime;
use itertools::Itertools;
use sqlx::{query_as, Postgres};
use tracing::trace;

#[derive(sqlx::FromRow, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Channel {
    pub id: i32,
    pub short_name: String,
    pub title: String,
    pub description: Option<String>,
    pub channel_image: Option<String>,
    pub tags: Option<String>,
    pub viewers: i32,
    pub nsfw: bool,
    pub is_live: bool,
    pub last_fetched_at: Option<NaiveDateTime>,
    pub next_check_at: Option<NaiveDateTime>,
    pub last_live_at: Option<NaiveDateTime>,
    pub watcher_ids: Vec<i32>,
    pub watcher_count: i32,
    pub r#type: ChannelType,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub associated_artist_tag_id: Option<i32>,
    pub viewer_minutes_today: i32,
    pub viewer_minutes_thisweek: i32,
    pub viewer_minutes_thismonth: i32,
    pub total_viewer_minutes: i32,
    pub banner_image: Option<String>,
    pub remote_stream_id: Option<i32>,
    pub thumbnail_url: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug, sqlx::Type)]
pub enum ChannelType {
    PicartoChannel,
    PiczelChannel,
    TwitchChannel,
}

impl ToString for ChannelType {
    fn to_string(&self) -> String {
        match self {
            ChannelType::PicartoChannel => "PicartoChannel",
            ChannelType::PiczelChannel => "PiczelChannel",
            ChannelType::TwitchChannel => "TwitchChannel",
        }.to_string()
    }
}

impl FromStr for ChannelType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "PicartoChannel" => Self::PicartoChannel,
            "PiczelChannel" => Self::PiczelChannel,
            "TwitchChannel" => Self::TwitchChannel,
            v => anyhow::bail!("Invalid channel type: {:?}", v)
        })
    }
}

impl Channel {
    pub async fn get_frontpage_channels<'a>(
        client: &mut Client,
    ) -> Result<Vec<Channel>, PhilomenaModelError> {
        trace!("loading frontpage channels");
        Ok(query_as!(
            Channel,
            "SELECT id, short_name, title, description, channel_image,
                tags, viewers, nsfw, is_live, last_fetched_at, next_check_at,
                last_live_at, watcher_ids, watcher_count, type as \"type: ChannelType\",
                created_at, updated_at, associated_artist_tag_id, viewer_minutes_today,
                viewer_minutes_thisweek, viewer_minutes_thismonth, total_viewer_minutes,
                banner_image, remote_Stream_id, thumbnail_url
                FROM channels WHERE nsfw = false AND last_fetched_at is not null"
        )
        .fetch_all(client.db().await?.deref_mut())
        .await?)
    }
    pub async fn get_all_channels<S: Into<String>>(
        client: &mut Client,
        channel_type: Option<S>,
    ) -> Result<Vec<Channel>, PhilomenaModelError> {
        trace!("loading all channels");
        let channel_type: Option<String> = channel_type.map(|x| x.into());
        Ok(match channel_type {
            Some(channel_type) => {
                query_as!(
                    Channel,
                    "SELECT id, short_name, title, description, channel_image,
                    tags, viewers, nsfw, is_live, last_fetched_at, next_check_at,
                    last_live_at, watcher_ids, watcher_count, type as \"type: ChannelType\",
                    created_at, updated_at, associated_artist_tag_id, viewer_minutes_today,
                    viewer_minutes_thisweek, viewer_minutes_thismonth, total_viewer_minutes,
                    banner_image, remote_Stream_id, thumbnail_url FROM channels WHERE type = $1",
                    channel_type
                )
                .fetch_all(client.db().await?.deref_mut())
                .await?
            }
            None => {
                query_as!(Channel, "SELECT id, short_name, title, description, channel_image,
                tags, viewers, nsfw, is_live, last_fetched_at, next_check_at,
                last_live_at, watcher_ids, watcher_count, type as \"type: ChannelType\",
                created_at, updated_at, associated_artist_tag_id, viewer_minutes_today,
                viewer_minutes_thisweek, viewer_minutes_thismonth, total_viewer_minutes,
                banner_image, remote_Stream_id, thumbnail_url FROM channels",)
                    .fetch_all(client.db().await?.deref_mut())
                    .await?
            }
        })
    }

    pub async fn get_live_count(client: &mut Client) -> Result<u64, PhilomenaModelError> {
        #[derive(serde::Deserialize)]
        struct Cnt {
            cnt: Option<i64>,
        }
        trace!("fetching live channel count from database");
        let count: u64 = query_as!(
            Cnt,
            "SELECT COUNT(title) AS cnt FROM channels WHERE is_live = TRUE"
        )
        .fetch_one(client.db().await?.deref_mut())
        .await?
        .cnt
        .unwrap_or(0) as u64;
        trace!("got channel live count: {}", count);
        Ok(count)
    }

    pub async fn update(&self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        trace!("updating channel {}", self.id);
        query_as!(
            Channel,
            r#"UPDATE channels AS c
                SET 
                    short_name = $2,
                    title = $3,
                    description = $4,
                    channel_image = $5,
                    tags = $6,
                    viewers = $7,
                    nsfw = $8,
                    is_live = $9,
                    last_fetched_at = $10,
                    last_live_at = $11,
                    watcher_ids = $12,
                    watcher_count = $13,
                    type = $14,
                    associated_artist_tag_id = $15,
                    viewer_minutes_today = $16,
                    viewer_minutes_thisweek = $17,
                    viewer_minutes_thismonth = $18,
                    total_viewer_minutes = $19,
                    banner_image = $20,
                    remote_stream_id = $21,
                    updated_at = $22
                WHERE id = $1"#,
            self.id,
            self.short_name,
            self.title,
            self.description,
            self.channel_image,
            self.tags,
            self.viewers,
            self.nsfw,
            self.is_live,
            self.last_fetched_at,
            self.last_live_at,
            &self.watcher_ids,
            self.watcher_count,
            self.r#type.to_string(),
            self.associated_artist_tag_id,
            self.viewer_minutes_today,
            self.viewer_minutes_thisweek,
            self.viewer_minutes_thismonth,
            self.total_viewer_minutes,
            self.banner_image,
            self.remote_stream_id,
            chrono::Utc::now().naive_utc()
        )
        .execute(client.db().await?.deref_mut())
        .await?;
        Ok(())
    }
    pub fn title(&self) -> String {
        if self.title.is_empty() {
            self.short_name.clone()
        } else {
            self.title.clone()
        }
    }
    pub fn image(&self) -> &String {
        &self.banner_image.as_ref().unwrap_or(
            self.channel_image
                .as_ref()
                .expect("must have iether channel or banner"),
        )
    }
    pub async fn associated_artist_tag(
        &self,
        client: &mut Client,
    ) -> Result<Option<Tag>, PhilomenaModelError> {
        match self.associated_artist_tag_id {
            Some(associated_artist_tag_id) => Ok(query_as!(
                Tag,
                "SELECT * FROM tags WHERE id = $1",
                associated_artist_tag_id
            )
            .fetch_optional(client.db().await?.deref_mut())
            .await?),
            None => Ok(None),
        }
    }
    pub async fn count(
        client: &mut Client,
        include_nsfw: bool,
    ) -> Result<u64, PhilomenaModelError> {
        #[derive(serde::Deserialize)]
        struct Cnt {
            cnt: Option<i64>,
        }
        let cnt = if include_nsfw {
            query_as!(Cnt, "SELECT COUNT(*) AS Cnt FROM channels")
                .fetch_one(client.db().await?.deref_mut())
                .await?
                .cnt
                .unwrap_or(0)
        } else {
            query_as!(
                Cnt,
                "SELECT COUNT(*) AS Cnt FROM channels WHERE nsfw = false"
            )
            .fetch_one(client.db().await?.deref_mut())
            .await?
            .cnt
            .unwrap_or(0)
        };
        assert!(cnt >= 0, "Table cannot have negative amount of elements");
        Ok(cnt as u64)
    }
}
