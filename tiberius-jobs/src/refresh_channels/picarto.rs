use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::http_client;
use tiberius_models::{Client, Channel};
use tiberius_dependencies::prelude::*;


#[instrument]
pub async fn refresh_picarto_channel(
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
    chan.last_fetched_at = Some(tiberius_dependencies::chrono::Utc::now().naive_utc());
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
