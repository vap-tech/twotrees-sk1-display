use crate::app::state::{ActiveOperation, AppState, FanKind, PrinterStatus};
use crate::hmi::command::HmiCommand;
use crate::ui::effect::{MoonrakerRequest, UiEffect};
use crate::ui::intent::UiIntent;
use crate::ui::render_target::resolve_render_target;

/// Проверяет, можно ли выполнять intent в текущем состоянии принтера.
///
/// Блокировка находится здесь, а не в route: одно и то же намерение может
/// прийти с разных страниц, но безопасность зависит от PrinterState.
pub fn intent_is_blocked_by_printer_state(state: &AppState, intent: &UiIntent) -> bool {
    match state.printer.status {
        PrinterStatus::Printing => is_printing_blocked_intent(intent),
        PrinterStatus::Paused => is_paused_blocked_intent(intent),
        _ => false,
    }
}

/// Применяет только локальное HMI/process состояние.
///
/// Эта функция не создает MoonrakerRequest и не пишет в UART.
pub fn apply_hmi_intent(state: &mut AppState, intent: &UiIntent) {
    match intent {
        UiIntent::Navigate(screen) => {
            state.request_page(*screen);
        }

        UiIntent::SelectMoveDistance(distance) => {
            state.hmi.move_distance = *distance;
        }

        UiIntent::HomeAllAxes => {
            state.lock_navigation(ActiveOperation::Homing);
        }

        UiIntent::LoadFilament => {
            state.lock_navigation(ActiveOperation::LoadFilament);
        }

        UiIntent::UnloadFilament => {
            state.lock_navigation(ActiveOperation::UnloadFilament);
        }

        UiIntent::OpenPrintControls
        | UiIntent::ToggleCaseLight
        | UiIntent::ToggleFan(_)
        | UiIntent::SetFanPercent { .. }
        | UiIntent::SetNozzleTarget { .. }
        | UiIntent::SetBedTarget { .. }
        | UiIntent::MoveAxis { .. }
        | UiIntent::StartPrint
        | UiIntent::TogglePauseResumePrint
        | UiIntent::PausePrint
        | UiIntent::ResumePrint
        | UiIntent::StopPrint
        | UiIntent::ClearPrintResult
        | UiIntent::ReprintCurrentFile
        | UiIntent::UnknownTouch { .. }
        | UiIntent::UnknownNumeric { .. } => {}
    }
}

/// Превращает intent в запросы к Moonraker.
///
/// Функция читает actual state, но не меняет его. Например, fan/light toggles
/// отправляют желаемое новое значение, а фактическое состояние должно вернуться
/// позже через MoonrakerEvent и reducer.
pub fn moonraker_requests_for_intent(state: &AppState, intent: &UiIntent) -> Vec<MoonrakerRequest> {
    match intent {
        UiIntent::ToggleCaseLight => {
            vec![MoonrakerRequest::SetCaseLight(!state.lights.case_light)]
        }

        UiIntent::ToggleFan(fan) => {
            let next = next_fan_percent(state, *fan);
            match fan {
                FanKind::Part => vec![MoonrakerRequest::SetPartFan(next)],
                FanKind::Side => vec![MoonrakerRequest::SetSideFan(next)],
                FanKind::Filter => vec![MoonrakerRequest::SetFilterFan(next)],
            }
        }

        UiIntent::SetFanPercent { fan, percent } => match fan {
            FanKind::Part => vec![MoonrakerRequest::SetPartFan(*percent)],
            FanKind::Side => vec![MoonrakerRequest::SetSideFan(*percent)],
            FanKind::Filter => vec![MoonrakerRequest::SetFilterFan(*percent)],
        },

        UiIntent::SetNozzleTarget { celsius } => vec![MoonrakerRequest::SetNozzleTarget(*celsius)],

        UiIntent::SetBedTarget { celsius } => vec![MoonrakerRequest::SetBedTarget(*celsius)],

        UiIntent::HomeAllAxes => vec![MoonrakerRequest::SendGcode("G28".to_string())],

        UiIntent::MoveAxis { axis, distance } => {
            vec![MoonrakerRequest::SendGcode(format!(
                "G91\nG1 {axis}{distance:.3}\nG90"
            ))]
        }

        UiIntent::LoadFilament => vec![MoonrakerRequest::SendGcode("LOAD_MATERIAL".to_string())],

        UiIntent::UnloadFilament => {
            vec![MoonrakerRequest::SendGcode("UNLOAD_MATERIAL".to_string())]
        }

        UiIntent::TogglePauseResumePrint => match state.printer.status {
            PrinterStatus::Paused => vec![MoonrakerRequest::ResumePrint],
            PrinterStatus::Printing => vec![MoonrakerRequest::PausePrint],
            _ => Vec::new(),
        },
        UiIntent::PausePrint => vec![MoonrakerRequest::PausePrint],
        UiIntent::ResumePrint => vec![MoonrakerRequest::ResumePrint],
        UiIntent::StopPrint => vec![MoonrakerRequest::CancelPrint],
        UiIntent::ClearPrintResult => vec![MoonrakerRequest::ClearPrintResult],
        UiIntent::ReprintCurrentFile => state
            .print
            .filename
            .as_ref()
            .map(|filename| MoonrakerRequest::StartPrint {
                filename: filename.clone(),
            })
            .into_iter()
            .collect(),

        UiIntent::Navigate(_)
        | UiIntent::OpenPrintControls
        | UiIntent::SelectMoveDistance(_)
        | UiIntent::StartPrint
        | UiIntent::UnknownTouch { .. }
        | UiIntent::UnknownNumeric { .. } => Vec::new(),
    }
}

