use log::trace;
use maud::Markup;

use crate::{app::HTTPReq, error::TiberiusResult, pages::views};

#[get("/")]
pub async fn activity_get() -> TiberiusResult<Markup> {
    trace!("rendering activity main page");
    views::activity::html(todo!()).await
}
