pub fn map_session_name(external: &str) -> Option<&'static str> {
    match external {
        "Practice 1" => Some("FirstPractice"),
        "Practice 2" => Some("SecondPractice"),
        "Practice 3" => Some("ThirdPractice"),
        "Qualifying" => Some("Qualifying"),
        "Race" => Some("Race"),
        _ => None,
    }
}