/// Совместимая обертка для старого API.
///
/// Новый runtime использует `apply_hmi_intent` и
/// `moonraker_requests_for_intent` отдельно, но unit tests и внешний код могут
/// пока вызывать этот метод.
pub fn handle_action(state: &mut AppState, intent: UiIntent) -> Vec<UiEffect> {
    if intent_is_blocked_by_printer_state(state, &intent) {
        tracing::debug!(
            ?intent,
            printer_status = ?state.printer.status,
            "UI intent blocked by printer state"
        );
        return Vec::new();
    }

    let old_target = resolve_render_target(state);
    apply_hmi_intent(state, &intent);

    let mut effects = Vec::new();
    let new_target = resolve_render_target(state);
    if old_target != new_target {
        effects.push(UiEffect::hmi(HmiCommand::page(new_target.page_id())));
    }

    effects.extend(
        moonraker_requests_for_intent(state, &intent)
            .into_iter()
            .map(UiEffect::Moonraker),
    );

    effects
}

fn is_printing_blocked_intent(intent: &UiIntent) -> bool {
    // Во время печати страницы можно открывать для просмотра, но запрещаем
    // сами действия, которые могут двигать механику или запускать филамент.
    matches!(
        intent,
        UiIntent::HomeAllAxes
            | UiIntent::MoveAxis { .. }
            | UiIntent::LoadFilament
            | UiIntent::UnloadFilament
    )
}

fn is_paused_blocked_intent(intent: &UiIntent) -> bool {
    // На паузе load/unload допустим, но homing/move все еще опасны.
    matches!(intent, UiIntent::HomeAllAxes | UiIntent::MoveAxis { .. })
}

