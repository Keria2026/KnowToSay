/// SvelteKit routes on `/`; an explicit `/index.html` becomes a missing route.
pub const QUICK_SEARCH_PATH: &str = "/?quickSearch=1";

/// Keep client windows inside the available logical monitor dimensions.
pub fn fit_window_size(target: (f64, f64), screen: (f64, f64)) -> (f64, f64) {
    (
        target.0.min((screen.0 - 32.0).max(1.0)),
        target.1.min((screen.1 - 80.0).max(1.0)),
    )
}

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
    fn search_window_resolves_to_the_sveltekit_root_in_dev_and_packaged_apps() {
        for base in [
            "http://127.0.0.1:1420/",
            "http://tauri.localhost/",
            "tauri://localhost/",
        ] {
            let url = reqwest::Url::parse(base)
                .unwrap()
                .join(QUICK_SEARCH_PATH)
                .unwrap();
            assert_eq!(url.path(), "/");
            assert_eq!(url.query(), Some("quickSearch=1"));
        }
    }
    #[test]
    fn windows_fit_small_and_scaled_displays() {
        assert_eq!(
            fit_window_size((720.0, 560.0), (1920.0, 1080.0)),
            (720.0, 560.0)
        );
        assert_eq!(
            fit_window_size((720.0, 560.0), (640.0, 480.0)),
            (608.0, 400.0)
        );
        assert_eq!(
            fit_window_size((400.0, 740.0), (1280.0, 720.0)),
            (400.0, 640.0)
        );
    }
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
