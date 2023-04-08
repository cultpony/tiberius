pub mod imageboard_type;

use axum::Router;
use axum_extra::routing::RouterExt;
use tiberius_core::state::TiberiusState;

pub fn setup_well_known(r: Router<TiberiusState>) -> Router<TiberiusState> {
    use imageboard_type::*;
    r.typed_get(imageboardtype)
        .typed_get(imageboardapiflavor)
        .typed_get(imageboardapiflavor_philomena_int)
        .typed_get(imageboardapiflavor_philomena_v1)
}
