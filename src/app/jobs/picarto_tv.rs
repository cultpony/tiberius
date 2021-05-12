use anyhow::Result;
use log::{info, trace};
use philomena_models::{Channel, Client};
use sqlxmq::{job, Checkpoint, CurrentJob};
use tide::Request;

use crate::request_helper::SafeSqlxRequestExt;

#[job(channel_name = "picarto_tv_refresh_channels")]
pub async fn run_job(mut current_job: CurrentJob) -> Result<()> {
    let pool = current_job.pool();
    let mut client = Request::get_db_client_standalone(pool.clone()).await?;
    info!("Job {}: Refreshing picarto channels", current_job.id());
    let mut progress = {
        if let Some(previous_progress) = current_job.json()? {
            previous_progress
        } else {
            let all_channels =
                Channel::get_all_channels(&mut client, Some("PicartoChannel")).await?;
            let progress = Progress {
                all_channels: all_channels,
                done_channels: Vec::new(),
            };
            progress
        }
    };
    let mut checkpoint = Checkpoint::new();
    //TODO: allow recovering broken jobs
    checkpoint.set_json(&progress)?;
    for mut channel in progress.all_channels.clone() {
        if progress.done_channels.contains(&channel.id) {
            continue;
        }
        refresh_channel(&mut client, &mut channel).await?;
        progress.done_channels.push(channel.id);
        checkpoint.set_json(&progress)?;
        current_job.checkpoint(&checkpoint).await?;
    }
    current_job.complete().await?;
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Progress {
    all_channels: Vec<Channel>,
    done_channels: Vec<i32>,
}

async fn refresh_channel(client: &mut Client, chan: &mut Channel) -> Result<()> {
    let http_client = crate::http_client()?;
    let url = format!(
        "https://api.picarto.tv/api/v1/channel/name/{}",
        chan.short_name
    );
    trace!("requesting picarto channel via {}", url);
    let pic_chan: PicartoChannel = http_client.get(url).send().await?.json().await?;
    chan.remote_stream_id = Some(pic_chan.user_id as i32);
    chan.thumbnail_url = Some(pic_chan.thumbnails.web);
    chan.channel_image = Some(pic_chan.avatar);
    chan.is_live = pic_chan.online;
    chan.last_fetched_at = Some(chrono::Utc::now().naive_utc());
    if chan.is_live {
        chan.last_live_at = chan.last_fetched_at;
    }
    chan.viewers = pic_chan.viewers as i32;
    chan.title = pic_chan.title;
    chan.update(client).await?;
    Ok(())
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
struct PicartoChannel {
    user_id: i64,
    name: String,
    avatar: String,
    online: bool,
    viewers: i64,
    viewers_total: i64,
    thumbnails: Thumbnails,
    followers: i64,
    subscribers: i64,
    adult: bool,
    category: Vec<String>,
    account_type: String,
    commissions: bool,
    recordings: bool,
    title: String,
    description_panels: Vec<DescriptionPanels>,
    private: bool,
    private_message: Option<String>,
    gaming: bool,
    chat_settings: ChatSettings,
    last_live: Option<String>,
    tags: Vec<String>,
    multistream: Vec<Multistream>,
    languages: Vec<Language>,
    following: bool,
    creation_date: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
struct Thumbnails {
    web: String,
    web_large: String,
    mobile: String,
    tablet: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
struct DescriptionPanels {
    title: String,
    body: String,
    image: Option<String>,
    image_link: Option<String>,
    button_text: Option<String>,
    button_link: Option<String>,
    position: i64,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
struct ChatSettings {
    guest_chat: bool,
    links: bool,
    level: bool,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
struct Multistream {
    user_id: i64,
    name: String,
    online: bool,
    adult: bool,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
struct Language {
    id: i64,
    name: String,
}
