use crate::app::state::{AppState, PrinterStatus};
use crate::hmi::command::HmiCommand;
use crate::ui::components::{render_case_light_icon, render_fan_icon};
use crate::ui::render_target::{HomeMode, RenderTarget, ResultMode, resolve_render_target};

/// Полная отрисовка текущей страницы.
///
/// Используется при первом входе на страницу и после init 0x91 от дисплея.
/// Дисплей нормально принимает повторный `page N`, поэтому full render можно
/// безопасно отправлять даже если визуально он уже на этой странице.
pub fn render_full(state: &AppState) -> Vec<HmiCommand> {
    render_full_target(resolve_render_target(state), state)
}

pub fn render_full_target(target: RenderTarget, state: &AppState) -> Vec<HmiCommand> {
    match target {
        RenderTarget::Home(HomeMode::Idle) => render_home_full(state),
        RenderTarget::Home(HomeMode::Printing) | RenderTarget::Print => render_print_full(state),
        RenderTarget::Home(HomeMode::Complete) | RenderTarget::Result(ResultMode::Success) => {
            render_result_full(true)
        }
        RenderTarget::Home(HomeMode::Cancelled)
        | RenderTarget::Home(HomeMode::Error)
        | RenderTarget::Result(ResultMode::Failed)
        | RenderTarget::Error => render_result_full(false),
        RenderTarget::MoveTemp => render_move_temp_full(state),
        RenderTarget::LoadUnload => render_load_unload_full(state),
        RenderTarget::Fans => render_fans_full(state),
        RenderTarget::Settings => render_settings_full(state),
        _ => Vec::new(),
    }
}

fn render_home_full(state: &AppState) -> Vec<HmiCommand> {
    let mut commands = vec![
        HmiCommand::value("n0", round_temperature(state.temperatures.nozzle.current)),
        HmiCommand::value("n1", round_temperature(state.temperatures.nozzle.target)),
        HmiCommand::value("n4", round_temperature(state.temperatures.bed.current)),
        HmiCommand::value("n5", round_temperature(state.temperatures.bed.target)),
    ];

    commands.extend(render_case_light_icon(
        RenderTarget::Home(HomeMode::Idle),
        state.lights.case_light,
    ));
    commands.extend(render_fan_icon(
        RenderTarget::Home(HomeMode::Idle),
        any_fan_enabled(state),
    ));

    commands
}

fn render_move_temp_full(state: &AppState) -> Vec<HmiCommand> {
    let (b5_pic, b6_pic, b7_pic) = move_distance_pics(state.hmi.move_distance);

    vec![
        HmiCommand::visible("t2", false),
        HmiCommand::picture("b5", b5_pic),
        HmiCommand::picture("b6", b6_pic),
        HmiCommand::picture("b7", b7_pic),
        HmiCommand::value("n3", round_temperature(state.temperatures.nozzle.current)),
        HmiCommand::value("n0", round_temperature(state.temperatures.nozzle.target)),
        HmiCommand::value("n2", round_temperature(state.temperatures.bed.current)),
        HmiCommand::value("n1", round_temperature(state.temperatures.bed.target)),
    ]
}

fn render_fans_full(state: &AppState) -> Vec<HmiCommand> {
    vec![
        HmiCommand::value("h0", state.fans.part.percent as i32),
        HmiCommand::value("n0", state.fans.part.percent as i32),
        HmiCommand::value("h1", state.fans.side.percent as i32),
        HmiCommand::value("n1", state.fans.side.percent as i32),
        HmiCommand::value("h2", state.fans.filter.percent as i32),
        HmiCommand::value("n2", state.fans.filter.percent as i32),
    ]
}

fn render_load_unload_full(state: &AppState) -> Vec<HmiCommand> {
    vec![HmiCommand::value(
        "n1",
        round_temperature(state.temperatures.filament_load_target),
    )]
}

fn render_print_full(state: &AppState) -> Vec<HmiCommand> {
    let elapsed_minutes = state.print.elapsed_seconds / 60;
    let remaining_minutes = state.print.remaining_seconds.unwrap_or(0) / 60;
    let (pause_pic, pause_pressed_pic) = print_pause_button_pics(state.printer.status);
    let filename = state.print.filename.as_deref().unwrap_or("");

    let mut commands = vec![
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
    ];

    commands.extend(render_case_light_icon(
        RenderTarget::Print,
        state.lights.case_light,
    ));
    commands.extend(render_fan_icon(RenderTarget::Print, any_fan_enabled(state)));

    commands
}

fn render_settings_full(_state: &AppState) -> Vec<HmiCommand> {
    Vec::new()
}

