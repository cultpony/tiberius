use philomena_models::Tag;
use tide::http::mime::JSON;

use crate::{app::HTTPReq, assets::AssetLoaderRequestExt, request_helper::SafeSqlxRequestExt};


pub async fn fetch(req: HTTPReq) -> tide::Result {
    let mut client = req.get_db_client().await?;
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
    let qs = req.query::<TagFetchQuery>()?;
    let tags = Tag::get_many(&mut client, qs.ids).await?;
    let tags: Vec<TagApiResponse> = tags.iter().map(|x| {
        let spoiler_img = match &x.image {
            Some(image) => Some(format!("{}/{}", req.site_config().tag_url_root(), image)),
            None => None,
        };
        TagApiResponse{
            id: x.id as i64,
            name: x.full_name(),
            images: x.images_count as i64,
            spoiler_image_uri: spoiler_img,
        }
    }).collect();
    let tags = ApiResponse{
        tags,
    };
    Ok(tide::Response::builder(200).content_type(JSON).body(serde_json::to_string(&tags)?).build())
}