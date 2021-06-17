use maud::Markup;

use crate::error::TiberiusResult;

#[get("/<image>")]
pub async fn show_image(image: u64) -> TiberiusResult<Markup> {
    todo!()
}

#[post("/image")]
pub async fn new_image() -> TiberiusResult<Markup> {
    todo!()
}
