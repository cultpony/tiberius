use std::collections::BTreeMap;
use std::{borrow::Cow, path::PathBuf, str::FromStr};

use anyhow::Context;
use async_std::sync::RwLock;
use rocket::response::content;
use rocket::response::status;
use rocket::response::stream::ReaderStream;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{ContentType, Status},
    response::stream::ByteStream,
    Request, State,
};
use tracing::info;

use crate::config::Configuration;
use crate::error::{TiberiusError, TiberiusResult};
use crate::footer::FooterData;
use crate::request_helper::FileResponse;

#[derive(rust_embed::RustEmbed)]
#[folder = "../res/assets-build/"]
#[prefix = "/static/"]
pub struct Assets;

#[get("/favicon.ico")]
pub async fn serve_favicon_ico() -> TiberiusResult<FileResponse> {
    serve_static_file(PathBuf::from_str("/static/favicon.ico")?).await
}

#[get("/favicon.svg")]
pub async fn serve_favicon_svg() -> TiberiusResult<FileResponse> {
    serve_static_file(PathBuf::from_str("/static/favicon.ico")?).await
}

#[get("/robots.txt")]
pub async fn serve_robots() -> TiberiusResult<FileResponse> {
    serve_static_file(PathBuf::from_str("/static/robots.txt")?).await
}

#[get("/static/<path..>")]
pub async fn serve_asset(path: PathBuf) -> TiberiusResult<FileResponse> {
    serve_static_file(path).await
}

pub async fn serve_static_file(file: PathBuf) -> TiberiusResult<FileResponse> {
    let file = PathBuf::from_str("/static/").unwrap().join(file);
    let path = file.clone();
    let file = Assets::get(file.to_str().unwrap());
    Ok(match file {
        None => {
            return Err(TiberiusError::Other(format!(
                "file {} not found",
                path.display()
            )))
        }
        Some(file) => {
            let content_type =
                new_mime_guess::from_ext(&path.extension().unwrap_or_default().to_string_lossy());
            let content_type = content_type.first();
            let content_type = match content_type {
                None => rocket::http::ContentType::Plain.to_string(),
                Some(t) => t.essence_str().to_string(),
            };
            trace!(
                "Serving static file {} with content type {}",
                path.display(),
                content_type
            );
            FileResponse {
                content: file.data,
                content_type: ContentType::from_str(&content_type)
                    .map_err(|x| TiberiusError::Other(x))?,
            }
        }
    })
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct SiteConfig {
    name: String,
    source_repo: String,
    source_name: String,
    activity_filter: String,
    tag_url_root: String,
}

impl SiteConfig {
    pub fn site_name(&self) -> &String {
        &self.name
    }
    pub fn source_repo(&self) -> &String {
        &self.source_repo
    }
    pub fn source_name(&self) -> &String {
        &self.source_name
    }
    pub fn activity_filter(&self) -> &String {
        &self.activity_filter
    }
    pub fn tag_url_root(&self) -> &String {
        &self.tag_url_root
    }
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct QuickTagTable(Vec<QuickTagTableEntry>);

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(tag = "mode")]
pub enum QuickTagTableContent {
    Default(QuickTagTableDefault),
    ShortHand(QuickTagTableShortHand),
    Shipping(QuickTagTableShipping),
    Season(QuickTagTableSeason),
}
#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableEntry {
    pub title: String,
    #[serde(flatten)]
    pub content: QuickTagTableContent,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableDefault {
    pub tables: Vec<QuickTagTableDefaultTable>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableDefaultTable {
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableShortHand {
    pub mappings: Vec<QuickTagTableShortHandMapping>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableShortHandMapping {
    pub title: String,
    pub map: BTreeMap<String, String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableShipping {
    pub implying: Vec<String>,
    pub not_implying: Vec<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableSeason {
    pub episodes: Vec<QuickTagTableSeasonSeason>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct QuickTagTableSeasonSeason {
    pub episode_number: String,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct AssetLoader {
    data: FooterData,
    siteconf: SiteConfig,
    quicktagtable: QuickTagTable,
}

impl AssetLoader {
    pub fn new(c: &Configuration) -> TiberiusResult<Self> {
        tracing::info!("Configuring Assets");
        let dataroot = std::path::PathBuf::from(&c.static_root);
        if !dataroot.exists() {
            tracing::error!("COULD NOT FIND ASSETS ON DISK");
            return Ok(Self::default());
        }
        tracing::debug!("Data root for static assets is {}", dataroot.display());
        let mut data = dataroot.clone();
        data.push("footer.json");
        let data = std::fs::File::open(data).context("Could not find footer data")?;
        let data: FooterData = serde_json::from_reader(data).context("Could not parse Footer Data")?;
        let mut siteconf = dataroot.clone();
        siteconf.push("site-conf.json");
        let siteconf = std::fs::File::open(siteconf).context("Could not find site config data")?;
        let siteconf: SiteConfig = serde_json::from_reader(siteconf).context("Could not parse site config data")?;
        let mut quicktagtable = dataroot.clone();
        quicktagtable.push("quick_tag_table.json");
        let quicktagtable = std::fs::File::open(quicktagtable).context("Could not find quick tag table")?;
        let quicktagtable = serde_json::from_reader(quicktagtable).context("Could not parse quicktag table")?;
        Ok(Self {
            data,
            siteconf,
            quicktagtable,
        })
    }
    pub fn footer_data(&self) -> &FooterData {
        &self.data
    }
    pub fn site_config(&self) -> &SiteConfig {
        &self.siteconf
    }
    pub fn quick_tag_table(&self) -> &Vec<QuickTagTableEntry> {
        &self.quicktagtable.0
    }
}

pub trait TiberiusStateAssetExt {}
