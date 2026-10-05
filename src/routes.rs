use axum::{
    Router,
    middleware::from_fn_with_state,
    response::Redirect,
    routing::{get, post},
};

use crate::{
    handlers::{health, log},
    middleware::auth,
    state::AppState,
};

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/", get(|| async { Redirect::temporary("/health") }))
        .route("/health", get(health::check));

    let protected = Router::new()
        .route(
            "/log",
            post(log::add_entry)
                .get(log::show_log)
                .delete(log::clear_all),
        )
        .route("/log/last", get(log::show_last).delete(log::clear_last))
        .route_layer(from_fn_with_state(state.clone(), auth::require_token));

    public.merge(protected).with_state(state)
}
