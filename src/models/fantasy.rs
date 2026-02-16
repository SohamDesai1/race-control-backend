use serde::{Deserialize, Serialize};

pub const FANTASY_BUDGET_MILLIONS: i32 = 100;
pub const MIN_SALARY_MILLIONS: i32 = 5;
pub const MAX_SALARY_MILLIONS: i32 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyDriver {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub team_id: i32,
    pub salary: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyConstructor {
    pub id: i32,
    pub name: String,
    pub salary: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyTeamSelectionRequest {
    pub driver_1_id: i32,
    pub driver_2_id: i32,
    pub constructor_id: i32,
    pub booster_driver_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyTeamValidationResult {
    pub is_valid: bool,
    pub budget_used: i32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScorePreviewRequest {
    pub driver_1_finish_position: Option<i32>,
    pub driver_2_finish_position: Option<i32>,
    pub constructor_finish_position: Option<i32>,
    pub fastest_lap_driver_id: Option<i32>,
    pub selected_driver_1_id: i32,
    pub selected_driver_2_id: i32,
    pub booster_driver_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScorePreviewResponse {
    pub driver_1_points: i32,
    pub driver_2_points: i32,
    pub constructor_points: i32,
    pub fastest_lap_bonus: i32,
    pub booster_bonus: i32,
    pub total_points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverPricePreviewRequest {
    pub current_salary: i32,
    pub finish_position: Option<i32>,
    pub dnf: bool,
    pub got_fastest_lap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverPricePreviewResponse {
    pub current_salary: i32,
    pub change: i32,
    pub projected_salary: i32,
}

pub fn preview_driver_salary(request: &DriverPricePreviewRequest) -> DriverPricePreviewResponse {
    let change = get_driver_price_change(
        request.finish_position,
        request.dnf,
        request.got_fastest_lap,
    );
    let projected_salary = clamp_salary(request.current_salary + change);

    DriverPricePreviewResponse {
        current_salary: request.current_salary,
        change,
        projected_salary,
    }
}

pub fn get_finishing_points(position: Option<i32>) -> i32 {
    match position {
        Some(1) => 25,
        Some(2) => 18,
        Some(3) => 15,
        Some(4) => 12,
        Some(5) => 10,
        Some(6) => 8,
        Some(7) => 6,
        Some(8) => 4,
        Some(9) => 2,
        Some(10) => 1,
        _ => 0,
    }
}

pub fn clamp_salary(salary: i32) -> i32 {
    salary.clamp(MIN_SALARY_MILLIONS, MAX_SALARY_MILLIONS)
}

pub fn get_driver_price_change(position: Option<i32>, dnf: bool, got_fastest_lap: bool) -> i32 {
    let mut change = if dnf {
        -2
    } else {
        match position {
            Some(1) => 5,
            Some(2) => 3,
            Some(3) => 2,
            Some(4..=6) => 1,
            Some(7..=10) => 0,
            Some(11..) => -1,
            _ => 0,
        }
    };

    if position == Some(1) {
        change += 2;
    }
    if got_fastest_lap {
        change += 1;
    }

    change
}

pub fn validate_team_selection(
    request: &FantasyTeamSelectionRequest,
    drivers: &[FantasyDriver],
    constructors: &[FantasyConstructor],
) -> FantasyTeamValidationResult {
    let mut errors = Vec::new();

    if request.driver_1_id == request.driver_2_id {
        errors.push("Drivers must be unique".to_string());
    }

    if let Some(booster_driver_id) = request.booster_driver_id {
        if booster_driver_id != request.driver_1_id && booster_driver_id != request.driver_2_id {
            errors.push("Booster driver must be one of the selected drivers".to_string());
        }
    }

    let driver_1 = drivers.iter().find(|d| d.id == request.driver_1_id);
    let driver_2 = drivers.iter().find(|d| d.id == request.driver_2_id);
    let constructor = constructors.iter().find(|c| c.id == request.constructor_id);

    if driver_1.is_none() {
        errors.push("Driver 1 does not exist".to_string());
    }
    if driver_2.is_none() {
        errors.push("Driver 2 does not exist".to_string());
    }
    if constructor.is_none() {
        errors.push("Constructor does not exist".to_string());
    }

    let budget_used = driver_1.map_or(0, |d| d.salary)
        + driver_2.map_or(0, |d| d.salary)
        + constructor.map_or(0, |c| c.salary);

    if budget_used > FANTASY_BUDGET_MILLIONS {
        errors.push(format!(
            "Budget exceeded: used ${budget_used}M, limit is ${FANTASY_BUDGET_MILLIONS}M"
        ));
    }

    FantasyTeamValidationResult {
        is_valid: errors.is_empty(),
        budget_used,
        errors,
    }
}

pub fn preview_team_score(request: &FantasyScorePreviewRequest) -> FantasyScorePreviewResponse {
    let driver_1_points = get_finishing_points(request.driver_1_finish_position);
    let driver_2_points = get_finishing_points(request.driver_2_finish_position);
    let constructor_points = get_finishing_points(request.constructor_finish_position);

    let fastest_lap_bonus = if request.fastest_lap_driver_id == Some(request.selected_driver_1_id)
        || request.fastest_lap_driver_id == Some(request.selected_driver_2_id)
    {
        5
    } else {
        0
    };

    let booster_bonus = match request.booster_driver_id {
        Some(driver_id) if driver_id == request.selected_driver_1_id => driver_1_points,
        Some(driver_id) if driver_id == request.selected_driver_2_id => driver_2_points,
        _ => 0,
    };

    let total_points =
        driver_1_points + driver_2_points + constructor_points + fastest_lap_bonus + booster_bonus;

    FantasyScorePreviewResponse {
        driver_1_points,
        driver_2_points,
        constructor_points,
        fastest_lap_bonus,
        booster_bonus,
        total_points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finishing_points_table_is_correct() {
        assert_eq!(get_finishing_points(Some(1)), 25);
        assert_eq!(get_finishing_points(Some(10)), 1);
        assert_eq!(get_finishing_points(Some(11)), 0);
        assert_eq!(get_finishing_points(None), 0);
    }

    #[test]
    fn team_validation_rejects_overspend_and_duplicate_driver() {
        let drivers = vec![
            FantasyDriver {
                id: 1,
                name: "A".to_string(),
                code: "AAA".to_string(),
                team_id: 1,
                salary: 49,
            },
            FantasyDriver {
                id: 2,
                name: "B".to_string(),
                code: "BBB".to_string(),
                team_id: 2,
                salary: 49,
            },
        ];

        let constructors = vec![FantasyConstructor {
            id: 1,
            name: "C".to_string(),
            salary: 10,
        }];

        let result = validate_team_selection(
            &FantasyTeamSelectionRequest {
                driver_1_id: 1,
                driver_2_id: 1,
                constructor_id: 1,
                booster_driver_id: Some(2),
            },
            &drivers,
            &constructors,
        );

        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Drivers must be unique")));
    }
}
