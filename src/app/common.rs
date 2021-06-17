use crate::{app::HTTPReq, config::Configuration, error::{TiberiusError, TiberiusResult}};
use either::Either;
use rocket::{Request, Response, State, fairing::{Fairing, Info, Kind}, http::Header};
use std::str::FromStr;

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
        let state: &State<crate::state::State> = req.guard().await.succeeded().unwrap();
        let config = state.config();
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
        let h = Header::new("Content-Security-Policy".to_string(), &csp.to_string());
        res.set_header(h);
    }
}

pub fn get_user_agent(req: &HTTPReq) -> TiberiusResult<Option<String>> {
    Ok(req
        .headers()
        .get_one(rocket::http::hyper::header::USER_AGENT.as_str())
        .map(|x| x.to_string()))
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
    type Err = TiberiusError;

    fn from_str(s: &str) -> TiberiusResult<Self> {
        return Ok(Self {
            query: Some(either::Left(s.to_string())),
        });
    }
}
