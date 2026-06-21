use crate::app::state::Page;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    ChangePage(Page),

    HomeAllAxes,

    StartPrint,
    PausePrint,
    ResumePrint,
    StopPrint,

    LoadFilament,
    UnloadFilament,

    TogglePartFan,
    ToggleSideFan,
    ToggleFilterFan,

    MoveDistance1,
    MoveDistance10,
    MoveDistance30,

    UnknownTouch { page: u8, component: u8 },
}

pub fn resolve_touch(page: u8, component: u8) -> UiAction {
    match (page, component) {
        // Home page
        (0, 1) => UiAction::ChangePage(Page::Settings),
        (0, 2) => UiAction::ChangePage(Page::Files),
        (0, 3) => UiAction::ChangePage(Page::MoveTemp),

        // Move/Temp page
        (3, 1) => UiAction::MoveDistance1,
        (3, 2) => UiAction::MoveDistance10,
        (3, 3) => UiAction::MoveDistance30,

        // Settings/System page
        (11, 1) => UiAction::ChangePage(Page::Network),
        (11, 2) => UiAction::ChangePage(Page::Calibration),

        // Calibration page
        (33, 1) => UiAction::HomeAllAxes,

        // Print page
        (2, 1) => UiAction::PausePrint,
        (2, 2) => UiAction::StopPrint,

        // Fallback
        _ => UiAction::UnknownTouch { page, component },
    }
}

impl UiAction {
    pub fn is_navigation(&self) -> bool {
        matches!(self, UiAction::ChangePage(_))
    }

    pub fn is_global_stop(&self) -> bool {
        matches!(self, UiAction::StopPrint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_component_1_goes_to_settings() {
        assert_eq!(
            resolve_touch(0, 1),
            UiAction::ChangePage(Page::Settings)
        );
    }

    #[test]
    fn home_component_2_goes_to_files() {
        assert_eq!(
            resolve_touch(0, 2),
            UiAction::ChangePage(Page::Files)
        );
    }

    #[test]
    fn move_temp_distance_buttons() {
        assert_eq!(resolve_touch(3, 1), UiAction::MoveDistance1);
        assert_eq!(resolve_touch(3, 2), UiAction::MoveDistance10);
        assert_eq!(resolve_touch(3, 3), UiAction::MoveDistance30);
    }

    #[test]
    fn settings_component_2_goes_to_calibration() {
        assert_eq!(
            resolve_touch(11, 2),
            UiAction::ChangePage(Page::Calibration)
        );
    }

    #[test]
    fn unknown_touch_is_preserved() {
        assert_eq!(
            resolve_touch(99, 88),
            UiAction::UnknownTouch {
                page: 99,
                component: 88,
            }
        );
    }

    #[test]
    fn action_knows_if_it_is_navigation() {
        assert!(UiAction::ChangePage(Page::Settings).is_navigation());
        assert!(!UiAction::HomeAllAxes.is_navigation());
    }

    #[test]
    fn stop_is_global_stop_action() {
        assert!(UiAction::StopPrint.is_global_stop());
        assert!(!UiAction::PausePrint.is_global_stop());
    }
}
