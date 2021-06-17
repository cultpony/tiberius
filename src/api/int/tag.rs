use philomena_models::{Client, Tag};
use rocket::{State, serde::json::Json};

use crate::{app::HTTPReq, assets::{AssetLoaderRequestExt, SiteConfig}, error::TiberiusResult, request_helper::SafeSqlxRequestExt};

#[derive(serde::Deserialize)]
struct TagFetchQuery {
    pub ids: Vec<i64>,
}
#[derive(serde::Serialize)]
struct TagApiResponse {
    id: i64,
    name: String,
    images: i64,
    spoiler_image_uri: Option<String>,
}
#[derive(serde::Serialize)]
struct ApiResponse {
    tags: Vec<TagApiResponse>,
}

#[post("/tags/fetch", data = "<qs>")]
pub async fn fetch(site_config: &State<SiteConfig>, client: &State<Client>, qs: Json<TagFetchQuery>) -> TiberiusResult<Json<ApiResponse>> {
    let mut client = client.inner().clone();
    let qs: TagFetchQuery = qs.into_inner();
    let tags = Tag::get_many(&mut client, qs.ids).await?;
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
