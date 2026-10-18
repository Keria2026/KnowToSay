/// Screen coordinates are physical pixels, including negative monitor origins.
pub fn active_edge(
    point: (i32, i32),
    bounds: (i32, i32, i32, i32),
    enabled: &[String],
) -> Option<&'static str> {
    let (x, y) = point;
    let (left, top, right, bottom) = bounds;
    [
        ("left", x >= left && x <= left + 1),
        ("right", x < right && x >= right - 2),
        ("top", y >= top && y <= top + 1),
        ("bottom", y < bottom && y >= bottom - 2),
    ]
    .into_iter()
    .find(|(edge, hit)| *hit && enabled.iter().any(|e| e == edge))
    .map(|(edge, _)| edge)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_edges_and_corners_are_respected() {
        let top = vec!["top".into()];
        assert_eq!(active_edge((0, 0), (0, 0, 1920, 1080), &top), Some("top"));
        assert_eq!(active_edge((1919, 500), (0, 0, 1920, 1080), &top), None);
        assert_eq!(active_edge((500, 500), (0, 0, 1920, 1080), &top), None);
    }
    #[test]
    fn negative_monitor_coordinates_work() {
        let enabled = vec!["right".into()];
        assert_eq!(
            active_edge((-1, 200), (-1920, 0, 0, 1080), &enabled),
            Some("right")
        );
        assert_eq!(
            active_edge((-1920, 200), (-1920, 0, 0, 1080), &enabled),
            None
        );
    }
}
