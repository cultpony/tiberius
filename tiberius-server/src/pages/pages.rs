use maud::{html, Markup, PreEscaped};
use tiberius_core::error::{TiberiusError, TiberiusResult};

pub mod activity;
pub mod apikeys;
pub mod blog;
pub mod channels;
pub mod errors;
pub mod images;
pub mod session;
pub mod tags;

use rocket::Request;

pub async fn todo_page<S: Into<String>>(name: S) -> TiberiusResult<Markup> {
    let name: String = name.into();
    let err: TiberiusResult<Markup> = Err(TiberiusError::RouteNotFound(name));
    err
}

pub async fn todo_page_fn(req: Request<'_>) -> TiberiusResult<Markup> {
    tracing::error!("ROUTE {:?} WAS NOT IMPLEMENTED!", req.uri().path());
    todo_page(req.uri().path().to_string()).await
}

pub async fn error_page(err: &TiberiusError) -> Markup {
    let error = err.to_string();
    html! {
        (maud::DOCTYPE)
        html {
            head {
                style {
                    (PreEscaped(r#"
                    "#))
                }
            }
            body {
                div.error.wrapper {
                     h1.error.title { "An error occured while processing your request" }
                     main {
                         (error)
                     }
                }
            }
        }
    }
}

pub async fn not_found_page(url: &str) -> Markup {
    error_page(&TiberiusError::PageNotFound(url.to_string())).await
}