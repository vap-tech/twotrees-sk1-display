use crate::app::state::{AppState, ConnectionStatus, KlipperStatus, PrinterStatus};
use crate::moonraker::event::{
    HeaterKind, KlippyState as MoonrakerKlippyState, MoonrakerEvent,
    PrinterStatus as MoonrakerPrinterStatus,
};

/// Применяет событие Moonraker к AppState.
///
/// Reducer ничего не отправляет наружу. Его задача - только привести cache
/// состояния к новому факту, полученному из websocket.
pub fn reduce_moonraker_event(state: &mut AppState, event: MoonrakerEvent) {
    match event {
        MoonrakerEvent::Connected => {
            state.connection.moonraker = ConnectionStatus::Connected;
        }

        MoonrakerEvent::Disconnected => {
            state.connection.moonraker = ConnectionStatus::Disconnected;
            state.connection.klipper = KlipperStatus::Unknown;
            state.printer.can_accept_commands = false;
        }

        MoonrakerEvent::KlippyStateChanged(klippy_state) => {
            apply_klippy_state(state, klippy_state);
        }

        MoonrakerEvent::PrinterStatusChanged(printer_status) => {
            apply_printer_status(state, printer_status);
        }

        MoonrakerEvent::Temperatures {
            nozzle_current,
            nozzle_target,
            bed_current,
            bed_target,
        } => {
            state.set_nozzle_temperature(nozzle_current, nozzle_target);
            state.set_bed_temperature(bed_current, bed_target);
        }

        MoonrakerEvent::HeaterUpdate {
            heater,
            current,
            target,
        } => {
            apply_heater_update(state, heater, current, target);
        }

        MoonrakerEvent::PrintProgress {
            filename,
            progress_percent,
            elapsed_seconds,
            remaining_seconds,
        } => {
            apply_print_progress(
                state,
                filename,
                progress_percent,
                elapsed_seconds,
                remaining_seconds,
            );
        }

        MoonrakerEvent::FileListChanged
        | MoonrakerEvent::GcodeResponse(_)
        | MoonrakerEvent::Error(_)
        | MoonrakerEvent::Unknown => {}
    }
}

fn apply_klippy_state(state: &mut AppState, klippy_state: MoonrakerKlippyState) {
    match klippy_state {
        MoonrakerKlippyState::Ready => {
            state.connection.klipper = KlipperStatus::Ready;
            state.printer.can_accept_commands = true;
        }

        MoonrakerKlippyState::Busy => {
            state.connection.klipper = KlipperStatus::Busy;
            state.printer.can_accept_commands = false;
        }

        MoonrakerKlippyState::Error => {
            state.connection.klipper = KlipperStatus::Error;
            state.printer.status = PrinterStatus::Error;
            state.printer.can_accept_commands = false;
        }

        MoonrakerKlippyState::Shutdown => {
            state.connection.klipper = KlipperStatus::Shutdown;
            state.printer.can_accept_commands = false;
        }

        MoonrakerKlippyState::Unknown => {
            state.connection.klipper = KlipperStatus::Unknown;
            state.printer.can_accept_commands = false;
        }
    }
}

fn apply_printer_status(state: &mut AppState, printer_status: MoonrakerPrinterStatus) {
    match printer_status {
        MoonrakerPrinterStatus::Standby => {
            state.printer.status = PrinterStatus::Standby;
            state.printer.can_accept_commands = true;
        }

        MoonrakerPrinterStatus::Printing => {
            state.printer.status = PrinterStatus::Printing;
            state.printer.can_accept_commands = false;
        }

        MoonrakerPrinterStatus::Paused => {
            state.printer.status = PrinterStatus::Paused;
            state.printer.can_accept_commands = true;
        }

        MoonrakerPrinterStatus::Complete => {
            state.printer.status = PrinterStatus::Complete;
            state.printer.can_accept_commands = true;
        }

        MoonrakerPrinterStatus::Cancelled => {
            state.printer.status = PrinterStatus::Cancelled;
            state.printer.can_accept_commands = true;
        }

        MoonrakerPrinterStatus::Error => {
            state.printer.status = PrinterStatus::Error;
            state.printer.can_accept_commands = false;
        }

        MoonrakerPrinterStatus::Unknown => {
            state.printer.status = PrinterStatus::Unknown;
            state.printer.can_accept_commands = false;
        }
    }
}

fn apply_heater_update(
    state: &mut AppState,
    heater: HeaterKind,
    current: Option<f32>,
    target: Option<f32>,
) {
    let target_heater = match heater {
        HeaterKind::Nozzle => &mut state.temperatures.nozzle,
        HeaterKind::Bed => &mut state.temperatures.bed,
    };

    if let Some(value) = current {
        target_heater.current = value;
    }

    if let Some(value) = target {
        target_heater.target = value;
    }
}

