use super::ResToResponse;
use log::trace;
use anyhow::Result;
use maud::Markup;

use crate::{app::HTTPReq, pages::views};

#[get("/")]
pub async fn activity_get(req: HTTPReq) -> Result<Markup> {
    trace!("rendering activity main page");
    views::activity::html(req).await
}
