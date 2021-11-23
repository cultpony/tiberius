use rocket::response::Responder;
use rocket::{http::private::cookie::CookieBuilder, State};
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::SessionMode;
use tiberius_core::state::{Flash, TiberiusRequestState, TiberiusState};
use tracing::trace;

pub async fn get_flash<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_, T>,
) -> TiberiusResult<Vec<Flash>> {
    trace!("loading flash notices from session");
    let config: &Configuration = &state.config;
    let flash_cookie = rstate.cookie_jar.get(&config.flash_cookie);
    let flashlist: Option<Vec<Flash>> = flash_cookie
        .map(|x| serde_json::from_str(x.value()))
        .transpose()?;
    let mut flashlist = flashlist.unwrap_or_default();
    match rstate.flash().await {
        Some(f) => flashlist.push(f),
        _ => (),
    }
    if let Some(flash_cookie) = flash_cookie {
        rstate.cookie_jar.remove(flash_cookie.clone());
    }
    Ok(flashlist)
}

pub async fn put_flash<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_, T>,
    f: Flash,
) -> TiberiusResult<()> {
    trace!("putting flash into session");
    let mut flashlist = get_flash(state, rstate).await?;
    flashlist.push(f);
    let config: &Configuration = &state.config;
    let flashlist = serde_json::to_string(&flashlist)?;
    rstate
        .cookie_jar
        .add(CookieBuilder::new(config.flash_cookie.clone(), flashlist).finish());
    Ok(())
}
