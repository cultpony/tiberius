use tiberius_core::error::TiberiusResult;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Client, Tag};
use rocket::{State, serde::json::Json};

#[derive(serde::Deserialize)]
pub struct TagFetchQuery {
    pub ids: Vec<i64>,
}
#[derive(serde::Serialize)]
pub struct TagApiResponse {
    id: i64,
    name: String,
    images: i64,
    spoiler_image_uri: Option<String>,
}
#[derive(serde::Serialize)]
pub struct ApiResponse {
    tags: Vec<TagApiResponse>,
}

#[get("/tags/fetch?<ids>")]
pub async fn fetch(state: &State<TiberiusState>, rstate: TiberiusRequestState<'_>, ids: Vec<i64>) -> TiberiusResult<Json<ApiResponse>> {
    let mut client = state.get_db_client().await?;
    let site_config = state.site_config();
    let tags = Tag::get_many(&mut client, ids).await?;
    let tags: Vec<TagApiResponse> = tags
        .iter()
        .map(|x| {
            let spoiler_img = match &x.image {
                Some(image) => Some(format!("{}/{}", site_config.tag_url_root(), image)),
                None => None,
            };
            TagApiResponse {
                id: x.id as i64,
                name: x.full_name(),
                images: x.images_count as i64,
                spoiler_image_uri: spoiler_img,
            }
        })
        .collect();
    let tags = ApiResponse { tags };
    Ok(Json::from(tags))
}
