use maud::Markup;
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::HtmlResponse;

#[get("/pages/<page>")]
pub async fn show(page: String) -> TiberiusResult<HtmlResponse> {
    todo!()
}
