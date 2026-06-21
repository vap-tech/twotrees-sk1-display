use crate::app::state::{
    ActiveOperation, AppState, FanKind, MoveDistance, Page,
};
use crate::hmi::command::HmiCommand;
use crate::ui::effect::{MoonrakerRequest, UiEffect};
use crate::ui::route::UiAction;

pub fn handle_action(
    state: &mut AppState,
    action: UiAction,
) -> Vec<UiEffect> {
    match action {
        UiAction::ChangePage(page) => handle_change_page(state, page),

        UiAction::MoveDistance1 => {
            state.ui.move_distance = MoveDistance::Mm1;
            Vec::new()
        }

        UiAction::MoveDistance10 => {
            state.ui.move_distance = MoveDistance::Mm10;
            Vec::new()
        }

        UiAction::MoveDistance30 => {
            state.ui.move_distance = MoveDistance::Mm30;
            Vec::new()
        }

        UiAction::TogglePartFan => {
            toggle_fan(state, FanKind::Part);
            vec![UiEffect::Moonraker(MoonrakerRequest::SetPartFan(
                state.fans.part.percent,
            ))]
        }

        UiAction::ToggleSideFan => {
            toggle_fan(state, FanKind::Side);
            vec![UiEffect::Moonraker(MoonrakerRequest::SetSideFan(
                state.fans.side.percent,
            ))]
        }

        UiAction::ToggleFilterFan => {
            toggle_fan(state, FanKind::Filter);
            vec![UiEffect::Moonraker(MoonrakerRequest::SetFilterFan(
                state.fans.filter.percent,
            ))]
        }

        UiAction::HomeAllAxes => {
            state.lock_navigation(ActiveOperation::Homing);

            vec![UiEffect::gcode("G28")]
        }

        UiAction::PausePrint => {
            vec![UiEffect::Moonraker(MoonrakerRequest::PausePrint)]
        }

        UiAction::ResumePrint => {
            vec![UiEffect::Moonraker(MoonrakerRequest::ResumePrint)]
        }

        UiAction::StopPrint => {
            vec![UiEffect::Moonraker(MoonrakerRequest::CancelPrint)]
        }

        UiAction::LoadFilament => {
            state.lock_navigation(ActiveOperation::LoadFilament);

            vec![UiEffect::gcode("LOAD_MATERIAL")]
        }

        UiAction::UnloadFilament => {
            state.lock_navigation(ActiveOperation::UnloadFilament);

            vec![UiEffect::gcode("UNLOAD_MATERIAL")]
        }

        UiAction::StartPrint => {
            // Позже сюда добавим выбранный файл:
            // MoonrakerRequest::StartPrint(filename)
            Vec::new()
        }

        UiAction::UnknownTouch { .. } => Vec::new(),
    }
}

fn handle_change_page(
    state: &mut AppState,
    page: Page,
) -> Vec<UiEffect> {
    if !state.request_page(page) {
        return Vec::new();
    }

    vec![UiEffect::hmi(HmiCommand::page(page.id()))]
}

