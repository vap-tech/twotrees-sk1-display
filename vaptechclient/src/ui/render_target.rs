use crate::app::state::{AppState, Page, PrinterStatus};

/// Visual target, который должен быть показан на HMI прямо сейчас.
///
/// HMI screen выбирает пользователь, а PrinterStatus приходит из Moonraker.
/// Этот слой единственный склеивает их в конкретное визуальное представление.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    Home(HomeMode),
    Files,
    Settings,
    Calibration,
    MoveTemp,
    LoadUnload,
    Print,
    Network,
    Result(ResultMode),
    Error,
    Unknown(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeMode {
    Idle,
    Printing,
    Complete,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultMode {
    Success,
    Failed,
}

impl RenderTarget {
    pub fn page_id(self) -> u16 {
        match self {
            Self::Home(HomeMode::Idle) => 0,
            Self::Home(HomeMode::Printing) | Self::Print => 2,
            Self::Home(HomeMode::Complete) | Self::Result(ResultMode::Success) => 77,
            Self::Home(HomeMode::Cancelled) | Self::Result(ResultMode::Failed) => 77,
            Self::Home(HomeMode::Error) | Self::Error => 56,
            Self::Files => 54,
            Self::Settings => 11,
            Self::Calibration => 33,
            Self::MoveTemp => 3,
            Self::LoadUnload => 4,
            Self::Network => 18,
            Self::Unknown(id) => id,
        }
    }

    pub fn is_print_view(self) -> bool {
        matches!(self, Self::Home(HomeMode::Printing) | Self::Print)
    }
}

pub fn resolve_render_target(state: &AppState) -> RenderTarget {
    match state.hmi.current_screen {
        Page::Home => RenderTarget::Home(resolve_home_mode(state.printer.status)),
        Page::Print | Page::Printing => RenderTarget::Print,
        Page::Files => RenderTarget::Files,
        Page::Settings => RenderTarget::Settings,
        Page::MoveTemp => RenderTarget::MoveTemp,
        Page::LoadUnload => RenderTarget::LoadUnload,
        Page::Calibration => RenderTarget::Calibration,
        Page::Network => RenderTarget::Network,
        Page::Error => RenderTarget::Error,
        Page::Unknown(id) => RenderTarget::Unknown(id),
    }
}

fn resolve_home_mode(status: PrinterStatus) -> HomeMode {
    match status {
        PrinterStatus::Printing | PrinterStatus::Paused => HomeMode::Printing,
        PrinterStatus::Complete => HomeMode::Complete,
        PrinterStatus::Cancelled => HomeMode::Cancelled,
        PrinterStatus::Error => HomeMode::Error,
        PrinterStatus::Unknown
        | PrinterStatus::Standby
        | PrinterStatus::Ready
        | PrinterStatus::Busy => HomeMode::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_standby_resolves_to_home_idle() {
        let mut state = AppState::default();
        state.set_page(Page::Home);
        state.printer.status = PrinterStatus::Standby;

        assert_eq!(
            resolve_render_target(&state),
            RenderTarget::Home(HomeMode::Idle)
        );
    }

    #[test]
    fn home_printing_resolves_to_home_printing() {
        let mut state = AppState::default();
        state.set_page(Page::Home);
        state.printer.status = PrinterStatus::Printing;

        assert_eq!(
            resolve_render_target(&state),
            RenderTarget::Home(HomeMode::Printing)
        );
    }

    #[test]
    fn home_complete_resolves_to_home_complete() {
        let mut state = AppState::default();
        state.set_page(Page::Home);
        state.printer.status = PrinterStatus::Complete;

        assert_eq!(
            resolve_render_target(&state),
            RenderTarget::Home(HomeMode::Complete)
        );
    }

    #[test]
    fn files_printing_resolves_to_files() {
        let mut state = AppState::default();
        state.set_page(Page::Files);
        state.printer.status = PrinterStatus::Printing;

        assert_eq!(resolve_render_target(&state), RenderTarget::Files);
    }

    #[test]
    fn settings_error_resolves_to_settings() {
        let mut state = AppState::default();
        state.set_page(Page::Settings);
        state.printer.status = PrinterStatus::Error;

        assert_eq!(resolve_render_target(&state), RenderTarget::Settings);
    }

    #[test]
    fn render_target_maps_to_hmi_page_id() {
        assert_eq!(RenderTarget::Home(HomeMode::Idle).page_id(), 0);
        assert_eq!(RenderTarget::Home(HomeMode::Printing).page_id(), 2);
        assert_eq!(RenderTarget::Home(HomeMode::Error).page_id(), 56);
        assert_eq!(RenderTarget::Home(HomeMode::Cancelled).page_id(), 77);
        assert_eq!(RenderTarget::Home(HomeMode::Complete).page_id(), 77);
    }
}
