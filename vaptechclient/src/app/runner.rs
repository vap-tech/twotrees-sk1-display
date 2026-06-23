use anyhow::Result;

use crate::app::event::AppEvent;
use crate::app::reducers::moonraker::reduce_moonraker_event;
use crate::app::state::{AppState, Page, PrinterStatus};
use crate::hmi::command::HmiCommand;
use crate::hmi::event::HmiEvent;
use crate::moonraker::event::MoonrakerEvent;
use crate::thumbnail::{ThumbnailKey, ThumbnailRequest, ThumbnailSource};
use crate::ui::action_handler::handle_action;
use crate::ui::effect::{MoonrakerRequest, UiEffect};
use crate::ui::render_diff::render_diff;
use crate::ui::render_full::render_full;
use crate::ui::route::resolve_touch;

/// Application core.
///
/// Здесь сходятся события от дисплея и Moonraker. Runner меняет AppState и
/// копит эффекты, но сам не знает, как физически устроены UART/WebSocket.
pub struct AppRunner {
    pub state: AppState,
    pub hmi_commands: Vec<HmiCommand>,
    pub moonraker_requests: Vec<MoonrakerRequest>,
    pub thumbnail_requests: Vec<ThumbnailRequest>,
}

impl AppRunner {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            hmi_commands: Vec::new(),
            moonraker_requests: Vec::new(),
            thumbnail_requests: Vec::new(),
        }
    }

    pub fn drain_hmi_commands(&mut self) -> Vec<HmiCommand> {
        self.hmi_commands.drain(..).collect()
    }

    pub fn drain_moonraker_requests(&mut self) -> Vec<MoonrakerRequest> {
        self.moonraker_requests.drain(..).collect()
    }

    pub fn drain_thumbnail_requests(&mut self) -> Vec<ThumbnailRequest> {
        self.thumbnail_requests.drain(..).collect()
    }

    pub fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Hmi(hmi_event) => {
                self.handle_hmi_event(hmi_event)?;
            }

            AppEvent::Moonraker(moonraker_event) => {
                self.handle_moonraker_event(moonraker_event)?;
            }

            AppEvent::Tick => {}

            AppEvent::Shutdown => {}
        }

        Ok(())
    }

    fn handle_hmi_event(&mut self, event: HmiEvent) -> Result<()> {
        match event {
            HmiEvent::Startup => {
                // Дисплей присылает одиночный 0x91 после питания.
                // В этот момент надо восстановить экран из cache,
                // а не безусловно уходить на Home.
                self.render_startup_page()?;
            }

            HmiEvent::Touch { page, component } => {
                let action = resolve_touch(page, component);
                let effects = handle_action(&mut self.state, action);

                for effect in effects {
                    self.execute_effect(effect)?;
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn render_startup_page(&mut self) -> Result<()> {
        let page = if is_print_active(self.state.printer.status) {
            Page::Printing
        } else {
            Page::Home
        };

        self.state.set_page(page);
        self.execute_effect(UiEffect::hmi(HmiCommand::page(page.id())))?;

        if page == Page::Printing {
            // Дисплей нормально принимает переход на текущую же страницу, поэтому
            // после init безопасно делать полный render page 2 повторно.
            for command in render_full(&self.state) {
                self.execute_effect(UiEffect::hmi(command))?;
            }

            self.request_print_thumbnail();
        }

        Ok(())
    }

    fn handle_moonraker_event(&mut self, event: MoonrakerEvent) -> Result<()> {
        let old_state = self.state.clone();
        let old_filename = self.state.print.filename.clone();

        // Reducer сначала меняет модель состояния, затем renderer сравнивает
        // старое и новое состояние и выдает минимальный набор HMI-команд.
        reduce_moonraker_event(&mut self.state, event);

        let commands = render_diff(&old_state, &self.state);

        self.hmi_commands.extend(commands);

        if self.state.ui.current_page == Page::Printing && old_filename != self.state.print.filename
        {
            self.request_print_thumbnail();
        }

        Ok(())
    }

    fn execute_effect(&mut self, effect: UiEffect) -> Result<()> {
        match effect {
            UiEffect::Hmi(command) => {
                self.hmi_commands.push(command);
            }

            UiEffect::Moonraker(request) => {
                self.moonraker_requests.push(request);
            }

            UiEffect::None => {}
        }

        Ok(())
    }

    fn request_print_thumbnail(&mut self) {
        let Some(filename) = self.state.print.filename.clone() else {
            return;
        };

        let key = ThumbnailKey::print(filename.clone());

        self.thumbnail_requests.push(ThumbnailRequest::Prepare {
            key,
            source: ThumbnailSource::MoonrakerFile {
                path: filename,
                modified: None,
                size: None,
            },
        });
    }
}

fn is_print_active(status: PrinterStatus) -> bool {
    matches!(status, PrinterStatus::Printing | PrinterStatus::Paused)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::state::ActiveOperation;
    use crate::moonraker::event::KlippyState;
    use crate::thumbnail::ThumbnailTarget;

    #[test]
    fn startup_sends_home_page() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::Startup))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Home);
        assert_eq!(runner.hmi_commands, vec![HmiCommand::page(Page::Home.id())]);
    }

    #[test]
    fn startup_during_active_print_sends_print_page_and_full_render() {
        let mut runner = AppRunner::new();

        runner.state.printer.status = PrinterStatus::Printing;
        runner.state.print.filename = Some("cube.gcode".to_string());
        runner.state.print.progress_percent = 32;
        runner.state.print.elapsed_seconds = 2 * 3600 + 41 * 60;
        runner.state.print.remaining_seconds = Some(5 * 3600 + 44 * 60);
        runner.state.set_nozzle_temperature(210.0, 220.0);
        runner.state.set_bed_temperature(50.0, 60.0);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::Startup))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Printing);
        assert_eq!(runner.hmi_commands.first(), Some(&HmiCommand::page(2)));
        assert!(
            runner
                .hmi_commands
                .contains(&HmiCommand::text("g0", "cube.gcode"))
        );
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n6", 32)));
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n4", 2)));
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n5", 41)));
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n7", 5)));
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n8", 44)));
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n0", 210)));
        assert!(runner.hmi_commands.contains(&HmiCommand::value("n1", 50)));
        assert!(runner.hmi_commands.contains(&HmiCommand::text("t8", "220")));
        assert!(runner.hmi_commands.contains(&HmiCommand::text("t9", "60")));
        assert_eq!(runner.thumbnail_requests.len(), 1);
        assert_eq!(
            runner.thumbnail_requests[0].key().target,
            ThumbnailTarget::PrintPage
        );
    }

    #[test]
    fn touch_home_component_1_goes_to_settings() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(0, 1)))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Settings);
        assert_eq!(
            runner.hmi_commands,
            vec![HmiCommand::page(Page::Settings.id())]
        );
    }

    #[test]
    fn locked_navigation_blocks_page_change() {
        let mut runner = AppRunner::new();

        runner.state.lock_navigation(ActiveOperation::Calibration);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(0, 1)))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Home);
        assert!(runner.hmi_commands.is_empty());
    }

    #[test]
    fn touch_calibration_home_all_axes_creates_moonraker_request() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(33, 1)))
            .unwrap();

        assert_eq!(
            runner.state.process.active_operation,
            ActiveOperation::Homing
        );

        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::SendGcode("G28".to_string())]
        );
    }

    #[test]
    fn touch_print_pause_creates_pause_request() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(2, 1)))
            .unwrap();

        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::PausePrint]
        );
    }

    #[test]
    fn moonraker_temperature_event_updates_state_and_renders_home_diff() {
        let mut runner = AppRunner::new();

        runner.state.set_page(Page::Home);

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::temperatures(
                215.0, 220.0, 59.6, 60.0,
            )))
            .unwrap();

        assert_eq!(runner.state.temperatures.nozzle.current, 215.0);
        assert_eq!(runner.state.temperatures.nozzle.target, 220.0);
        assert_eq!(runner.state.temperatures.bed.current, 59.6);
        assert_eq!(runner.state.temperatures.bed.target, 60.0);

        assert_eq!(
            runner.hmi_commands,
            vec![
                HmiCommand::value("n0", 215),
                HmiCommand::value("n1", 220),
                HmiCommand::value("n4", 60),
                HmiCommand::value("n5", 60),
            ]
        );
    }

    #[test]
    fn moonraker_same_rounded_temperature_does_not_render_again() {
        let mut runner = AppRunner::new();

        runner.state.set_page(Page::Home);

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::temperatures(
                215.1, 220.0, 60.0, 60.0,
            )))
            .unwrap();

        runner.hmi_commands.clear();

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::temperatures(
                215.4, 220.0, 60.0, 60.0,
            )))
            .unwrap();

        assert!(runner.hmi_commands.is_empty());
    }

    #[test]
    fn moonraker_klippy_ready_updates_state_without_rendering() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::klippy_state(
                KlippyState::Ready,
            )))
            .unwrap();

        assert!(runner.state.printer.can_accept_commands);
        assert!(runner.hmi_commands.is_empty());
    }
}
