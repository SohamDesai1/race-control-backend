use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;
use serde_json::json;

use crate::{
    models::fantasy::{
        preview_driver_salary, preview_team_score, validate_team_selection,
        DriverPricePreviewRequest, FantasyConstructor, FantasyDriver, FantasyScorePreviewRequest,
        FantasyTeamSelectionRequest, FANTASY_BUDGET_MILLIONS, MAX_SALARY_MILLIONS,
        MIN_SALARY_MILLIONS,
    },
    utils::state::AppState,
};

fn seeded_drivers() -> Vec<FantasyDriver> {
    vec![
        FantasyDriver {
            id: 1,
            name: "Max Verstappen".to_string(),
            code: "VER".to_string(),
            team_id: 1,
            salary: 37,
        },
        FantasyDriver {
            id: 2,
            name: "Lando Norris".to_string(),
            code: "NOR".to_string(),
            team_id: 2,
            salary: 30,
        },
        FantasyDriver {
            id: 3,
            name: "Charles Leclerc".to_string(),
            code: "LEC".to_string(),
            team_id: 3,
            salary: 28,
        },
        FantasyDriver {
            id: 4,
            name: "Oscar Piastri".to_string(),
            code: "PIA".to_string(),
            team_id: 2,
            salary: 27,
        },
    ]
}

fn seeded_constructors() -> Vec<FantasyConstructor> {
    vec![
        FantasyConstructor {
            id: 1,
            name: "Red Bull".to_string(),
            salary: 32,
        },
        FantasyConstructor {
            id: 2,
            name: "McLaren".to_string(),
            salary: 30,
        },
        FantasyConstructor {
            id: 3,
            name: "Ferrari".to_string(),
            salary: 31,
        },
    ]
}

pub async fn get_fantasy_catalog(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "budget_millions": FANTASY_BUDGET_MILLIONS,
            "drivers": seeded_drivers(),
            "constructors": seeded_constructors(),
            "note": "Seeded data for API scaffolding. Replace with DB-backed reads in Phase 1."
        })),
    )
        .into_response()
}

pub async fn validate_fantasy_team(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<FantasyTeamSelectionRequest>,
) -> impl IntoResponse {
    let validation = validate_team_selection(&request, &seeded_drivers(), &seeded_constructors());

    if validation.is_valid {
        (StatusCode::OK, Json(json!({"validation": validation}))).into_response()
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"validation": validation})),
        )
            .into_response()
    }
}

pub async fn preview_fantasy_score(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<FantasyScorePreviewRequest>,
) -> impl IntoResponse {
    let score = preview_team_score(&request);
    (StatusCode::OK, Json(json!({"score": score}))).into_response()
}

pub async fn preview_driver_price(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<DriverPricePreviewRequest>,
) -> impl IntoResponse {
    let preview = preview_driver_salary(&request);
    (
        StatusCode::OK,
        Json(json!({
            "salary_limits_millions": {
                "min": MIN_SALARY_MILLIONS,
                "max": MAX_SALARY_MILLIONS
            },
            "preview": preview
        })),
    )
        .into_response()
}