fn next_fan_percent(state: &AppState, fan: FanKind) -> u8 {
    let current = match fan {
        FanKind::Part => state.fans.part.percent,
        FanKind::Side => state.fans.side.percent,
        FanKind::Filter => state.fans.filter.percent,
    };

    if current == 0 { 100 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{MoveDistance, Page};
    use crate::ui::intent::Axis;

    #[test]
    fn navigate_updates_hmi_state_without_moonraker_request() {
        let mut state = AppState::default();

        apply_hmi_intent(&mut state, &UiIntent::Navigate(Page::Settings));
        let requests = moonraker_requests_for_intent(&state, &UiIntent::Navigate(Page::Settings));

        assert_eq!(state.hmi.current_screen, Page::Settings);
        assert!(requests.is_empty());
    }

    #[test]
    fn select_move_distance_updates_only_hmi_state() {
        let mut state = AppState::default();

        apply_hmi_intent(&mut state, &UiIntent::SelectMoveDistance(MoveDistance::Mm1));
        let requests =
            moonraker_requests_for_intent(&state, &UiIntent::SelectMoveDistance(MoveDistance::Mm1));

        assert_eq!(state.hmi.move_distance, MoveDistance::Mm1);
        assert!(requests.is_empty());
    }

    #[test]
    fn toggle_part_fan_reads_actual_state_without_mutating_it() {
        let mut state = AppState::default();

        let requests = moonraker_requests_for_intent(&state, &UiIntent::ToggleFan(FanKind::Part));

        assert_eq!(state.fans.part.percent, 0);
        assert_eq!(requests, vec![MoonrakerRequest::SetPartFan(100)]);

        state.fans.part.percent = 100;
        let requests = moonraker_requests_for_intent(&state, &UiIntent::ToggleFan(FanKind::Part));

        assert_eq!(state.fans.part.percent, 100);
        assert_eq!(requests, vec![MoonrakerRequest::SetPartFan(0)]);
    }

    #[test]
    fn toggle_side_and_filter_fans_create_requests() {
        let state = AppState::default();

        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::ToggleFan(FanKind::Side)),
            vec![MoonrakerRequest::SetSideFan(100)]
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::ToggleFan(FanKind::Filter)),
            vec![MoonrakerRequest::SetFilterFan(100)]
        );
    }

    #[test]
    fn set_fan_percent_creates_request_without_mutating_state() {
        let state = AppState::default();

        let requests = moonraker_requests_for_intent(
            &state,
            &UiIntent::SetFanPercent {
                fan: FanKind::Side,
                percent: 42,
            },
        );

        assert_eq!(state.fans.side.percent, 0);
        assert_eq!(requests, vec![MoonrakerRequest::SetSideFan(42)]);
    }

    #[test]
    fn set_temperature_targets_create_requests() {
        let state = AppState::default();

        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::SetNozzleTarget { celsius: 215 }),
            vec![MoonrakerRequest::SetNozzleTarget(215)]
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::SetBedTarget { celsius: 60 }),
            vec![MoonrakerRequest::SetBedTarget(60)]
        );
    }

    #[test]
    fn toggle_case_light_reads_actual_state_without_mutating_it() {
        let mut state = AppState::default();

        let requests = moonraker_requests_for_intent(&state, &UiIntent::ToggleCaseLight);

        assert!(!state.lights.case_light);
        assert_eq!(requests, vec![MoonrakerRequest::SetCaseLight(true)]);

        state.lights.case_light = true;
        let requests = moonraker_requests_for_intent(&state, &UiIntent::ToggleCaseLight);

        assert!(state.lights.case_light);
        assert_eq!(requests, vec![MoonrakerRequest::SetCaseLight(false)]);
    }

    #[test]
    fn home_all_axes_locks_navigation_and_returns_gcode_request() {
        let mut state = AppState::default();

        apply_hmi_intent(&mut state, &UiIntent::HomeAllAxes);
        let requests = moonraker_requests_for_intent(&state, &UiIntent::HomeAllAxes);

        assert!(state.hmi.navigation_locked);
        assert_eq!(state.process.active_operation, ActiveOperation::Homing);
        assert_eq!(
            requests,
            vec![MoonrakerRequest::SendGcode("G28".to_string())]
        );
    }

    #[test]
    fn move_axis_returns_relative_move_gcode() {
        let state = AppState::default();

        let requests = moonraker_requests_for_intent(
            &state,
            &UiIntent::MoveAxis {
                axis: Axis::Y,
                distance: -10.0,
            },
        );

        assert_eq!(
            requests,
            vec![MoonrakerRequest::SendGcode(
                "G91\nG1 Y-10.000\nG90".to_string()
            )]
        );
    }

    #[test]
    fn printing_allows_navigation_to_dangerous_pages_for_viewing() {
        let mut state = AppState::default();
        state.printer.status = PrinterStatus::Printing;

        for page in [Page::Calibration, Page::MoveTemp, Page::LoadUnload] {
            assert!(!intent_is_blocked_by_printer_state(
                &state,
                &UiIntent::Navigate(page)
            ));
        }
    }

    #[test]
    fn printing_keeps_settings_and_files_navigation_allowed() {
        let mut state = AppState::default();
        state.printer.status = PrinterStatus::Printing;

        assert!(!intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::Navigate(Page::Settings)
        ));
        assert!(!intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::Navigate(Page::Files)
        ));
    }

    #[test]
    fn printing_blocks_homing_movement_and_filament_commands() {
        let mut state = AppState::default();
        state.printer.status = PrinterStatus::Printing;

        for intent in [
            UiIntent::HomeAllAxes,
            UiIntent::MoveAxis {
                axis: Axis::X,
                distance: 10.0,
            },
            UiIntent::LoadFilament,
            UiIntent::UnloadFilament,
        ] {
            assert!(intent_is_blocked_by_printer_state(&state, &intent));
        }
    }

    #[test]
    fn paused_allows_navigation_and_blocks_only_movement_actions() {
        let mut state = AppState::default();
        state.printer.status = PrinterStatus::Paused;

        assert!(!intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::Navigate(Page::Calibration)
        ));
        assert!(!intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::Navigate(Page::MoveTemp)
        ));
        assert!(!intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::Navigate(Page::LoadUnload)
        ));
        assert!(intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::HomeAllAxes
        ));
        assert!(intent_is_blocked_by_printer_state(
            &state,
            &UiIntent::MoveAxis {
                axis: Axis::Y,
                distance: 1.0,
            }
        ));
    }

    #[test]
    fn print_control_intents_return_moonraker_requests() {
        let mut state = AppState::default();

        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::PausePrint),
            vec![MoonrakerRequest::PausePrint]
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::ResumePrint),
            vec![MoonrakerRequest::ResumePrint]
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::StopPrint),
            vec![MoonrakerRequest::CancelPrint]
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::ClearPrintResult),
            vec![MoonrakerRequest::ClearPrintResult]
        );

        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::ReprintCurrentFile),
            Vec::<MoonrakerRequest>::new()
        );

        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::TogglePauseResumePrint),
            Vec::<MoonrakerRequest>::new()
        );

        state.printer.status = PrinterStatus::Printing;
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::TogglePauseResumePrint),
            vec![MoonrakerRequest::PausePrint]
        );

        state.printer.status = PrinterStatus::Paused;
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::TogglePauseResumePrint),
            vec![MoonrakerRequest::ResumePrint]
        );
    }

    #[test]
    fn reprint_current_file_uses_current_print_filename() {
        let mut state = AppState::default();
        state.print.filename = Some("cube.gcode".to_string());

        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::ReprintCurrentFile),
            vec![MoonrakerRequest::StartPrint {
                filename: "cube.gcode".to_string(),
            }]
        );
    }

    #[test]
    fn load_unload_lock_navigation_and_return_gcode() {
        let mut state = AppState::default();

        apply_hmi_intent(&mut state, &UiIntent::LoadFilament);
        assert_eq!(
            state.process.active_operation,
            ActiveOperation::LoadFilament
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::LoadFilament),
            vec![MoonrakerRequest::SendGcode("LOAD_MATERIAL".to_string())]
        );

        state.unlock_navigation();

        apply_hmi_intent(&mut state, &UiIntent::UnloadFilament);
        assert_eq!(
            state.process.active_operation,
            ActiveOperation::UnloadFilament
        );
        assert_eq!(
            moonraker_requests_for_intent(&state, &UiIntent::UnloadFilament),
            vec![MoonrakerRequest::SendGcode("UNLOAD_MATERIAL".to_string())]
        );
    }

    #[test]
    fn unknown_touch_does_nothing() {
        let mut state = AppState::default();

        apply_hmi_intent(
            &mut state,
            &UiIntent::UnknownTouch {
                page: 99,
                component: 88,
            },
        );

        assert_eq!(state, AppState::default());
        assert!(
            moonraker_requests_for_intent(
                &state,
                &UiIntent::UnknownTouch {
                    page: 99,
                    component: 88,
                },
            )
            .is_empty()
        );
    }

    #[test]
    fn handle_action_wrapper_keeps_navigation_effect_for_compatibility() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiIntent::Navigate(Page::Settings));

        assert_eq!(state.hmi.current_screen, Page::Settings);
        assert_eq!(
            effects,
            vec![UiEffect::hmi(HmiCommand::page(Page::Settings.id()))]
        );
    }
}
