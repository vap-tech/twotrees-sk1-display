use anyhow::Result;

use crate::app::event::AppEvent;
use crate::app::state::{AppState, Page};
use crate::hmi::command::HmiCommand;
use crate::hmi::event::HmiEvent;
use crate::hmi::transport::HmiTransport;
use crate::ui::action_handler::handle_action;
use crate::ui::effect::{MoonrakerRequest, UiEffect};
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
}
