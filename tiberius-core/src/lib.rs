//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(deprecated)]

#[macro_use]
extern crate tracing;

use async_trait::async_trait;
use axum::{extract::State, http::Uri, middleware::FromFnLayer};
use axum_extra::routing::TypedPath;
use reqwest::{header::HeaderMap, Proxy};
use state::TiberiusState;
use tiberius_dependencies::{
    axum::{
        self,
        extract::FromRequest,
        headers::{HeaderMapExt, UserAgent},
        http::{HeaderValue, Request},
        middleware::{self, Next},
        response::Response,
        Extension,
    },
    reqwest, serde_qs,
    tower::{Layer, ServiceBuilder},
};
use tracing::trace;

use crate::{config::Configuration, error::TiberiusResult};

pub mod acl;
pub mod app;
pub mod assets;
pub mod config;
pub mod error;
pub mod footer;
pub mod links;
pub mod nodeid;
pub mod request_helper;
pub mod session;
pub mod state;

// How long to hold Subtext in Cache while they're being used
pub const PAGE_SUBTEXT_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
// How long to hold Subtext in Cache while they're not being used
pub const PAGE_SUBTEXT_CACHE_TTI: Duration = Duration::from_secs(60);

pub const PAGE_SUBTEXT_CACHE_SIZE: u64 = 1_000;
pub const PAGE_SUBTEXT_CACHE_START_SIZE: usize = 100;

pub const CSD_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
pub const CSD_CACHE_TTI: Duration = Duration::from_secs(60);
pub const CSD_CACHE_SIZE: u64 = 10_000;
pub const CSD_CACHE_START_SIZE: usize = 100;

pub const COMMENT_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
pub const COMMENT_CACHE_TTI: Duration = Duration::from_secs(60);
pub const COMMENT_CACHE_SIZE: u64 = 100;
pub const COMMENT_CACHE_START_SIZE: usize = 10;

pub use nodeid::NodeId;

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

#[derive(Clone, Default)]
pub struct CSPHeader {
    pub static_host: Option<String>,
    pub camo_host: Option<String>,
}

impl CSPHeader {
    fn header_name(&self) -> &'static str {
        "Content-Security-Policy"
    }
    fn header_value(&self) -> HeaderValue {
        use csp::*;
        let default_src = match &self.static_host {
            Some(static_host) => Sources::new_with(Source::Self_).add(Source::Host(static_host)),
            None => Sources::new_with(Source::Self_),
        };
        let style_src = match &self.static_host {
            Some(static_host) => Sources::new_with(Source::Self_)
                .add(Source::UnsafeInline)
                .add(Source::Host(static_host)),
            None => Sources::new_with(Source::Self_).add(Source::UnsafeInline),
        };
        let img_src = {
            let s = Sources::new_with(Source::Self_)
                .add(Source::Scheme("data"))
                // Picarto CDN
                .add(Source::Host("images.picarto.tv"))
                .add(Source::Host("*.picarto.tv"));
            let s = match &self.static_host {
                Some(static_host) => s.add(Source::Host(static_host)),
                None => s,
            };
            let s = match &self.camo_host {
                Some(v) => s.add(Source::Host(v)),
                None => s,
            };
            s
        };
        let csp = CSP::new()
            .add(Directive::DefaultSrc(default_src))
            .add(Directive::ObjectSrc(Sources::new()))
            .add(Directive::FrameAncestors(Sources::new()))
            .add(Directive::FrameSrc(Sources::new()))
            .add(Directive::FormAction(Sources::new_with(Source::Self_)))
            .add(Directive::ManifestSrc(Sources::new_with(Source::Self_)))
            .add(Directive::StyleSrc(style_src))
            .add(Directive::ImgSrc(img_src))
            .add(Directive::BlockAllMixedContent);
        HeaderValue::from_str(&csp.to_string()).unwrap()
    }
}

pub async fn csp_header<B>(
    State(state): State<TiberiusState>,
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let csp_header = state.csp();
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert(csp_header.header_name(), csp_header.header_value());
    resp
}

pub fn get_user_agent<T: SessionMode>(
    rstate: TiberiusRequestState<T>,
) -> TiberiusResult<Option<UserAgent>> {
    Ok(rstate.headers.typed_get::<UserAgent>())
}

use crate::{
    error::TiberiusError,
    session::{SessionMode, Unauthenticated},
    state::TiberiusRequestState,
};
use either::Either;
use std::{str::FromStr, time::Duration};

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
        Ok(Self {
            query: Some(either::Left(s.to_string())),
        })
    }
}

#[derive(Debug, Clone)]
pub enum LayoutClass {
    Wide,
    Narrow,
    Other(String),
}

impl ToString for LayoutClass {
    fn to_string(&self) -> String {
        match self {
            Self::Wide => "layout--wide".to_string(),
            Self::Narrow => "layout--narrow".to_string(),
            Self::Other(v) => v.clone(),
        }
    }
}

pub trait PathQuery: serde::Serialize {}

pub fn path_and_query<T: TypedPath, Q: PathQuery>(
    path: T,
    query: Option<&Q>,
) -> TiberiusResult<Uri> {
    let path = path.to_uri();
    let query = match query {
        Some(query) => Some(serde_qs::to_string(query)?),
        None => None,
    };
    let p_and_q = path.path_and_query().map(|x| match query {
        Some(query) => {
            if !query.is_empty() {
                x.to_string() + "?" + query.as_str()
            } else {
                x.to_string()
            }
        }
        None => x.to_string(),
    });
    let builder = Uri::builder();
    let builder = match path.authority() {
        Some(auth) => builder.authority(auth.clone()),
        None => builder,
    };
    let builder = match p_and_q {
        Some(p_and_q) => builder.path_and_query(p_and_q),
        None => builder,
    };
    Ok(builder.build()?)
}
