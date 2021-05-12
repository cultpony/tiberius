use maud::Markup;
use rocket::State;
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{HtmlResponse, JsonResponse, TiberiusResponse};
use tiberius_core::state::{Flash, TiberiusRequestState, TiberiusState};
use tiberius_models::Tag;

use crate::pages::todo_page;

#[get("/tags")]
pub async fn list_tags() -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("list_tags").await?.into_string(),
    })
}

#[get("/tags/<tag_id>")]
pub async fn show_tag(tag_id: i64) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("show_tags").await?.into_string(),
    })
}

#[get("/tags/by_name/<tag>")]
pub async fn show_tag_by_name(tag: String) -> TiberiusResult<HtmlResponse> {
    todo!()
}

#[get("/tags/<tag_id>/edit")]
pub async fn edit_tag(tag_id: i64) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("edit_tags").await?.into_string(),
    })
}

#[get("/tags/<tag_id>/changes")]
pub async fn tag_changes(tag_id: i64) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("tag_tags").await?.into_string(),
    })
}

#[get("/tags/<tag_id>/usage")]
pub async fn usage(tag_id: i64) -> TiberiusResult<HtmlResponse> {
    Ok(HtmlResponse {
        content: todo_page("usage_tags").await?.into_string(),
    })
}

#[post("/tags/<tag_id>/reindex")]
pub async fn reindex(tag_id: i64) -> TiberiusResult<TiberiusResponse<()>> {
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: todo_page("usage_tags").await?.into_string(),
    }))
}

#[post("/tags/<tag_id>/alias")]
pub async fn alias(tag_id: i64) -> TiberiusResult<TiberiusResponse<()>> {
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: todo_page("usage_tags").await?.into_string(),
    }))
}

#[derive(serde::Serialize)]
struct AutocompleteResponse {
    label: String,
    value: String,
}

#[get("/tags/autocomplete?<term>")]
pub async fn autocomplete(
    state: &State<TiberiusState>,
    term: String,
) -> TiberiusResult<JsonResponse> {
    let mut client = state.get_db_client().await?;
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
    Ok(JsonResponse {
        content: act,
        headers: rocket::http::Header::new("X-Autocomplete-Count", count.to_string()),
    })
}
