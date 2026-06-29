use anyhow::Result;

use crate::app::event::AppEvent;
use crate::app::reducers::moonraker::reduce_moonraker_event;
use crate::app::state::AppState;
use crate::hmi::command::HmiCommand;
use crate::hmi::event::HmiEvent;
use crate::moonraker::event::MoonrakerEvent;
use crate::thumbnail::ThumbnailRequest;
use crate::ui::action_handler::{
    apply_hmi_intent, intent_is_blocked_by_printer_state, moonraker_requests_for_intent,
};
use crate::ui::effect::{MoonrakerRequest, UiEffect};
use crate::ui::intent::UiIntent;
use crate::ui::render_diff::render_diff;
use crate::ui::render_full::render_full_target;
use crate::ui::render_target::{RenderTarget, resolve_render_target};
use crate::ui::route::{route_numeric, route_touch};

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
                self.handle_intent(route_touch(page, component));
            }

            HmiEvent::NumericInput {
                page,
                component,
                value,
            } => {
                self.handle_intent(route_numeric(page, component, value));
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_intent(&mut self, intent: UiIntent) {
        if intent_is_blocked_by_printer_state(&self.state, &intent) {
            tracing::debug!(
                ?intent,
                printer_status = ?self.state.printer.status,
                "UI intent blocked by printer state"
            );
            return;
        }

        let old_state = self.state.clone();
        let old_target = resolve_render_target(&old_state);

        apply_hmi_intent(&mut self.state, &intent);

        self.moonraker_requests
            .extend(moonraker_requests_for_intent(&self.state, &intent));

        self.hmi_commands
            .extend(render_diff(&old_state, &self.state));

        let new_target = resolve_render_target(&self.state);
        if old_target != new_target {
            self.request_thumbnail_for_target(new_target);
        }
    }

    fn render_startup_page(&mut self) -> Result<()> {
        let target = resolve_render_target(&self.state);

        self.execute_effect(UiEffect::hmi(HmiCommand::page(target.page_id())))?;

        // Дисплей нормально принимает переход на текущую же страницу, поэтому
        // после init безопасно делать полный render текущего visual target.
        for command in render_full_target(target, &self.state) {
            self.execute_effect(UiEffect::hmi(command))?;
        }

        self.request_thumbnail_for_target(target);

        Ok(())
    }

    fn handle_moonraker_event(&mut self, event: MoonrakerEvent) -> Result<()> {
        let old_state = self.state.clone();
        let old_target = resolve_render_target(&old_state);
        let old_filename = old_state.print.filename.clone();

        // Reducer сначала меняет модель состояния, затем renderer сравнивает
        // старое и новое состояние и выдает минимальный набор HMI-команд.
        reduce_moonraker_event(&mut self.state, event);

        let commands = render_diff(&old_state, &self.state);
        let new_target = resolve_render_target(&self.state);
        let filename_changed = old_filename != self.state.print.filename;

        self.hmi_commands.extend(commands);

        if old_target != new_target || filename_changed {
            self.request_thumbnail_for_target(new_target);
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

    fn request_thumbnail_for_target(&mut self, target: RenderTarget) {
        if let Some(request) = target.thumbnail_request(&self.state) {
            self.thumbnail_requests.push(request);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::state::{ActiveOperation, FanKind, Page, PrinterStatus};
    use crate::moonraker::event::KlippyState;
    use crate::thumbnail::{ThumbnailKey, ThumbnailTarget};

    #[test]
    fn startup_sends_home_page() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::Startup))
            .unwrap();

        assert_eq!(runner.state.hmi.current_screen, Page::Home);
        assert_eq!(
            runner.hmi_commands,
            vec![
                HmiCommand::page(Page::Home.id()),
                HmiCommand::value("n0", 0),
                HmiCommand::value("n1", 0),
                HmiCommand::value("n4", 0),
                HmiCommand::value("n5", 0),
                HmiCommand::picture("b5", 2),
                HmiCommand::picture_pressed("b5", 2),
                HmiCommand::picture("b6", 2),
                HmiCommand::picture_pressed("b6", 2),
            ]
        );
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

        // Startup не меняет пользовательский screen: HMI остается на Home, а
        // RenderTarget решает показать physical page 2, потому что идет печать.
        assert_eq!(runner.state.hmi.current_screen, Page::Home);
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
    fn startup_after_completed_print_requests_result_thumbnail() {
        let mut runner = AppRunner::new();

        runner.state.printer.status = PrinterStatus::Complete;
        runner.state.print.filename = Some("cube.gcode".to_string());

        runner
            .handle_event(AppEvent::hmi(HmiEvent::Startup))
            .unwrap();

        assert_eq!(runner.state.hmi.current_screen, Page::Home);
        assert_eq!(runner.hmi_commands.first(), Some(&HmiCommand::page(77)));
        assert!(
            runner
                .hmi_commands
                .contains(&HmiCommand::raw("print_done.cp0.close()"))
        );
        assert!(
            runner
                .hmi_commands
                .contains(&HmiCommand::visible("print_done.cp0", false))
        );
        assert_eq!(runner.thumbnail_requests.len(), 1);
        assert_eq!(
            runner.thumbnail_requests[0].key().target,
            ThumbnailTarget::ResultPage
        );
    }

    #[test]
    fn moonraker_complete_status_requests_result_thumbnail() {
        let mut runner = AppRunner::new();

        runner.state.printer.status = PrinterStatus::Printing;
        runner.state.print.filename = Some("cube.gcode".to_string());

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::printer_status(
                crate::moonraker::event::PrinterStatus::Complete,
            )))
            .unwrap();

        assert_eq!(runner.hmi_commands.first(), Some(&HmiCommand::page(77)));
        assert_eq!(runner.thumbnail_requests.len(), 1);
        assert_eq!(
            runner.thumbnail_requests[0].key(),
            &ThumbnailKey::result("cube.gcode")
        );
    }

    #[test]
    fn touch_home_component_1_goes_to_move_temp() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(0, 1)))
            .unwrap();

        assert_eq!(runner.state.hmi.current_screen, Page::MoveTemp);
        assert_eq!(
            runner.hmi_commands,
            vec![
                HmiCommand::page(Page::MoveTemp.id()),
                HmiCommand::value("n0", 0),
                HmiCommand::value("n1", 0),
                HmiCommand::value("n4", 0),
                HmiCommand::value("n5", 0),
                HmiCommand::value("n3", 10),
            ]
        );
    }

    #[test]
    fn locked_navigation_blocks_page_change() {
        let mut runner = AppRunner::new();

        runner.state.lock_navigation(ActiveOperation::Calibration);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(0, 1)))
            .unwrap();

        assert_eq!(runner.state.hmi.current_screen, Page::Home);
        assert!(runner.hmi_commands.is_empty());
    }

    #[test]
    fn touch_calibration_home_all_axes_creates_moonraker_request() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(33, 5)))
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
    fn touch_print_component_5_pauses_when_printing() {
        let mut runner = AppRunner::new();

        runner.state.printer.status = PrinterStatus::Printing;

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(2, 5)))
            .unwrap();

        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::PausePrint]
        );
    }

    #[test]
    fn touch_print_component_5_resumes_when_paused() {
        let mut runner = AppRunner::new();

        runner.state.printer.status = PrinterStatus::Paused;

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(2, 5)))
            .unwrap();

        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::ResumePrint]
        );
    }

    #[test]
    fn touch_print_component_6_requests_case_light_toggle() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(2, 6)))
            .unwrap();

        assert!(!runner.state.lights.case_light);
        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::SetCaseLight(true)]
        );
    }

    #[test]
    fn touch_print_result_component_6_requests_clear_result() {
        let mut runner = AppRunner::new();

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(77, 6)))
            .unwrap();

        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::ClearPrintResult]
        );
    }

    #[test]
    fn touch_print_result_component_5_requests_reprint() {
        let mut runner = AppRunner::new();
        runner.state.print.filename = Some("cube.gcode".to_string());

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(77, 5)))
            .unwrap();

        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::StartPrint {
                filename: "cube.gcode".to_string()
            }]
        );
    }

    #[test]
    fn touch_fans_component_0_returns_home() {
        let mut runner = AppRunner::new();

        runner.state.set_page(Page::Fans);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(6, 0)))
            .unwrap();

        assert_eq!(runner.state.hmi.current_screen, Page::Home);
        assert_eq!(runner.hmi_commands.first(), Some(&HmiCommand::page(0)));
    }

    #[test]
    fn touch_fans_component_0_during_print_requests_print_thumbnail() {
        let mut runner = AppRunner::new();

        runner.state.set_page(Page::Fans);
        runner.state.printer.status = PrinterStatus::Printing;
        runner.state.print.filename = Some("cube.gcode".to_string());

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(6, 0)))
            .unwrap();

        assert_eq!(runner.state.hmi.current_screen, Page::Home);
        assert_eq!(runner.hmi_commands.first(), Some(&HmiCommand::page(2)));
        assert_eq!(runner.thumbnail_requests.len(), 1);
        assert_eq!(
            runner.thumbnail_requests[0].key(),
            &ThumbnailKey::print("cube.gcode")
        );
    }

    #[test]
    fn numeric_fan_slider_creates_request_without_mutating_state() {
        let mut runner = AppRunner::new();

        runner.state.set_page(Page::Fans);
        runner.state.set_fan_percent(FanKind::Side, 0);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::numeric_input(6, 1, 42)))
            .unwrap();

        assert_eq!(runner.state.fans.side.percent, 0);
        assert_eq!(
            runner.moonraker_requests,
            vec![MoonrakerRequest::SetSideFan(42)]
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
