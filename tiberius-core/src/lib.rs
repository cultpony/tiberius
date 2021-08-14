//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(deprecated)]

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate rocket;

use reqwest::header::HeaderMap;
use reqwest::Proxy;
use tracing::trace;

use crate::config::Configuration;
use crate::error::TiberiusResult;

pub mod app;
pub mod assets;
pub mod config;
pub mod error;
pub mod footer;
pub mod request_helper;
pub mod session;
pub mod state;

pub fn http_client(config: &Configuration) -> TiberiusResult<reqwest::Client> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none());
    let client = if let Some(proxy) = &config.proxy {
        client.proxy(Proxy::all(proxy.clone())?)
    } else {
        client
    };
    Ok(client.default_headers(common_headers()).build()?)
}
pub struct StatelessPaths {}

impl StatelessPaths {
    pub fn contains(path: &str) -> bool {
        match path {
            "/favicon.svg" => true,
            "/favicon.ico" => true,
            "/robots.txt" => true,
            _ => path.starts_with("/static/") || path.starts_with("/img/"),
        }
    }
}

fn common_headers() -> HeaderMap {
    let mut hm = HeaderMap::new();
    let user_agent = format!("Mozilla/5.0 ({} v{})", package_name(), package_version());
    trace!("new user agent with value {}", user_agent);
    hm.append(reqwest::header::USER_AGENT, user_agent.parse().unwrap());
    hm
}

pub fn package_full() -> String {
    format!("{} v{}", package_name(), package_version())
}

pub const fn package_name() -> &'static str {
    const NAME: &str = env!("CARGO_PKG_NAME");
    NAME
}

pub const fn package_version() -> &'static str {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    VERSION
}

pub struct CSPHeader;

#[rocket::async_trait]
impl Fairing for CSPHeader {
    fn info(&self) -> Info {
        Info {
            name: "CSP Header Middleware",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        use csp::*;
        let state: &State<crate::state::TiberiusState> = req.guard().await.succeeded().unwrap();
        let rstate: TiberiusRequestState = req.guard().await.succeeded().unwrap();
        let config = state.config();
        let static_host = config.static_host(&rstate);
        let camo_host = config.camo_config().map(|x| x.0);
        let csp = CSP::new()
            .add(Directive::DefaultSrc(
                Sources::new_with(Source::Self_).add(Source::Host(&static_host)),
            ))
            .add(Directive::ObjectSrc(Sources::new()))
            .add(Directive::FrameAncestors(Sources::new()))
            .add(Directive::FrameSrc(Sources::new()))
            .add(Directive::FormAction(Sources::new_with(Source::Self_)))
            .add(Directive::ManifestSrc(Sources::new_with(Source::Self_)))
            .add(Directive::StyleSrc(
                Sources::new_with(Source::Self_)
                    .add(Source::UnsafeInline)
                    .add(Source::Host(&static_host)),
            ))
            .add(Directive::ImgSrc({
                let s = Sources::new_with(Source::Self_)
                    .add(Source::Scheme("data"))
                    .add(Source::Host(&static_host))
                    // Picarto CDN
                    .add(Source::Host("images.picarto.tv"))
                    .add(Source::Host("*.picarto.tv"));
                let s = match camo_host {
                    Some(v) => s.add(Source::Host(v)),
                    None => s,
                };
                s
            }))
            .add(Directive::BlockAllMixedContent);
        let h = Header::new("Content-Security-Policy".to_string(), csp.to_string());
        res.set_header(h);
    }
}

pub fn get_user_agent(rstate: &TiberiusRequestState<'_>) -> TiberiusResult<Option<String>> {
    Ok(rstate
        .headers
        .get_one(rocket::http::hyper::header::USER_AGENT.as_str())
        .map(|x| x.to_string()))
}

use crate::{error::TiberiusError, state::TiberiusRequestState};
use either::Either;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::Header,
    Request, Response, State,
};
use std::str::FromStr;

pub struct Query {
    query: Option<Either<String, QueryData>>,
}

pub struct QueryData();

impl Query {
    pub fn empty() -> Query {
        Self { query: None }
    }
}

impl std::string::ToString for Query {
    fn to_string(&self) -> String {
        match self.query.as_ref() {
            Some(either::Left(v)) => v.clone(),
            Some(either::Right(_)) => todo!("cannot convert query to string"),
            None => "".to_string(),
        }
    }
}

impl std::str::FromStr for Query {
    type Err = TiberiusError;

    fn from_str(s: &str) -> TiberiusResult<Self> {
        return Ok(Self {
            query: Some(either::Left(s.to_string())),
        });
    }
}

#[derive(Debug, Clone)]
pub enum LayoutClass {
    Narrow,
    Other(String),
}

impl ToString for LayoutClass {
    fn to_string(&self) -> String {
        match self {
            Self::Narrow => "layout--narrow".to_string(),
            Self::Other(v) => v.clone(),
        }
    }
}
