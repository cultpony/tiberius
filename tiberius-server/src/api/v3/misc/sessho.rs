
use maud::html;
use rocket::form::Form;
use rocket::State;
use tiberius_core::app::PageTitle;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::{HtmlResponse, JsonResponse, TiberiusResponse};
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Image, User};

#[get("/api/v3/misc/session/handover")]
pub async fn session_handover_user(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let body = html!{
        form action=(rocket::uri!(session_handover).to_string()) method="POST" {
            div {
                b {
                    "Handover Status: "
                    (rstate.session.read().await.get_data(tiberius_core::session::philomena_plug::METADATA_KEY)?.unwrap_or("none".to_string()))
                }
                br;
            }
            label for="session_handover_secret" { "Session Handover Secret" }
            input type="text" name="session_handover_secret" id="session_handover_secret" placeholder="session_handover_secret" {}
            input type="submit" value="Submit" { "Submit" }
        }
    };
    let mut client = state.get_db_client().await?;
    let app = crate::pages::common::frontmatter::app(
        state,
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

#[post("/api/v3/misc/session/handover")]
pub async fn session_handover(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_>,
) -> TiberiusResult<JsonResponse> {
    todo!()
}