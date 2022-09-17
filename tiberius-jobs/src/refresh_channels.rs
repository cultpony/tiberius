use sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_core::{
    config::Configuration, error::TiberiusResult, http_client, state::TiberiusState,
};
use tiberius_models::{Channel, Client};
use tracing::{debug, info, trace};

use crate::SharedCtx;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PicartoConfig {
    pub config: Configuration,
    pub all_channels: Vec<Channel>,
    pub done_channels: Vec<i32>,
    pub started: bool,
}

impl Default for PicartoConfig {
    fn default() -> Self {
        Self {
            config: Configuration::default(),
            all_channels: Vec::new(),
            done_channels: Vec::new(),
            started: false,
        }
    }
}

#[instrument(level = "trace")]
#[sqlxmq::job(retries = 1)]
pub async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    let pool = current_job.pool();
    let progress: PicartoConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    info!("Job {}: Refreshing picarto channels", current_job.id());
    let mut client = sctx.client;
    let mut progress = {
        if progress.started {
            progress
        } else {
            let all_channels =
                Channel::get_all_channels(&mut client, Some("PicartoChannel")).await?;
            let progress = PicartoConfig {
                config: progress.config,
                all_channels: all_channels,
                done_channels: Vec::new(),
                started: true,
            };
            progress
        }
    };
    info!("Loading checkpoint for channel refresh");
    let mut checkpoint = Checkpoint::new();
    checkpoint.set_json(&progress)?;
    for mut channel in progress.all_channels.clone() {
        debug!("Job {}: refreshing channel {}", current_job.id(), channel.id);
        if progress.done_channels.contains(&channel.id) {
            continue;
        }
        refresh_channel(&progress.config, &mut client, &mut channel).await?;
        progress.done_channels.push(channel.id);
        checkpoint.set_json(&progress)?;
        current_job.checkpoint(&checkpoint).await?;
        debug!("Completed refresh for channel {}", channel.id);
    }
    info!("Job {}: Completed refresh", current_job.id());
    current_job.complete().await?;
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Progress {
    config: Configuration,
    all_channels: Vec<Channel>,
    done_channels: Vec<i32>,
}

async fn refresh_channel(
    config: &Configuration,
    client: &mut Client,
    chan: &mut Channel,
) -> TiberiusResult<()> {
    match chan.r#type {
        tiberius_models::ChannelType::PicartoChannel => {
            refresh_picarto_channel(config, client, chan).await
        }
        tiberius_models::ChannelType::PiczelChannel => todo!(),
        tiberius_models::ChannelType::TwitchChannel => todo!(),
    }
}

#[instrument(level = "trace")]
async fn refresh_picarto_channel(
    config: &Configuration,
    client: &mut Client,
    chan: &mut Channel,
) -> TiberiusResult<()> {
    let http_client = http_client(config)?;
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
        debug!("Channel {} is online: {}", chan.short_name, pic_chan.title);
        chan.last_live_at = chan.last_fetched_at;
    } else {
        debug!("Channel {} is offline: {}", chan.short_name, pic_chan.title);
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
