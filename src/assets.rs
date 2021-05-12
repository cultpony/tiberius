use anyhow::Result;
use async_std::sync::RwLock;
use tide::Request;

use crate::{
    app::HTTPReq, config::Configuration, pages::common::frontmatter::FooterData, state::State,
};

#[derive(rust_embed::RustEmbed)]
#[folder = "res/assets-build/"]
#[prefix = "/static/"]
pub struct Assets;

pub async fn serve_asset(req: HTTPReq) -> tide::Result {
    let path = req.url().path();
    return_asset(path).await
}

pub async fn serve_topfile(req: HTTPReq) -> tide::Result {
    let path = match req.url().path() {
        "/favicon.ico" => "/static/favicon.ico",
        "/favicon.svg" => "/static/favicon.svg",
        "/robots.txt" => "/static/robots.txt",
        _ => {
            return Ok(tide::Response::builder(404)
                .content_type(tide::http::mime::PLAIN)
                .body("not found")
                .build())
        }
    };
    return_asset(path).await
}

pub async fn return_asset(path: &str) -> tide::Result {
    let file = Assets::get(path);
    Ok(match file {
        None => tide::Response::builder(404)
            .content_type(tide::http::mime::PLAIN)
            .body("not found")
            .build(),
        Some(file) => {
            let content_type = new_mime_guess::from_path(path);
            let content_type = content_type.first();
            let content_type = match content_type {
                None => tide::http::mime::PLAIN.essence().to_string(),
                Some(t) => t.essence_str().to_string(),
            };
            tide::Response::builder(200)
                .content_type(&*content_type)
                .body(&file[..])
                .build()
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
    pub fn new(c: &Configuration) -> Result<Self> {
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

#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for AssetLoader {
    async fn handle(&self, mut req: Request<State>, next: tide::Next<'_, State>) -> tide::Result {
        let data: FooterData = self.data.read().await.clone();
        let siteconf: SiteConfig = self.siteconf.read().await.clone();
        req.set_ext(data);
        req.set_ext(siteconf);
        let res = next.run(req).await;
        Ok(res)
    }
}

pub trait AssetLoaderRequestExt {
    fn site_config(&self) -> &SiteConfig;
    fn footer_data(&self) -> &FooterData;
}

impl AssetLoaderRequestExt for Request<State> {
    fn site_config(&self) -> &SiteConfig {
        self.ext::<SiteConfig>()
            .expect("SiteConfig expected but not in connection")
    }
    fn footer_data(&self) -> &FooterData {
        self.ext::<FooterData>()
            .expect("FooterData expected but not in connection")
    }
}
