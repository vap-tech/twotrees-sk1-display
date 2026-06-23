use crate::app::state::{AppState, Page, PrinterStatus};
use crate::hmi::command::HmiCommand;

/// Полная отрисовка текущей страницы.
///
/// Используется при первом входе на страницу и после init 0x91 от дисплея.
/// Дисплей нормально принимает повторный `page N`, поэтому full render можно
/// безопасно отправлять даже если визуально он уже на этой странице.
pub fn render_full(state: &AppState) -> Vec<HmiCommand> {
    match state.ui.current_page {
        Page::Home => render_home_full(state),
        Page::Print | Page::Printing => render_print_full(state),
        Page::MoveTemp => render_move_temp_full(state),
        Page::Settings => render_settings_full(state),
        _ => Vec::new(),
    }
}

fn render_home_full(state: &AppState) -> Vec<HmiCommand> {
    vec![
        HmiCommand::value("n0", round_temperature(state.temperatures.nozzle.current)),
        HmiCommand::value("n1", round_temperature(state.temperatures.nozzle.target)),
        HmiCommand::value("n4", round_temperature(state.temperatures.bed.current)),
        HmiCommand::value("n5", round_temperature(state.temperatures.bed.target)),
    ]
}

fn render_move_temp_full(state: &AppState) -> Vec<HmiCommand> {
    vec![
        HmiCommand::value("n0", round_temperature(state.temperatures.nozzle.current)),
        HmiCommand::value("n1", round_temperature(state.temperatures.nozzle.target)),
        HmiCommand::value("n4", round_temperature(state.temperatures.bed.current)),
        HmiCommand::value("n5", round_temperature(state.temperatures.bed.target)),
        HmiCommand::value("n3", state.ui.move_distance.value_mm() as i32),
    ]
}

fn render_print_full(state: &AppState) -> Vec<HmiCommand> {
    let elapsed_minutes = state.print.elapsed_seconds / 60;
    let remaining_minutes = state.print.remaining_seconds.unwrap_or(0) / 60;
    let (pause_pic, pause_pressed_pic) = print_pause_button_pics(state.printer.status);
    let filename = state.print.filename.as_deref().unwrap_or("");

    vec![
        // Пока thumbnails не подключены, скрываем cp0 и рисуем только поля
        // печати. Базовая wifi-иконка нужна, чтобы не оставался мусор с Home.
        HmiCommand::picture("Print_Trun_1.p0", 67),
        HmiCommand::text("g0", filename),
        HmiCommand::value("n0", round_temperature(state.temperatures.nozzle.current)),
        HmiCommand::value("n1", round_temperature(state.temperatures.bed.current)),
        HmiCommand::text(
            "t8",
            round_temperature(state.temperatures.nozzle.target).to_string(),
        ),
        HmiCommand::text(
            "t9",
            round_temperature(state.temperatures.bed.target).to_string(),
        ),
        HmiCommand::value("n6", state.print.progress_percent as i32),
        HmiCommand::value("n4", (elapsed_minutes / 60) as i32),
        HmiCommand::value("n5", (elapsed_minutes % 60) as i32),
        HmiCommand::value("n7", (remaining_minutes / 60) as i32),
        HmiCommand::value("n8", (remaining_minutes % 60) as i32),
        HmiCommand::visible("cp0", false),
        HmiCommand::picture("b5", pause_pic),
        HmiCommand::picture_pressed("b5", pause_pressed_pic),
    ]
}

fn render_settings_full(_state: &AppState) -> Vec<HmiCommand> {
    Vec::new()
}

fn round_temperature(value: f32) -> i32 {
    value.round() as i32
}

fn print_pause_button_pics(status: PrinterStatus) -> (u16, u16) {
    // У штатного HMI одна кнопка меняет смысл: во время печати это Pause,
    // на паузе - Resume. Иконки задаются picc/picc2.
    if status == PrinterStatus::Paused {
        (5, 4)
    } else {
        (4, 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::state::{MoveDistance, Page, PrinterStatus};

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
    fn render_print_full_includes_print_page_fields() {
        let mut state = AppState::default();

        state.set_page(Page::Printing);
        state.set_nozzle_temperature(210.0, 220.0);
        state.set_bed_temperature(50.0, 60.0);
        state.print.filename = Some("cube.gcode".to_string());
        state.print.progress_percent = 32;
        state.print.elapsed_seconds = 2 * 3600 + 41 * 60;
        state.print.remaining_seconds = Some(5 * 3600 + 52 * 60);

        let commands = render_full(&state);

        assert_eq!(
            commands,
            vec![
                HmiCommand::picture("Print_Trun_1.p0", 67),
                HmiCommand::text("g0", "cube.gcode"),
                HmiCommand::value("n0", 210),
                HmiCommand::value("n1", 50),
                HmiCommand::text("t8", "220"),
                HmiCommand::text("t9", "60"),
                HmiCommand::value("n6", 32),
                HmiCommand::value("n4", 2),
                HmiCommand::value("n5", 41),
                HmiCommand::value("n7", 5),
                HmiCommand::value("n8", 52),
                HmiCommand::visible("cp0", false),
                HmiCommand::picture("b5", 4),
                HmiCommand::picture_pressed("b5", 5),
            ]
        );
    }

    #[test]
    fn render_print_full_clears_unknown_remaining_time() {
        let mut state = AppState::default();

        state.set_page(Page::Printing);
        state.print.remaining_seconds = None;

        let commands = render_full(&state);

        assert!(commands.contains(&HmiCommand::value("n7", 0)));
        assert!(commands.contains(&HmiCommand::value("n8", 0)));
    }

    #[test]
    fn render_print_full_uses_resume_button_when_paused() {
        let mut state = AppState::default();

        state.set_page(Page::Printing);
        state.printer.status = PrinterStatus::Paused;

        let commands = render_full(&state);

        assert!(commands.contains(&HmiCommand::picture("b5", 5)));
        assert!(commands.contains(&HmiCommand::picture_pressed("b5", 4)));
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
