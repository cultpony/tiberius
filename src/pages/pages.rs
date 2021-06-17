use maud::{Markup, PreEscaped, html};

pub mod activity;
pub mod channels;
pub mod images;
//pub mod session;
//pub mod tags;

use rocket::Request;

use crate::error::TiberiusResult;

pub async fn todo_page<S: Into<String>>(name: S) -> TiberiusResult<Markup> {
    let name: String = name.into();
    let err: TiberiusResult<Markup> = Err(anyhow::format_err!("route {:?} not implemented", name));
    err
}

pub async fn todo_page_fn(req: Request<'_>) -> TiberiusResult<Markup> {
    log::error!("ROUTE {:?} WAS NOT IMPLEMENTED!", req.url().path());
    todo_page(req.url().path()).await
}

pub async fn error_page(err: &anyhow::Error) -> Markup {
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
    error_page(anyhow::format_err!("The page located under {:?} could not be found, have you tried looking in Celestia's secret stash?"))
}
