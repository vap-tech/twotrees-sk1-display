use crate::app::state::{AppState, Page};
use crate::hmi::command::HmiCommand;

pub fn render_full(state: &AppState) -> Vec<HmiCommand> {
    match state.ui.current_page {
        Page::Home => render_home_full(state),
        Page::MoveTemp => render_move_temp_full(state),
        Page::Settings => render_settings_full(state),
        _ => Vec::new(),
    }
}

fn render_home_full(state: &AppState) -> Vec<HmiCommand> {
    vec![
        HmiCommand::value(
            "n0",
            round_temperature(state.temperatures.nozzle.current),
        ),
        HmiCommand::value(
            "n1",
            round_temperature(state.temperatures.nozzle.target),
        ),
        HmiCommand::value(
            "n4",
            round_temperature(state.temperatures.bed.current),
        ),
        HmiCommand::value(
            "n5",
            round_temperature(state.temperatures.bed.target),
        ),
    ]
}

fn render_move_temp_full(state: &AppState) -> Vec<HmiCommand> {
    vec![
        HmiCommand::value(
            "n0",
            round_temperature(state.temperatures.nozzle.current),
        ),
        HmiCommand::value(
            "n1",
            round_temperature(state.temperatures.nozzle.target),
        ),
        HmiCommand::value(
            "n4",
            round_temperature(state.temperatures.bed.current),
        ),
        HmiCommand::value(
            "n5",
            round_temperature(state.temperatures.bed.target),
        ),
        HmiCommand::value(
            "n3",
            state.ui.move_distance.value_mm() as i32,
        ),
    ]
}

fn render_settings_full(_state: &AppState) -> Vec<HmiCommand> {
    Vec::new()
}

fn round_temperature(value: f32) -> i32 {
    value.round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::state::{MoveDistance, Page};

    #[test]
    fn render_home_full_temperatures() {
        let mut state = AppState::default();

        state.set_page(Page::Home);
        state.set_nozzle_temperature(215.4, 220.0);
        state.set_bed_temperature(59.6, 60.0);

        let commands = render_full(&state);

        assert_eq!(
            commands,
            vec![
                HmiCommand::value("n0", 215),
                HmiCommand::value("n1", 220),
                HmiCommand::value("n4", 60),
                HmiCommand::value("n5", 60),
            ]
        );
    }

    #[test]
    fn render_move_temp_full_includes_move_distance() {
        let mut state = AppState::default();

        state.set_page(Page::MoveTemp);
        state.ui.move_distance = MoveDistance::Mm30;

        state.set_nozzle_temperature(200.0, 210.0);
        state.set_bed_temperature(50.0, 55.0);

        let commands = render_full(&state);

        assert_eq!(
            commands,
            vec![
                HmiCommand::value("n0", 200),
                HmiCommand::value("n1", 210),
                HmiCommand::value("n4", 50),
                HmiCommand::value("n5", 55),
                HmiCommand::value("n3", 30),
            ]
        );
    }

    #[test]
    fn render_unknown_page_returns_no_commands() {
        let mut state = AppState::default();

        state.set_page(Page::Unknown(999));

        let commands = render_full(&state);

        assert!(commands.is_empty());
    }

    #[test]
    fn temperature_rounding() {
        assert_eq!(round_temperature(23.4), 23);
        assert_eq!(round_temperature(23.5), 24);
        assert_eq!(round_temperature(23.6), 24);
    }
}
