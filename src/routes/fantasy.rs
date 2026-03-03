use crate::handlers::fantasy::*;
use crate::handlers::middleware::auth_middleware;
use crate::utils::state::AppState;
use axum::{
    extract::State,
    middleware::from_fn,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

pub fn fantasy_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let fantasy_router = Router::new()
        .route("/drivers", get(get_drivers))
        .route("/constructors", get(get_constructors))
        .route("/contests", get(get_user_contests).post(create_contest))
        .route("/contests/invite/{invite_code}", get(get_contest_by_invite))
        .route("/contests/invite/{invite_code}/join", post(join_contest))
        .route("/contests/{id}/leave", delete(leave_contest))
        .route("/contests/{id}", get(get_contest_details))
        .route("/contests/{id}/leaderboard", get(get_contest_leaderboard))
        .route("/race/{gp_id}", get(get_race_info))
        .route(
            "/contests/{gp_id}/team",
            get(get_team_for_gp)
                .post(create_or_update_team)
                .delete(delete_team),
        )
        .route("/contests/{gp_id}/booster", post(set_booster))
        .route("/leaderboard", get(get_global_leaderboard))
        .with_state(state.clone());

    fantasy_router.layer(from_fn(move |req, next| {
        auth_middleware(State(state.clone()), req, next)
    }))
}
