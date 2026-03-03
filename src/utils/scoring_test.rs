#[cfg(test)]
mod tests {
    const DRIVER_POINTS: [i32; 10] = [25, 18, 15, 12, 10, 8, 6, 4, 2, 1];
    const FASTEST_LAP_BONUS: i32 = 5;

    fn get_finishing_points(position: i32) -> i32 {
        if position < 1 {
            return 0;
        }
        let idx = (position - 1) as usize;
        if idx < DRIVER_POINTS.len() {
            DRIVER_POINTS[idx]
        } else {
            0
        }
    }

    #[test]
    fn test_get_finishing_points() {
        assert_eq!(get_finishing_points(1), 25);
        assert_eq!(get_finishing_points(2), 18);
        assert_eq!(get_finishing_points(3), 15);
        assert_eq!(get_finishing_points(4), 12);
        assert_eq!(get_finishing_points(5), 10);
        assert_eq!(get_finishing_points(6), 8);
        assert_eq!(get_finishing_points(7), 6);
        assert_eq!(get_finishing_points(8), 4);
        assert_eq!(get_finishing_points(9), 2);
        assert_eq!(get_finishing_points(10), 1);
        assert_eq!(get_finishing_points(11), 0);
        assert_eq!(get_finishing_points(20), 0);
        assert_eq!(get_finishing_points(0), 0);
    }

    #[test]
    fn test_points_calculation_1st_place() {
        assert_eq!(get_finishing_points(1), 25);
    }

    #[test]
    fn test_points_calculation_2nd_place() {
        assert_eq!(get_finishing_points(2), 18);
    }

    #[test]
    fn test_points_calculation_outside_top_10() {
        assert_eq!(get_finishing_points(11), 0);
        assert_eq!(get_finishing_points(15), 0);
        assert_eq!(get_finishing_points(20), 0);
    }

    #[test]
    fn test_fastest_lap_bonus() {
        assert_eq!(FASTEST_LAP_BONUS, 5);
    }
}
