use axum::{
    extract::Query,
    http::{HeaderMap, HeaderValue},
    Extension, Router,
};
use axum_extra::routing::{RouterExt, TypedPath};
use maud::Markup;
use tiberius_core::{
    error::TiberiusResult,
    request_helper::{HtmlResponse, JsonResponse, TiberiusResponse},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Client, Tag, TagLike};
use crate::set_scope_tx;

use crate::pages::todo_page;

pub fn tags_pages(r: Router) -> Router {
    r.typed_get(list_tags)
        .typed_get(show_tag)
        .typed_get(show_tag_by_name)
        .typed_post(edit_tag)
        .typed_get(tag_changes)
        .typed_get(usage)
        .typed_post(reindex)
        .typed_post(alias)
        .typed_post(autocomplete)
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags")]
pub struct PathTagsListTags {}

#[tracing::instrument]
pub async fn list_tags(_: PathTagsListTags) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("list_tags").await?.into_string(),
    })
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id")]
pub struct PathTagsShowTag {
    pub tag_id: either::Either<i64, String>,
}

#[tracing::instrument]
pub async fn show_tag(PathTagsShowTag { tag_id }: PathTagsShowTag) -> TiberiusResult<HtmlResponse> {
    set_scope_tx!("GET /tags/:tag_id");
    Ok(HtmlResponse {
        content: todo_page("show_tags").await?.into_string(),
    })
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/watch")]
pub struct PathTagsWatchTag {
    pub tag_id: u64,
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/spoiler")]
pub struct PathTagsSpoilerTag {
    pub tag_id: u64,
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/hide")]
pub struct PathTagsHideTag {
    pub tag_id: u64,
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/by_name/:tag")]
pub struct PathTagsByNameShowTag {
    pub tag: String,
}

#[tracing::instrument]
pub async fn show_tag_by_name(
    PathTagsByNameShowTag { tag }: PathTagsByNameShowTag,
) -> TiberiusResult<HtmlResponse> {
    todo!()
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/edit")]
pub struct TagsByIdEditTag {
    tag_id: i64,
}

#[tracing::instrument]
pub async fn edit_tag(TagsByIdEditTag { tag_id }: TagsByIdEditTag) -> TiberiusResult<String> {
    Ok(todo_page("edit_tags").await?.into_string())
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/changes")]
pub struct TagsByIdTagChanges {
    tag_id: i64,
}

#[tracing::instrument]
pub async fn tag_changes(
    TagsByIdTagChanges { tag_id }: TagsByIdTagChanges,
) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("tag_tags").await?.into_string(),
    })
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/usage")]
pub struct TagsByIdTagUsage {
    tag_id: i64,
}

#[tracing::instrument]
pub async fn usage(TagsByIdTagUsage { tag_id }: TagsByIdTagUsage) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("usage_tags").await?.into_string(),
    })
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/reindex")]
pub struct TagsByIdTagReindex {
    tag_id: i64,
}

#[tracing::instrument]
pub async fn reindex(
    TagsByIdTagReindex { tag_id }: TagsByIdTagReindex,
) -> TiberiusResult<TiberiusResponse<()>> {
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: todo_page("usage_tags").await?.into_string(),
    }))
}
#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/:tag_id/alias")]
pub struct TagsByIdTagAlias {
    tag_id: i64,
}

#[tracing::instrument]
pub async fn alias(
    TagsByIdTagAlias { tag_id }: TagsByIdTagAlias,
) -> TiberiusResult<TiberiusResponse<()>> {
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: todo_page("usage_tags").await?.into_string(),
    }))
}

#[derive(serde::Serialize)]
struct AutocompleteResponse {
    label: String,
    value: String,
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/tags/autocomplete")]
pub struct TagsAutocompletePath {}

#[derive(serde::Deserialize)]
pub struct TagsAutocompleteQuery {
    term: String,
}

#[instrument(skip(state))]
pub async fn autocomplete(
    _: TagsAutocompletePath,
    Extension(state): Extension<TiberiusState>,
    Query(TagsAutocompleteQuery { term }): Query<TagsAutocompleteQuery>,
) -> TiberiusResult<JsonResponse> {
    let mut client = state.get_db_client();
    let autocompleted_tags = Tag::autocomplete(&mut client, &term).await?;
    let count = autocompleted_tags.0;
    tracing::debug!("Autocomplete found {} tags for {:?}", count, term);
    let autocompleted_tags: Vec<AutocompleteResponse> = autocompleted_tags
        .1
        .iter()
        .map(|tag| AutocompleteResponse {
            label: format!("{} ({})", tag.full_name(), tag.images_count),
            value: tag.full_name(),
        })
        .collect();
    let act = serde_json::to_value(autocompleted_tags)?;
    let mut hm = HeaderMap::new();
    hm.insert(
        "X-Autocomplete-Count",
        HeaderValue::from_str(&count.to_string()).unwrap(),
    );
    Ok(JsonResponse {
        content: act,
        headers: hm,
    })
}
