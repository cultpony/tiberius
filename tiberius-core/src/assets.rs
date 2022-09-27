use std::{borrow::Cow, collections::BTreeMap, path::PathBuf, str::FromStr};

use anyhow::Context;
use async_std::sync::RwLock;
use axum::{Router, Extension};
use tiberius_dependencies::axum::headers::{ContentType, HeaderMapExt};
use tokio::io::AsyncReadExt;
use tracing::info;

use crate::state::TiberiusState;
use crate::{
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    footer::FooterData,
    request_helper::FileResponse,
};
use tiberius_dependencies::{axum_extra, mime, rust_embed};

use axum_extra::routing::{RouterExt, TypedPath};

pub fn embedded_file_pages(r: Router) -> Router {
    r.typed_get(serve_favicon_ico)
        .typed_get(serve_favicon_svg)
        .typed_get(serve_robots)
        .typed_get(serve_asset)
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../res/assets-build/"]
#[prefix = "/"]
pub struct Assets;

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/favicon.ico")]
pub struct GetFaviconIco {}

pub async fn serve_favicon_ico(_: GetFaviconIco, Extension(state): Extension<TiberiusState>) -> TiberiusResult<FileResponse> {
    if state.config().try_use_ondisk_favicon {
        let base = PathBuf::from_str(&state.config().static_root)?;
        let favicon = base.join("favicon.ico");
        if favicon.exists() {
            return serve_disk_file(&state, favicon).await;
        }
    }
    serve_static_file(PathBuf::from_str("/favicon.ico")?).await
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/favicon.svg")]
pub struct GetFaviconSvg {}
pub async fn serve_favicon_svg(_: GetFaviconSvg, Extension(state): Extension<TiberiusState>) -> TiberiusResult<FileResponse> {
    if state.config().try_use_ondisk_favicon {
        let base = PathBuf::from_str(&state.config().static_root)?;
        let favicon = base.join("favicon.svg");
        if favicon.exists() {
            return serve_disk_file(&state, favicon).await;
        }
    }
    serve_static_file(PathBuf::from_str("/favicon.svg")?).await
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/robots.txt")]
pub struct GetRobotsTxt {}

pub async fn serve_robots(_: GetRobotsTxt) -> TiberiusResult<FileResponse> {
    serve_static_file(PathBuf::from_str("/static/robots.txt")?).await
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/static/*path")]
pub struct GetStaticFile {
    path: String,
}

pub async fn serve_asset(GetStaticFile { path }: GetStaticFile) -> TiberiusResult<FileResponse> {
    serve_static_file(path.try_into()?).await
}

#[instrument]
pub async fn serve_static_file(file: PathBuf) -> TiberiusResult<FileResponse> {
    trace!("Serving static file {:?}", file);
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
            let content_type = content_type.first().unwrap_or(mime::TEXT_PLAIN_UTF_8);
            trace!(
                "Serving static file {} with content type {}",
                path.display(),
                content_type
            );
            use tiberius_dependencies::http::HeaderMap;
            let mut hm = HeaderMap::new();
            hm.typed_insert(ContentType::from(content_type));
            FileResponse {
                content: file.data,
                headers: hm,
            }
        }
    })
}

pub async fn serve_disk_file(state: &TiberiusState, file: PathBuf) -> TiberiusResult<FileResponse> {
    trace!("Serving disk file {:?}", file);
    assert!(file.starts_with(state.config().static_root.clone()), "Disk Files must come from Static Root or known safe file location");
    let path = file.clone();
    let file = tokio::fs::File::open(file).await;
    Ok(match file {
        Err(e) => {
            warn!("Static File serving failed : {}, pretending 404", e);
            return Err(TiberiusError::Other(format!(
                "file {} not found",
                path.display()
            )))
        }
        Ok(mut file) => {
            let content_type =
                new_mime_guess::from_ext(&path.extension().unwrap_or_default().to_string_lossy());
            let content_type = content_type.first().unwrap_or(mime::TEXT_PLAIN_UTF_8);
            trace!(
                "Serving static file '{}' with content type {}",
                path.display(),
                content_type
            );
            use tiberius_dependencies::http::HeaderMap;
            let mut hm = HeaderMap::new();
            hm.typed_insert(ContentType::from(content_type));
            let mut buffer = Vec::with_capacity(file.metadata().await?.len() as usize);
            let read = file.read_to_end(&mut buffer).await?;
            assert!(read == buffer.len(), "Under or overread, wanted {} bytes got {} bytes", buffer.len(), read);
            let buffer = Cow::from(buffer);
            FileResponse {
                content: buffer,
                headers: hm,
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
        let data: FooterData =
            serde_json::from_reader(data).context("Could not parse Footer Data")?;
        let mut siteconf = dataroot.clone();
        siteconf.push("site-conf.json");
        let siteconf = std::fs::File::open(siteconf).context("Could not find site config data")?;
        let siteconf: SiteConfig =
            serde_json::from_reader(siteconf).context("Could not parse site config data")?;
        let mut quicktagtable = dataroot.clone();
        quicktagtable.push("quick_tag_table.json");
        let quicktagtable =
            std::fs::File::open(quicktagtable).context("Could not find quick tag table")?;
        let quicktagtable =
            serde_json::from_reader(quicktagtable).context("Could not parse quicktag table")?;
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