fn apply_print_progress(
    state: &mut AppState,
    filename: Option<String>,
    progress_percent: Option<u8>,
    elapsed_seconds: Option<u32>,
    remaining_seconds: Option<u32>,
) {
    if let Some(value) = filename {
        state.print.filename = Some(value);
    }

    if let Some(value) = progress_percent {
        state.print.progress_percent = value.min(100);
    }

    if let Some(value) = elapsed_seconds {
        state.print.elapsed_seconds = value;
    }

    if let Some(value) = remaining_seconds {
        state.print.remaining_seconds = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_updates_moonraker_connection() {
        let mut state = AppState::default();

        reduce_moonraker_event(&mut state, MoonrakerEvent::Connected);

        assert_eq!(state.connection.moonraker, ConnectionStatus::Connected);
    }

    #[test]
    fn disconnected_resets_connection_state() {
        let mut state = AppState::default();

        state.connection.moonraker = ConnectionStatus::Connected;
        state.connection.klipper = KlipperStatus::Ready;
        state.printer.can_accept_commands = true;

        reduce_moonraker_event(&mut state, MoonrakerEvent::Disconnected);

        assert_eq!(state.connection.moonraker, ConnectionStatus::Disconnected);
        assert_eq!(state.connection.klipper, KlipperStatus::Unknown);
        assert!(!state.printer.can_accept_commands);
    }

    #[test]
    fn klippy_ready_allows_commands() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::klippy_state(MoonrakerKlippyState::Ready),
        );

        assert_eq!(state.connection.klipper, KlipperStatus::Ready);
        assert!(state.printer.can_accept_commands);
    }

    #[test]
    fn klippy_busy_blocks_commands() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::klippy_state(MoonrakerKlippyState::Busy),
        );

        assert_eq!(state.connection.klipper, KlipperStatus::Busy);
        assert!(!state.printer.can_accept_commands);
    }

    #[test]
    fn printer_status_printing_blocks_generic_commands() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::printer_status(MoonrakerPrinterStatus::Printing),
        );

        assert_eq!(state.printer.status, PrinterStatus::Printing);
        assert!(!state.printer.can_accept_commands);
    }

    #[test]
    fn printer_status_printing_from_home_keeps_user_screen() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::printer_status(MoonrakerPrinterStatus::Printing),
        );

        assert_eq!(state.hmi.current_screen, crate::app::state::Page::Home);
    }

    #[test]
    fn printer_status_printing_does_not_steal_non_home_page() {
        let mut state = AppState::default();
        state.set_page(crate::app::state::Page::Files);

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::printer_status(MoonrakerPrinterStatus::Printing),
        );

        assert_eq!(state.hmi.current_screen, crate::app::state::Page::Files);
    }

    #[test]
    fn printer_status_paused_allows_commands() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::printer_status(MoonrakerPrinterStatus::Paused),
        );

        assert_eq!(state.printer.status, PrinterStatus::Paused);
        assert!(state.printer.can_accept_commands);
    }

    #[test]
    fn temperatures_update_state() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::temperatures(215.5, 220.0, 59.5, 60.0),
        );

        assert_eq!(state.temperatures.nozzle.current, 215.5);
        assert_eq!(state.temperatures.nozzle.target, 220.0);
        assert_eq!(state.temperatures.bed.current, 59.5);
        assert_eq!(state.temperatures.bed.target, 60.0);
    }

    #[test]
    fn heater_update_current_only_updates_current() {
        let mut state = AppState::default();

        state.set_nozzle_temperature(200.0, 220.0);

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::HeaterUpdate {
                heater: HeaterKind::Nozzle,
                current: Some(215.5),
                target: None,
            },
        );

        assert_eq!(state.temperatures.nozzle.current, 215.5);
        assert_eq!(state.temperatures.nozzle.target, 220.0);
    }

    #[test]
    fn heater_update_target_only_updates_target() {
        let mut state = AppState::default();

        state.set_bed_temperature(50.0, 0.0);

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::HeaterUpdate {
                heater: HeaterKind::Bed,
                current: None,
                target: Some(60.0),
            },
        );

        assert_eq!(state.temperatures.bed.current, 50.0);
        assert_eq!(state.temperatures.bed.target, 60.0);
    }

    #[test]
    fn print_progress_updates_all_fields() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::PrintProgress {
                filename: Some("cube.gcode".to_string()),
                progress_percent: Some(42),
                elapsed_seconds: Some(120),
                remaining_seconds: Some(300),
            },
        );

        assert_eq!(state.print.filename, Some("cube.gcode".to_string()));
        assert_eq!(state.print.progress_percent, 42);
        assert_eq!(state.print.elapsed_seconds, 120);
        assert_eq!(state.print.remaining_seconds, Some(300));
    }

    #[test]
    fn print_progress_partial_update_preserves_old_values() {
        let mut state = AppState::default();

        state.print.filename = Some("cube.gcode".to_string());
        state.print.progress_percent = 40;
        state.print.elapsed_seconds = 100;
        state.print.remaining_seconds = Some(500);

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::PrintProgress {
                filename: None,
                progress_percent: Some(41),
                elapsed_seconds: None,
                remaining_seconds: None,
            },
        );

        assert_eq!(state.print.filename, Some("cube.gcode".to_string()));
        assert_eq!(state.print.progress_percent, 41);
        assert_eq!(state.print.elapsed_seconds, 100);
        assert_eq!(state.print.remaining_seconds, Some(500));
    }

    #[test]
    fn print_progress_percent_is_clamped_to_100() {
        let mut state = AppState::default();

        reduce_moonraker_event(
            &mut state,
            MoonrakerEvent::PrintProgress {
                filename: None,
                progress_percent: Some(150),
                elapsed_seconds: None,
                remaining_seconds: None,
            },
        );

        assert_eq!(state.print.progress_percent, 100);
    }
}
