use axum::Router;
use axum_extra::routing::RouterExt;

pub mod image;
pub mod oembed;
pub mod tag;

pub fn setup_api_int(r: Router) -> Router {
    r.typed_get(tag::fetch)
        .typed_get(oembed::fetch)
        .typed_post(image::favorite)
}
