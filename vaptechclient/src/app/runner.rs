use anyhow::Result;

use crate::app::event::AppEvent;
use crate::app::reducers::moonraker::reduce_moonraker_event;
use crate::app::state::{AppState, Page};
use crate::hmi::command::HmiCommand;
use crate::hmi::event::HmiEvent;
use crate::hmi::transport::HmiTransport;
use crate::moonraker::event::MoonrakerEvent;
use crate::ui::action_handler::handle_action;
use crate::ui::effect::{MoonrakerRequest, UiEffect};
use crate::ui::render_diff::render_diff;
use crate::ui::route::resolve_touch;

pub struct AppRunner<T: HmiTransport> {
    pub state: AppState,
    pub transport: T,
    pub moonraker_requests: Vec<MoonrakerRequest>,
}

impl<T: HmiTransport> AppRunner<T> {
    pub fn new(transport: T) -> Self {
        Self {
            state: AppState::default(),
            transport,
            moonraker_requests: Vec::new(),
        }
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
                self.state.set_page(Page::Home);
                self.execute_effect(UiEffect::hmi(HmiCommand::page(Page::Home.id())))?;
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

    fn handle_moonraker_event(&mut self, event: MoonrakerEvent) -> Result<()> {
        let old_state = self.state.clone();

        reduce_moonraker_event(&mut self.state, event);

        let commands = render_diff(&old_state, &self.state);

        for command in commands {
            self.transport.send(&command)?;
        }

        Ok(())
    }

    fn execute_effect(&mut self, effect: UiEffect) -> Result<()> {
        match effect {
            UiEffect::Hmi(command) => {
                self.transport.send(&command)?;
            }

            UiEffect::Moonraker(request) => {
                self.moonraker_requests.push(request);
            }

            UiEffect::None => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::state::ActiveOperation;
    use crate::hmi::mock_transport::MockTransport;
    use crate::moonraker::event::KlippyState;

    #[test]
    fn startup_sends_home_page() {
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::Startup))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Home);
        assert_eq!(runner.transport.sent[0], b"page 0\xFF\xFF\xFF");
    }

    #[test]
    fn touch_home_component_1_goes_to_settings() {
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(0, 1)))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Settings);
        assert_eq!(runner.transport.sent[0], b"page 11\xFF\xFF\xFF");
    }

    #[test]
    fn locked_navigation_blocks_page_change() {
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

        runner.state.lock_navigation(ActiveOperation::Calibration);

        runner
            .handle_event(AppEvent::hmi(HmiEvent::touch(0, 1)))
            .unwrap();

        assert_eq!(runner.state.ui.current_page, Page::Home);
        assert!(runner.transport.sent.is_empty());
    }

    #[test]
    fn touch_calibration_home_all_axes_creates_moonraker_request() {
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

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
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

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
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

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
            runner.transport.sent,
            vec![
                b"n0.val=215\xFF\xFF\xFF".to_vec(),
                b"n1.val=220\xFF\xFF\xFF".to_vec(),
                b"n4.val=60\xFF\xFF\xFF".to_vec(),
                b"n5.val=60\xFF\xFF\xFF".to_vec(),
            ]
        );
    }

    #[test]
    fn moonraker_same_rounded_temperature_does_not_render_again() {
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

        runner.state.set_page(Page::Home);

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::temperatures(
                215.1, 220.0, 60.0, 60.0,
            )))
            .unwrap();

        runner.transport.sent.clear();

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::temperatures(
                215.4, 220.0, 60.0, 60.0,
            )))
            .unwrap();

        assert!(runner.transport.sent.is_empty());
    }

    #[test]
    fn moonraker_klippy_ready_updates_state_without_rendering() {
        let transport = MockTransport::new();
        let mut runner = AppRunner::new(transport);

        runner
            .handle_event(AppEvent::moonraker(MoonrakerEvent::klippy_state(
                KlippyState::Ready,
            )))
            .unwrap();

        assert!(runner.state.printer.can_accept_commands);
        assert!(runner.transport.sent.is_empty());
    }
}