fn render_result_full(success: bool) -> Vec<HmiCommand> {
    vec![
        HmiCommand::raw("print_done.cp0.close()"),
        HmiCommand::visible("print_done.cp0", false),
        HmiCommand::raw(format!("print_done_flag={}", if success { 1 } else { 0 })),
        HmiCommand::raw("print_done.tm0.en=1"),
    ]
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

fn any_fan_enabled(state: &AppState) -> bool {
    state.fans.part.percent > 0 || state.fans.side.percent > 0 || state.fans.filter.percent > 0
}

fn move_distance_pics(distance: crate::app::state::MoveDistance) -> (u16, u16, u16) {
    match distance {
        crate::app::state::MoveDistance::Mm1 => (7, 6, 6),
        crate::app::state::MoveDistance::Mm10 => (6, 7, 6),
        crate::app::state::MoveDistance::Mm30 => (6, 6, 7),
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
                HmiCommand::picture("b5", 2),
                HmiCommand::picture_pressed("b5", 2),
                HmiCommand::picture("b6", 2),
                HmiCommand::picture_pressed("b6", 2),
            ]
        );
    }

    #[test]
    fn render_home_full_includes_enabled_fan_icon_when_any_fan_is_active() {
        let mut state = AppState::default();

        state.set_page(Page::Home);
        state.fans.part.percent = 10;

        let commands = render_full(&state);

        assert!(commands.contains(&HmiCommand::picture("b6", 3)));
        assert!(commands.contains(&HmiCommand::picture_pressed("b6", 3)));
    }

    #[test]
    fn render_move_temp_full_includes_move_distance() {
        let mut state = AppState::default();

        state.set_page(Page::MoveTemp);
        state.hmi.move_distance = MoveDistance::Mm30;

        state.set_nozzle_temperature(200.0, 210.0);
        state.set_bed_temperature(50.0, 55.0);

        let commands = render_full(&state);

        assert_eq!(
            commands,
            vec![
                HmiCommand::visible("t2", false),
                HmiCommand::picture("b5", 6),
                HmiCommand::picture("b6", 6),
                HmiCommand::picture("b7", 7),
                HmiCommand::value("n3", 200),
                HmiCommand::value("n0", 210),
                HmiCommand::value("n2", 50),
                HmiCommand::value("n1", 55),
            ]
        );
    }

    #[test]
    fn render_fans_full_includes_sliders_and_values() {
        let mut state = AppState::default();

        state.set_page(Page::Fans);
        state.fans.part.percent = 100;
        state.fans.side.percent = 42;
        state.fans.filter.percent = 9;

        let commands = render_full(&state);

        assert_eq!(
            commands,
            vec![
                HmiCommand::value("h0", 100),
                HmiCommand::value("n0", 100),
                HmiCommand::value("h1", 42),
                HmiCommand::value("n1", 42),
                HmiCommand::value("h2", 9),
                HmiCommand::value("n2", 9),
            ]
        );
    }

    #[test]
    fn render_load_unload_full_includes_filament_temperature() {
        let mut state = AppState::default();

        state.set_page(Page::LoadUnload);
        state.temperatures.filament_load_target = 230.0;

        let commands = render_full(&state);

        assert_eq!(commands, vec![HmiCommand::value("n1", 230)]);
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
                HmiCommand::picture("b6", 2),
                HmiCommand::picture_pressed("b6", 2),
                HmiCommand::picture("b7", 2),
                HmiCommand::picture_pressed("b7", 2),
            ]
        );
    }

    #[test]
    fn render_print_full_includes_case_light_icon() {
        let mut state = AppState::default();

        state.set_page(Page::Printing);
        state.lights.case_light = true;

        let commands = render_full(&state);

        assert!(commands.contains(&HmiCommand::picture("b6", 3)));
        assert!(commands.contains(&HmiCommand::picture_pressed("b6", 3)));
    }

    #[test]
    fn render_print_full_includes_enabled_fan_icon_when_any_fan_is_active() {
        let mut state = AppState::default();

        state.set_page(Page::Printing);
        state.fans.side.percent = 10;

        let commands = render_full(&state);

        assert!(commands.contains(&HmiCommand::picture("b7", 3)));
        assert!(commands.contains(&HmiCommand::picture_pressed("b7", 3)));
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
    fn render_complete_result_full_enables_hmi_status_timer() {
        let mut state = AppState::default();

        state.set_page(Page::Home);
        state.printer.status = PrinterStatus::Complete;

        let commands = render_full(&state);

        assert_eq!(
            commands,
            vec![
                HmiCommand::raw("print_done.cp0.close()"),
                HmiCommand::visible("print_done.cp0", false),
                HmiCommand::raw("print_done_flag=1"),
                HmiCommand::raw("print_done.tm0.en=1"),
            ]
        );
    }

    #[test]
    fn render_cancelled_result_full_enables_hmi_status_timer() {
        let mut state = AppState::default();

        state.set_page(Page::Home);
        state.printer.status = PrinterStatus::Cancelled;

        let commands = render_full(&state);

        assert!(commands.contains(&HmiCommand::raw("print_done_flag=0")));
        assert!(commands.contains(&HmiCommand::raw("print_done.tm0.en=1")));
    }

    #[test]
    fn render_target_home_modes_map_to_expected_page_ids() {
        assert_eq!(RenderTarget::Home(HomeMode::Idle).page_id(), 0);
        assert_eq!(RenderTarget::Home(HomeMode::Printing).page_id(), 2);
        assert_eq!(RenderTarget::Home(HomeMode::Error).page_id(), 77);
        assert_eq!(RenderTarget::Home(HomeMode::Cancelled).page_id(), 77);
        assert_eq!(RenderTarget::Home(HomeMode::Complete).page_id(), 77);
    }

    #[test]
    fn temperature_rounding() {
        assert_eq!(round_temperature(23.4), 23);
        assert_eq!(round_temperature(23.5), 24);
        assert_eq!(round_temperature(23.6), 24);
    }
}
