use crate::app::state::{AppState, PrinterStatus};
use crate::hmi::command::HmiCommand;
use crate::ui::components::render_case_light_icon;
use crate::ui::render_full::render_full_target;
use crate::ui::render_target::{HomeMode, RenderTarget, resolve_render_target};

/// Минимальная перерисовка после изменения AppState.
///
/// Если изменилась страница - отправляем `page` и полный render. Если страница
/// та же, шлем только изменившиеся поля, чтобы не забивать UART.
pub fn render_diff(old: &AppState, new: &AppState) -> Vec<HmiCommand> {
    let old_target = resolve_render_target(old);
    let new_target = resolve_render_target(new);

    if old_target != new_target {
        let mut commands = vec![HmiCommand::page(new_target.page_id())];

        commands.extend(render_full_target(new_target, new));
        return commands;
    }

    match new_target {
        RenderTarget::Home(HomeMode::Idle) => render_home_diff(old, new, new_target),
        RenderTarget::Home(HomeMode::Printing) | RenderTarget::Print => render_print_diff(old, new),
        RenderTarget::MoveTemp => render_move_temp_diff(old, new),
        _ => Vec::new(),
    }
}

fn render_home_diff(old: &AppState, new: &AppState, target: RenderTarget) -> Vec<HmiCommand> {
    let mut commands = render_temperature_diff(old, new);

    commands.extend(render_case_light_icon_diff(old, new, target));

    commands
}

fn render_move_temp_diff(old: &AppState, new: &AppState) -> Vec<HmiCommand> {
    let mut commands = render_temperature_diff(old, new);

    if old.hmi.move_distance != new.hmi.move_distance {
        commands.push(HmiCommand::value(
            "n3",
            new.hmi.move_distance.value_mm() as i32,
        ));
    }

    commands
}

fn render_print_diff(old: &AppState, new: &AppState) -> Vec<HmiCommand> {
    let mut commands = render_print_temperature_diff(old, new);

    // Имя файла приходит из print_stats и может появиться позже статуса Printing,
    // поэтому обновляем его отдельно даже на уже открытой page 2.
    if old.print.filename != new.print.filename {
        commands.push(HmiCommand::text(
            "g0",
            new.print.filename.as_deref().unwrap_or(""),
        ));
    }

    if old.print.progress_percent != new.print.progress_percent {
        commands.push(HmiCommand::value("n6", new.print.progress_percent as i32));
    }

    push_elapsed_time_diff(&mut commands, old, new);
    push_remaining_time_diff(&mut commands, old, new);
    push_print_button_diff(&mut commands, old, new);
    commands.extend(render_case_light_icon_diff(old, new, RenderTarget::Print));

    commands
}

fn render_print_temperature_diff(old: &AppState, new: &AppState) -> Vec<HmiCommand> {
    let mut commands = Vec::new();

    push_if_changed(
        &mut commands,
        "n0",
        round_temperature(old.temperatures.nozzle.current),
        round_temperature(new.temperatures.nozzle.current),
    );

    push_if_changed(
        &mut commands,
        "n1",
        round_temperature(old.temperatures.bed.current),
        round_temperature(new.temperatures.bed.current),
    );

    push_text_if_changed(
        &mut commands,
        "t8",
        round_temperature(old.temperatures.nozzle.target),
        round_temperature(new.temperatures.nozzle.target),
    );

    push_text_if_changed(
        &mut commands,
        "t9",
        round_temperature(old.temperatures.bed.target),
        round_temperature(new.temperatures.bed.target),
    );

    commands
}

fn push_elapsed_time_diff(commands: &mut Vec<HmiCommand>, old: &AppState, new: &AppState) {
    let old_minutes = old.print.elapsed_seconds / 60;
    let new_minutes = new.print.elapsed_seconds / 60;

    // HMI показывает часы/минуты, поэтому секунды внутри той же минуты не шлем.
    if old_minutes == new_minutes {
        return;
    }

    commands.push(HmiCommand::value("n4", (new_minutes / 60) as i32));
    commands.push(HmiCommand::value("n5", (new_minutes % 60) as i32));
}

fn push_remaining_time_diff(commands: &mut Vec<HmiCommand>, old: &AppState, new: &AppState) {
    let old_minutes = old.print.remaining_seconds.map(|v| v / 60);
    let new_minutes = new.print.remaining_seconds.map(|v| v / 60);

    if old_minutes == new_minutes {
        return;
    }

    // Если ETA пропал, явно чистим поля в 0:00, иначе старые числа останутся.
    let minutes = new_minutes.unwrap_or(0);

    commands.push(HmiCommand::value("n7", (minutes / 60) as i32));
    commands.push(HmiCommand::value("n8", (minutes % 60) as i32));
}

