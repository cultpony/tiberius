use rocket::response::content::Json;
use tiberius_core::error::TiberiusResult;

#[get("/oembed")]
pub async fn fetch() -> TiberiusResult<Json<()>> {
    todo!()
}
