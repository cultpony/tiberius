use maud::{html, Markup, DOCTYPE};

pub mod activity;
pub mod channels;
pub mod images;
pub mod tags;
pub mod session;

use anyhow::Result;

use crate::{app::HTTPReq, state::State};

pub async fn todo_page<S: Into<String>>(name: S) -> tide::Result {
    let name: String = name.into();
    let err: Result<Markup> = Err(anyhow::format_err!("route {:?} not implemented", name));
    err.res2res().await
}

pub async fn todo_page_fn(req: HTTPReq) -> tide::Result {
    log::error!("ROUTE {:?} WAS NOT IMPLEMENTED!", req.url().path());
    todo_page(req.url().path()).await
}

#[async_trait::async_trait]
pub trait ResToResponse {
    async fn res2res(&self) -> tide::Result;
}

#[async_trait::async_trait]
impl ResToResponse for Result<Markup> {
    async fn res2res(&self) -> tide::Result {
        match self {
            Ok(content) => Ok(tide::Response::builder(200)
                .content_type("text/html")
                .body(content.clone().into_string())
                .build()),
            Err(e) => Ok(tide::Response::builder(500)
                .content_type("text/html")
                .body(error_page(e).await.into_string())
                .build()),
        }
    }
}

pub async fn error_page(err: &anyhow::Error) -> Markup {
    let error = err.to_string();
    html! {
        DOCTYPE
        html {
            body {
                div.error {
                     h1.error-title { "An error occured while processing your request" }
                     main {
                         (error)
                     }
                }
            }
        }
    }
}
