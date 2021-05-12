use crate::app::HTTPReq;
use anyhow::Result;
use either::Either;
use std::str::FromStr;
use tide::http::headers::HeaderValue;

pub fn content_security_policy(
    req: &HTTPReq,
) -> std::result::Result<HeaderValue, tide::http::Error> {
    use csp::*;
    let state = req.state();
    let config = &state.config;
    let static_host = config.static_host(req);
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
    HeaderValue::from_str(&csp.to_string())
}

pub fn get_user_agent(req: &HTTPReq) -> Result<Option<String>> {
    Ok(req
        .header(tide::http::headers::USER_AGENT)
        .map(|x| x.get(0).map(|x| x.to_string()))
        .flatten())
}

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
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return Ok(Self {
            query: Some(either::Left(s.to_string())),
        });
    }
}
