use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query, FromRequestParts},
    Extension, Json, http::request::Parts,
};
use axum_extra::routing::TypedPath;
use serde::Deserialize;
use tiberius_core::{
    error::{TiberiusError, TiberiusResult},
    session::{SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Client, Tag, TagLike};

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

#[derive(TypedPath, Deserialize)]
#[typed_path("/tags/fetch")]
pub struct PathTagsFetch {}

#[derive(Deserialize)]
pub struct QueryTagsFetch {
    ids: Vec<i64>,
}

#[async_trait]
impl<S> FromRequestParts<S> for QueryTagsFetch
where
    S: Send + Sync,
{
    type Rejection = TiberiusError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let query = req.uri.query();
        let query = match query {
            Some(q) => q,
            None => {
                return Err(TiberiusError::Other(
                    "Missing required query parameter 'ids[]'".to_string(),
                ))
            }
        };
        Ok(serde_qs::from_str(query)?)
    }
}

#[instrument(skip(state, rstate))]
pub async fn fetch(
    _: PathTagsFetch,
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
    QueryTagsFetch { ids }: QueryTagsFetch,
) -> TiberiusResult<Json<ApiResponse>> {
    let mut client = state.get_db_client();
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
    Ok(Json(tags))
}
