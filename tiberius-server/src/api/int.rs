use axum::Router;
use axum_extra::routing::RouterExt;
use tiberius_core::state::TiberiusState;

pub mod image;
pub mod oembed;
pub mod tag;

pub fn setup_api_int(r: Router<TiberiusState>) -> Router<TiberiusState> {
    r.typed_get(tag::fetch)
        .typed_get(oembed::fetch)
        .typed_post(image::favorite)
}
