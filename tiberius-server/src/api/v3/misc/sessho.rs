use axum::{extract::State, Extension};
use axum_extra::routing::TypedPath;
use maud::html;
use serde::Deserialize;
use tiberius_core::{
    app::PageTitle,
    error::{TiberiusError, TiberiusResult},
    request_helper::{HtmlResponse, JsonResponse, TiberiusResponse},
    session::{SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Image, User};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v3/misc/session/handover")]
pub struct PathApiV3MiscSessionHandover {}

#[instrument(skip(state, rstate))]
pub async fn session_handover_user(
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client();
    let body = html! {
        div {
            p { b {
                "Handover Status: "
                (rstate.session().get_data(tiberius_core::session::philomena_plug::METADATA_KEY)?.unwrap_or("none".to_string()))
            } }
            p {
                "Login Stats: "
                (format!("{:?}", rstate.session().raw_user()))
            }
            br;
        }
    };
    let app = crate::templates::common::frontmatter::app(
        &state,
        &rstate,
        Some(PageTitle::from("API - Session Handover")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}
