use std::{borrow::Cow, path::PathBuf, str::FromStr};

use async_std::sync::RwLock;
use rocket::{Request, State, fairing::{Fairing, Info, Kind}, http::{ContentType, Status}, response::stream::ByteStream};
use rocket::response::status;
use rocket::response::stream::ReaderStream;
use rocket::response::content;

use crate::{config::Configuration, error::{TiberiusError, TiberiusResult}, pages::{common::frontmatter::FooterData, not_found_page}};

#[derive(rust_embed::RustEmbed)]
#[folder = "res/assets-build/"]
#[prefix = "/static/"]
pub struct Assets;

#[get("/favicon.ico")]
pub async fn serve_favicon_ico() -> TiberiusResult<status::Custom<content::Custom<ReaderStream![std::io::Cursor<Cow<'static, [u8]>>]>>> {
    serve_static_file(PathBuf::from_str("/static/favicon.ico")?).await
}

#[get("/favicon.svg")]
pub async fn serve_favicon_svg() -> TiberiusResult<status::Custom<content::Custom<ReaderStream![std::io::Cursor<Cow<'static, [u8]>>]>>> {
    serve_static_file(PathBuf::from_str("/static/favicon.ico")?).await
}

#[get("/robots.txt")]
pub async fn serve_robots() -> TiberiusResult<status::Custom<content::Custom<ReaderStream![std::io::Cursor<Cow<'static, [u8]>>]>>> {
    serve_static_file(PathBuf::from_str("/static/robots.txt")?).await
}

#[get("/static/<path..>")]
pub async fn serve_asset(path: PathBuf) -> TiberiusResult<status::Custom<content::Custom<ReaderStream![std::io::Cursor<Cow<'static, [u8]>>]>>> {
    serve_static_file(path).await
}

pub async fn serve_static_file(file: PathBuf) -> TiberiusResult<status::Custom<content::Custom<ReaderStream![std::io::Cursor<Cow<'static, [u8]>>]>>> {
    let file = Assets::get(file.to_str().unwrap());
    Ok(match file {
        None => return Err(TiberiusError::Other("file not found".to_string())),
        Some(file) => {
            let content_type = new_mime_guess::from_path(PathBuf::from(String::from_utf8_lossy(file.as_ref()).to_string()));
            let content_type = content_type.first();
            let content_type = match content_type {
                None => rocket::http::ContentType::Plain.to_string(),
                Some(t) => t.essence_str().to_string(),
            };
            status::Custom(
                Status::Ok,
                content::Custom(
                    ContentType::from_str(&content_type).map_err(|x| TiberiusError::Other(x))?,
                    ReaderStream::one(std::io::Cursor::new(file))
                )
            )
        }
    })
}

#[derive(serde::Deserialize, Clone)]
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

pub struct AssetLoader {
    data: RwLock<FooterData>,
    siteconf: RwLock<SiteConfig>,
}

impl AssetLoader {
    pub fn new(c: &Configuration) -> TiberiusResult<Self> {
        let dataroot = std::path::PathBuf::from(&c.static_root);
        let mut data = dataroot.clone();
        data.push("footer.json");
        let data = std::fs::File::open(data)?;
        let data: FooterData = serde_json::from_reader(data)?;
        let data = RwLock::new(data);
        let mut siteconf = dataroot.clone();
        siteconf.push("site-conf.json");
        let siteconf = std::fs::File::open(siteconf)?;
        let siteconf: SiteConfig = serde_json::from_reader(siteconf)?;
        let siteconf = RwLock::new(siteconf);
        Ok(Self { data, siteconf })
    }
}

#[rocket::async_trait]
impl Fairing for AssetLoader {
    fn info(&self) -> rocket::fairing::Info {
        Info {
            name: "Asset and Internal Configuration Loader",
            kind: Kind::Ignite | Kind::Request,
        }
    }

    async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
        rocket.manage(self.data.read().await.clone());
        rocket.manage(self.siteconf.read().await.clone());
        Ok(rocket)
    }
}

#[rocket::async_trait]
pub trait AssetLoaderRequestExt {
    async fn site_config(&self) -> &State<SiteConfig>;
    async fn footer_data(&self) -> &State<FooterData>;
}

#[rocket::async_trait]
impl AssetLoaderRequestExt for Request<'_> {
    async fn site_config(&self) -> &State<SiteConfig> {
        self.guard().await.unwrap()
    }
    async fn footer_data(&self) -> &State<FooterData> {
        self.guard().await.unwrap()
    }
}