fn push_print_button_diff(commands: &mut Vec<HmiCommand>, old: &AppState, new: &AppState) {
    if old.printer.status == new.printer.status {
        return;
    }

    let (pic, pressed_pic) = print_pause_button_pics(new.printer.status);

    commands.push(HmiCommand::picture("b5", pic));
    commands.push(HmiCommand::picture_pressed("b5", pressed_pic));
}

fn render_case_light_icon_diff(
    old: &AppState,
    new: &AppState,
    target: RenderTarget,
) -> Vec<HmiCommand> {
    if old.lights.case_light == new.lights.case_light {
        return Vec::new();
    }

    render_case_light_icon(target, new.lights.case_light)
}

fn render_temperature_diff(old: &AppState, new: &AppState) -> Vec<HmiCommand> {
    let mut commands = Vec::new();

    push_if_changed(
        &mut commands,
        "n0",
        round_temperature(old.temperatures.nozzle.current),
        round_temperature(new.temperatures.nozzle.current),
    );

    push_if_changed(
        &mut commands,
        "n1",
        round_temperature(old.temperatures.nozzle.target),
        round_temperature(new.temperatures.nozzle.target),
    );

    push_if_changed(
        &mut commands,
        "n4",
        round_temperature(old.temperatures.bed.current),
        round_temperature(new.temperatures.bed.current),
    );

    push_if_changed(
        &mut commands,
        "n5",
        round_temperature(old.temperatures.bed.target),
        round_temperature(new.temperatures.bed.target),
    );

    commands
}

fn push_if_changed(
    commands: &mut Vec<HmiCommand>,
    component: &str,
    old_value: i32,
    new_value: i32,
) {
    if old_value != new_value {
        commands.push(HmiCommand::value(component, new_value));
    }
}

fn push_text_if_changed(
    commands: &mut Vec<HmiCommand>,
    component: &str,
    old_value: i32,
    new_value: i32,
) {
    if old_value != new_value {
        commands.push(HmiCommand::text(component, new_value.to_string()));
    }
}

fn round_temperature(value: f32) -> i32 {
    value.round() as i32
}

