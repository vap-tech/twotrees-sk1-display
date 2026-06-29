use crate::app::state::{FanKind, Page};
use crate::ui::intent::UiIntent;

/// Тупая таблица маршрутизации touch-событий.
///
/// Здесь нет доступа к AppState: одинаковый touch всегда превращается в один
/// и тот же intent, а блокировки и побочные эффекты решаются ниже.
pub fn route_touch(page: u8, component: u8) -> UiIntent {
    if let Some(intent) = route_global_navigation(component) {
        return intent;
    }

    if let Some(intent) = route_faq_tabs(page, component) {
        return intent;
    }

    if let Some(intent) = route_file_tabs(page, component) {
        return intent;
    }

    match (page, component) {
        // Home page. Номера компонентов выше 4 - локальные кнопки страницы.
        (0, 5) => UiIntent::ToggleCaseLight,
        (0, 6) => UiIntent::Navigate(Page::Fans),

        // Settings/System page
        (11, 5) => UiIntent::Navigate(Page::Network),
        (11, 6) => UiIntent::Navigate(Page::Calibration),

        // Calibration page
        (33, 5) => UiIntent::HomeAllAxes,

        // Print page
        (2, 5) => UiIntent::TogglePauseResumePrint,
        (2, 6) => UiIntent::ToggleCaseLight,
        (2, 7) => UiIntent::Navigate(Page::Fans),

        // Print result page. Component 5 is Reprint, 6 is Back/Clear.
        (77, 5) => UiIntent::ReprintCurrentFile,
        (77, 6) => UiIntent::ClearPrintResult,

        // Неизвестные touch не теряем: их видно в логах и можно дописать позже.
        _ => UiIntent::UnknownTouch { page, component },
    }
}

fn route_global_navigation(component: u8) -> Option<UiIntent> {
    let page = match component {
        0 => Page::Home,
        1 => Page::MoveTemp,
        2 => Page::Files,
        3 => Page::Settings,
        4 => Page::Faq,
        _ => return None,
    };

    Some(UiIntent::Navigate(page))
}

fn route_faq_tabs(page: u8, component: u8) -> Option<UiIntent> {
    if !matches!(page, 21 | 52 | 53) {
        return None;
    }

    let page = match component {
        5 => Page::Faq,
        6 => Page::OnlineManual,
        7 => Page::Contact,
        _ => return None,
    };

    Some(UiIntent::Navigate(page))
}

fn route_file_tabs(page: u8, component: u8) -> Option<UiIntent> {
    if !matches!(page, 7 | 10 | 54) {
        return None;
    }

    let page = match component {
        5 => Page::Files,
        6 => Page::UsbFiles,
        7 => Page::FileHistory,
        _ => return None,
    };

    Some(UiIntent::Navigate(page))
}

pub fn route_numeric(page: u8, component: u8, value: i32) -> UiIntent {
    match (page, component) {
        // Fans page. Значение приходит уже в процентах.
        (6, 0) => UiIntent::SetFanPercent {
            fan: FanKind::Part,
            percent: clamp_percent(value),
        },
        (6, 1) => UiIntent::SetFanPercent {
            fan: FanKind::Side,
            percent: clamp_percent(value),
        },
        (6, 2) => UiIntent::SetFanPercent {
            fan: FanKind::Filter,
            percent: clamp_percent(value),
        },

        _ => UiIntent::UnknownNumeric {
            page,
            component,
            value,
        },
    }
}

fn clamp_percent(value: i32) -> u8 {
    value.clamp(0, 100) as u8
}

// Старое имя оставляем, чтобы наружный код мигрировал постепенно.
pub fn resolve_touch(page: u8, component: u8) -> UiIntent {
    route_touch(page, component)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_0_to_4_are_global_navigation_on_any_page() {
        for page in [0, 2, 3, 6, 11, 33, 54, 77] {
            assert_eq!(route_touch(page, 0), UiIntent::Navigate(Page::Home));
            assert_eq!(route_touch(page, 1), UiIntent::Navigate(Page::MoveTemp));
            assert_eq!(route_touch(page, 2), UiIntent::Navigate(Page::Files));
            assert_eq!(route_touch(page, 3), UiIntent::Navigate(Page::Settings));
            assert_eq!(route_touch(page, 4), UiIntent::Navigate(Page::Faq));
        }
    }

    #[test]
    fn home_component_5_toggles_case_light() {
        assert_eq!(route_touch(0, 5), UiIntent::ToggleCaseLight);
    }

    #[test]
    fn home_component_6_goes_to_fans_page() {
        assert_eq!(route_touch(0, 6), UiIntent::Navigate(Page::Fans));
    }

    #[test]
    fn settings_local_components_go_to_network_and_calibration() {
        assert_eq!(route_touch(11, 5), UiIntent::Navigate(Page::Network));
        assert_eq!(route_touch(11, 6), UiIntent::Navigate(Page::Calibration));
    }

    #[test]
    fn faq_group_components_5_to_7_are_second_level_navigation() {
        for page in [21, 52, 53] {
            assert_eq!(route_touch(page, 5), UiIntent::Navigate(Page::Faq));
            assert_eq!(route_touch(page, 6), UiIntent::Navigate(Page::OnlineManual));
            assert_eq!(route_touch(page, 7), UiIntent::Navigate(Page::Contact));
        }
    }

    #[test]
    fn file_group_components_5_to_7_are_second_level_navigation() {
        for page in [7, 10, 54] {
            assert_eq!(route_touch(page, 5), UiIntent::Navigate(Page::Files));
            assert_eq!(route_touch(page, 6), UiIntent::Navigate(Page::UsbFiles));
            assert_eq!(route_touch(page, 7), UiIntent::Navigate(Page::FileHistory));
        }
    }

    #[test]
    fn print_component_6_toggles_case_light() {
        assert_eq!(route_touch(2, 6), UiIntent::ToggleCaseLight);
    }

    #[test]
    fn print_component_5_toggles_pause_resume() {
        assert_eq!(route_touch(2, 5), UiIntent::TogglePauseResumePrint);
    }

    #[test]
    fn print_component_7_goes_to_fans_page() {
        assert_eq!(route_touch(2, 7), UiIntent::Navigate(Page::Fans));
    }

    #[test]
    fn print_result_component_6_clears_print_result() {
        assert_eq!(route_touch(77, 6), UiIntent::ClearPrintResult);
    }

    #[test]
    fn print_result_component_5_reprints_current_file() {
        assert_eq!(route_touch(77, 5), UiIntent::ReprintCurrentFile);
    }

    #[test]
    fn calibration_local_component_5_homes_all_axes() {
        assert_eq!(route_touch(33, 5), UiIntent::HomeAllAxes);
    }

    #[test]
    fn fans_numeric_values_set_fan_percent() {
        assert_eq!(
            route_numeric(6, 0, 120),
            UiIntent::SetFanPercent {
                fan: FanKind::Part,
                percent: 100,
            }
        );
        assert_eq!(
            route_numeric(6, 1, 42),
            UiIntent::SetFanPercent {
                fan: FanKind::Side,
                percent: 42,
            }
        );
        assert_eq!(
            route_numeric(6, 2, -1),
            UiIntent::SetFanPercent {
                fan: FanKind::Filter,
                percent: 0,
            }
        );
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
        assert!(!UiIntent::TogglePauseResumePrint.is_global_stop());
        assert!(!UiIntent::PausePrint.is_global_stop());
    }
}
