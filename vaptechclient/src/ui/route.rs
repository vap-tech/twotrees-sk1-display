use crate::app::state::{MoveDistance, Page};
use crate::ui::intent::UiIntent;

/// Тупая таблица маршрутизации touch-событий.
///
/// Здесь нет доступа к AppState: одинаковый touch всегда превращается в один
/// и тот же intent, а блокировки и побочные эффекты решаются ниже.
pub fn route_touch(page: u8, component: u8) -> UiIntent {
    match (page, component) {
        // Home page. Номера компонентов взяты из снифов штатного дисплея.
        (0, 1) => UiIntent::Navigate(Page::Settings),
        (0, 2) => UiIntent::Navigate(Page::Files),
        (0, 3) => UiIntent::Navigate(Page::MoveTemp),

        // Move/Temp page
        (3, 1) => UiIntent::SelectMoveDistance(MoveDistance::Mm1),
        (3, 2) => UiIntent::SelectMoveDistance(MoveDistance::Mm10),
        (3, 3) => UiIntent::SelectMoveDistance(MoveDistance::Mm30),

        // Settings/System page
        (11, 1) => UiIntent::Navigate(Page::Network),
        (11, 2) => UiIntent::Navigate(Page::Calibration),

        // Calibration page
        (33, 1) => UiIntent::HomeAllAxes,

        // Print page
        (2, 1) => UiIntent::PausePrint,
        (2, 2) => UiIntent::StopPrint,
        (2, 6) => UiIntent::ToggleCaseLight,

        // Неизвестные touch не теряем: их видно в логах и можно дописать позже.
        _ => UiIntent::UnknownTouch { page, component },
    }
}

// Старое имя оставляем, чтобы наружный код мигрировал постепенно.
pub fn resolve_touch(page: u8, component: u8) -> UiIntent {
    route_touch(page, component)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_component_1_goes_to_settings() {
        assert_eq!(route_touch(0, 1), UiIntent::Navigate(Page::Settings));
    }

    #[test]
    fn home_component_2_goes_to_files() {
        assert_eq!(route_touch(0, 2), UiIntent::Navigate(Page::Files));
    }

    #[test]
    fn move_temp_distance_buttons() {
        assert_eq!(
            route_touch(3, 1),
            UiIntent::SelectMoveDistance(MoveDistance::Mm1)
        );
        assert_eq!(
            route_touch(3, 2),
            UiIntent::SelectMoveDistance(MoveDistance::Mm10)
        );
        assert_eq!(
            route_touch(3, 3),
            UiIntent::SelectMoveDistance(MoveDistance::Mm30)
        );
    }

    #[test]
    fn settings_component_2_goes_to_calibration() {
        assert_eq!(route_touch(11, 2), UiIntent::Navigate(Page::Calibration));
    }

    #[test]
    fn print_component_6_toggles_case_light() {
        assert_eq!(route_touch(2, 6), UiIntent::ToggleCaseLight);
    }

    #[test]
    fn unknown_touch_is_preserved() {
        assert_eq!(
            route_touch(99, 88),
            UiIntent::UnknownTouch {
                page: 99,
                component: 88,
            }
        );
    }

    #[test]
    fn intent_knows_if_it_is_navigation() {
        assert!(UiIntent::Navigate(Page::Settings).is_navigation());
        assert!(!UiIntent::HomeAllAxes.is_navigation());
    }

    #[test]
    fn stop_is_global_stop_intent() {
        assert!(UiIntent::StopPrint.is_global_stop());
        assert!(!UiIntent::PausePrint.is_global_stop());
    }
}