fn print_pause_button_pics(status: PrinterStatus) -> (u16, u16) {
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
    fn page_change_sends_page_and_full_render() {
        let old = AppState::default();

        let mut new = AppState::default();
        new.set_page(Page::MoveTemp);
        new.set_nozzle_temperature(200.0, 210.0);
        new.set_bed_temperature(50.0, 55.0);
        new.hmi.move_distance = MoveDistance::Mm30;

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![
                HmiCommand::page(Page::MoveTemp.id()),
                HmiCommand::value("n0", 200),
                HmiCommand::value("n1", 210),
                HmiCommand::value("n4", 50),
                HmiCommand::value("n5", 55),
                HmiCommand::value("n3", 30),
            ]
        );
    }

    #[test]
    fn same_home_page_no_changes_returns_empty() {
        let old = AppState::default();
        let new = AppState::default();

        let commands = render_diff(&old, &new);

        assert!(commands.is_empty());
    }

    #[test]
    fn same_home_page_temperature_change_renders_only_changed_value() {
        let mut old = AppState::default();
        old.set_page(Page::Home);
        old.set_nozzle_temperature(215.0, 220.0);
        old.set_bed_temperature(60.0, 60.0);

        let mut new = old.clone();
        new.set_nozzle_temperature(216.0, 220.0);

        let commands = render_diff(&old, &new);

        assert_eq!(commands, vec![HmiCommand::value("n0", 216)]);
    }

    #[test]
    fn same_home_page_case_light_change_updates_icon() {
        let mut old = AppState::default();
        old.set_page(Page::Home);
        old.lights.case_light = false;

        let mut new = old.clone();
        new.lights.case_light = true;

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![
                HmiCommand::picture("b5", 3),
                HmiCommand::picture_pressed("b5", 3),
            ]
        );
    }

    #[test]
    fn rounded_temperature_same_value_is_not_rendered() {
        let mut old = AppState::default();
        old.set_page(Page::Home);
        old.set_nozzle_temperature(215.1, 220.0);

        let mut new = old.clone();
        new.set_nozzle_temperature(215.4, 220.0);

        let commands = render_diff(&old, &new);

        assert!(commands.is_empty());
    }

    #[test]
    fn rounded_temperature_changed_value_is_rendered() {
        let mut old = AppState::default();
        old.set_page(Page::Home);
        old.set_nozzle_temperature(215.4, 220.0);

        let mut new = old.clone();
        new.set_nozzle_temperature(215.6, 220.0);

        let commands = render_diff(&old, &new);

        assert_eq!(commands, vec![HmiCommand::value("n0", 216)]);
    }

    #[test]
    fn same_move_temp_page_move_distance_change_is_rendered() {
        let mut old = AppState::default();
        old.set_page(Page::MoveTemp);
        old.hmi.move_distance = MoveDistance::Mm10;

        let mut new = old.clone();
        new.hmi.move_distance = MoveDistance::Mm30;

        let commands = render_diff(&old, &new);

        assert_eq!(commands, vec![HmiCommand::value("n3", 30)]);
    }

    #[test]
    fn same_print_page_progress_change_is_rendered() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.print.progress_percent = 10;

        let mut new = old.clone();
        new.print.progress_percent = 11;

        let commands = render_diff(&old, &new);

        assert_eq!(commands, vec![HmiCommand::value("n6", 11)]);
    }

    #[test]
    fn same_print_page_filename_change_is_rendered() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.print.filename = None;

        let mut new = old.clone();
        new.print.filename = Some("cube.gcode".to_string());

        let commands = render_diff(&old, &new);

        assert_eq!(commands, vec![HmiCommand::text("g0", "cube.gcode")]);
    }

    #[test]
    fn same_print_page_temperature_change_uses_print_page_components() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.set_nozzle_temperature(209.4, 210.0);
        old.set_bed_temperature(49.4, 50.0);

        let mut new = old.clone();
        new.set_nozzle_temperature(210.0, 220.0);
        new.set_bed_temperature(50.0, 60.0);

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![
                HmiCommand::value("n0", 210),
                HmiCommand::value("n1", 50),
                HmiCommand::text("t8", "220"),
                HmiCommand::text("t9", "60"),
            ]
        );
    }

    #[test]
    fn same_print_page_elapsed_minute_change_is_rendered_as_hours_and_minutes() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.print.elapsed_seconds = 14 * 60 + 59;

        let mut new = old.clone();
        new.print.elapsed_seconds = 15 * 60;

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![HmiCommand::value("n4", 0), HmiCommand::value("n5", 15),]
        );
    }

    #[test]
    fn same_print_page_elapsed_seconds_inside_same_minute_are_not_rendered() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.print.elapsed_seconds = 15 * 60 + 1;

        let mut new = old.clone();
        new.print.elapsed_seconds = 15 * 60 + 59;

        let commands = render_diff(&old, &new);

        assert!(commands.is_empty());
    }

    #[test]
    fn same_print_page_remaining_minute_change_is_rendered_as_hours_and_minutes() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.print.remaining_seconds = Some(2 * 3600 + 41 * 60 + 59);

        let mut new = old.clone();
        new.print.remaining_seconds = Some(2 * 3600 + 42 * 60);

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![HmiCommand::value("n7", 2), HmiCommand::value("n8", 42),]
        );
    }

    #[test]
    fn same_print_page_remaining_time_none_clears_values() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.print.remaining_seconds = Some(3600);

        let mut new = old.clone();
        new.print.remaining_seconds = None;

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![HmiCommand::value("n7", 0), HmiCommand::value("n8", 0)]
        );
    }

    #[test]
    fn same_print_page_paused_status_updates_pause_button() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.printer.status = PrinterStatus::Printing;

        let mut new = old.clone();
        new.printer.status = PrinterStatus::Paused;

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![
                HmiCommand::picture("b5", 5),
                HmiCommand::picture_pressed("b5", 4),
            ]
        );
    }

    #[test]
    fn same_print_page_case_light_change_updates_icon() {
        let mut old = AppState::default();
        old.set_page(Page::Printing);
        old.lights.case_light = false;

        let mut new = old.clone();
        new.lights.case_light = true;

        let commands = render_diff(&old, &new);

        assert_eq!(
            commands,
            vec![
                HmiCommand::picture("b6", 3),
                HmiCommand::picture_pressed("b6", 3),
            ]
        );
    }

    #[test]
    fn unknown_page_returns_no_diff_commands() {
        let mut old = AppState::default();
        old.set_page(Page::Unknown(99));

        let mut new = old.clone();
        new.set_nozzle_temperature(200.0, 210.0);

        let commands = render_diff(&old, &new);

        assert!(commands.is_empty());
    }
}
