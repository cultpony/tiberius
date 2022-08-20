pub mod imageboard_type;

use axum::Router;
use axum_extra::routing::RouterExt;

pub fn setup_well_known(r: Router) -> Router {
    use imageboard_type::*;
    r.typed_get(imageboardtype)
        .typed_get(imageboardapiflavor)
        .typed_get(imageboardapiflavor_philomena_int)
        .typed_get(imageboardapiflavor_philomena_v1)
}