fn toggle_fan(state: &mut AppState, fan: FanKind) {
    let current = match fan {
        FanKind::Part => state.fans.part.percent,
        FanKind::Side => state.fans.side.percent,
        FanKind::Filter => state.fans.filter.percent,
    };

    let next = if current == 0 { 100 } else { 0 };

    state.set_fan_percent(fan, next);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_page_updates_state_and_returns_hmi_effect() {
        let mut state = AppState::default();

        let effects = handle_action(
            &mut state,
            UiAction::ChangePage(Page::Settings),
        );

        assert_eq!(state.ui.current_page, Page::Settings);
        assert_eq!(
            effects,
            vec![UiEffect::hmi(HmiCommand::page(Page::Settings.id()))]
        );
    }

    #[test]
    fn change_page_is_blocked_when_navigation_locked() {
        let mut state = AppState::default();

        state.lock_navigation(ActiveOperation::Calibration);

        let effects = handle_action(
            &mut state,
            UiAction::ChangePage(Page::Settings),
        );

        assert_eq!(state.ui.current_page, Page::Home);
        assert!(effects.is_empty());
    }

    #[test]
    fn move_distance_1_updates_state_without_effects() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::MoveDistance1);

        assert_eq!(state.ui.move_distance, MoveDistance::Mm1);
        assert!(effects.is_empty());
    }

    #[test]
    fn move_distance_10_updates_state_without_effects() {
        let mut state = AppState::default();

        state.ui.move_distance = MoveDistance::Mm1;

        let effects = handle_action(&mut state, UiAction::MoveDistance10);

        assert_eq!(state.ui.move_distance, MoveDistance::Mm10);
        assert!(effects.is_empty());
    }

    #[test]
    fn move_distance_30_updates_state_without_effects() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::MoveDistance30);

        assert_eq!(state.ui.move_distance, MoveDistance::Mm30);
        assert!(effects.is_empty());
    }

    #[test]
    fn toggle_part_fan_returns_moonraker_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::TogglePartFan);

        assert_eq!(state.fans.part.percent, 100);
        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::SetPartFan(100))]
        );
    }

    #[test]
    fn toggle_part_fan_second_press_turns_it_off() {
        let mut state = AppState::default();

        handle_action(&mut state, UiAction::TogglePartFan);
        let effects = handle_action(&mut state, UiAction::TogglePartFan);

        assert_eq!(state.fans.part.percent, 0);
        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::SetPartFan(0))]
        );
    }

    #[test]
    fn toggle_side_fan_returns_moonraker_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::ToggleSideFan);

        assert_eq!(state.fans.side.percent, 100);
        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::SetSideFan(100))]
        );
    }

    #[test]
    fn toggle_filter_fan_returns_moonraker_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::ToggleFilterFan);

        assert_eq!(state.fans.filter.percent, 100);
        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::SetFilterFan(100))]
        );
    }

    #[test]
    fn home_all_axes_locks_navigation_and_returns_gcode_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::HomeAllAxes);

        assert!(state.ui.navigation_locked);
        assert_eq!(
            state.process.active_operation,
            ActiveOperation::Homing
        );
        assert_eq!(effects, vec![UiEffect::gcode("G28")]);
    }

    #[test]
    fn pause_print_returns_moonraker_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::PausePrint);

        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::PausePrint)]
        );
    }

    #[test]
    fn resume_print_returns_moonraker_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::ResumePrint);

        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::ResumePrint)]
        );
    }

    #[test]
    fn stop_print_returns_cancel_effect() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::StopPrint);

        assert_eq!(
            effects,
            vec![UiEffect::Moonraker(MoonrakerRequest::CancelPrint)]
        );
    }

    #[test]
    fn load_filament_locks_navigation_and_returns_gcode() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::LoadFilament);

        assert_eq!(
            state.process.active_operation,
            ActiveOperation::LoadFilament
        );
        assert!(state.ui.navigation_locked);
        assert_eq!(effects, vec![UiEffect::gcode("LOAD_MATERIAL")]);
    }

    #[test]
    fn unload_filament_locks_navigation_and_returns_gcode() {
        let mut state = AppState::default();

        let effects = handle_action(&mut state, UiAction::UnloadFilament);

        assert_eq!(
            state.process.active_operation,
            ActiveOperation::UnloadFilament
        );
        assert!(state.ui.navigation_locked);
        assert_eq!(effects, vec![UiEffect::gcode("UNLOAD_MATERIAL")]);
    }

    #[test]
    fn unknown_touch_does_nothing() {
        let mut state = AppState::default();

        let effects = handle_action(
            &mut state,
            UiAction::UnknownTouch {
                page: 99,
                component: 88,
            },
        );

        assert_eq!(state, AppState::default());
        assert!(effects.is_empty());
    }
}
