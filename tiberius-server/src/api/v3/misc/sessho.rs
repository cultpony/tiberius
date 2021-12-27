use maud::html;
use rocket::form::Form;
use rocket::State;
use tiberius_core::app::PageTitle;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::{HtmlResponse, JsonResponse, TiberiusResponse};
use tiberius_core::session::{SessionMode, Unauthenticated};
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Image, User};

#[get("/api/v3/misc/session/handover")]
pub async fn session_handover_user(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client().await?;
    let body = html! {
        div {
            p { b {
                "Handover Status: "
                (rstate.session.read().await.get_data(tiberius_core::session::philomena_plug::METADATA_KEY)?.unwrap_or("none".to_string()))
            } }
            p {
                "Login Stats: "
                (format!("{:?}", rstate.session.read().await.get_user(&mut client).await?.map(|x| x.id())))
            }
            br;
        }
    };
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