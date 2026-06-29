use crate::app::state::{AppState, Page, PrinterStatus};
use crate::thumbnail::{ThumbnailKey, ThumbnailRequest, ThumbnailSource, ThumbnailTarget};

/// Visual target, который должен быть показан на HMI прямо сейчас.
///
/// HMI screen выбирает пользователь, а PrinterStatus приходит из Moonraker.
/// Этот слой единственный склеивает их в конкретное визуальное представление.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    Home(HomeMode),
    Files,
    Settings,
    Fans,
    Calibration,
    MoveTemp,
    LoadUnload,
    Print,
    Network,
    Faq,
    OnlineManual,
    Contact,
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
            Self::Home(HomeMode::Error) | Self::Error => 77,
            Self::Files => 54,
            Self::Settings => 11,
            Self::Fans => 6,
            Self::Calibration => 33,
            Self::MoveTemp => 3,
            Self::LoadUnload => 4,
            Self::Network => 18,
            Self::Faq => 21,
            Self::OnlineManual => 52,
            Self::Contact => 53,
            Self::Unknown(id) => id,
        }
    }

    pub fn is_print_view(self) -> bool {
        matches!(self, Self::Home(HomeMode::Printing) | Self::Print)
    }

    pub fn is_result_view(self) -> bool {
        matches!(
            self,
            Self::Home(HomeMode::Complete | HomeMode::Cancelled) | Self::Result(_)
        )
    }

    pub fn wants_thumbnail(self) -> bool {
        self.is_print_view() || self.is_result_view()
    }

    pub fn thumbnail_request(self, state: &AppState) -> Option<ThumbnailRequest> {
        let filename = state.print.filename.clone()?;

        let key = if self.is_print_view() {
            ThumbnailKey::print(filename.clone())
        } else if self.is_result_view() {
            ThumbnailKey::result(filename.clone())
        } else {
            return None;
        };

        Some(ThumbnailRequest::Prepare {
            key,
            source: ThumbnailSource::MoonrakerFile {
                path: filename,
                modified: None,
                size: None,
            },
        })
    }

    pub fn accepts_thumbnail(self, state: &AppState, key: &ThumbnailKey) -> bool {
        match key.target {
            ThumbnailTarget::PrintPage => {
                self.is_print_view()
                    && state.print.filename.as_deref() == Some(key.file_path.as_str())
            }
            ThumbnailTarget::FileSlot { slot } => {
                self == Self::Files
                    && state
                        .files
                        .visible_file_at(slot)
                        .map(|file| file.path.as_str())
                        == Some(key.file_path.as_str())
            }
            ThumbnailTarget::PreviewPage => false,
            ThumbnailTarget::ResultPage => {
                self.is_result_view()
                    && state.print.filename.as_deref() == Some(key.file_path.as_str())
            }
        }
    }
}

pub fn resolve_render_target(state: &AppState) -> RenderTarget {
    match state.hmi.current_screen {
        Page::Home => resolve_home_target(state),
        Page::Print | Page::Printing => RenderTarget::Print,
        Page::Files => RenderTarget::Files,
        Page::Settings => RenderTarget::Settings,
        Page::Fans => RenderTarget::Fans,
        Page::MoveTemp => RenderTarget::MoveTemp,
        Page::LoadUnload => RenderTarget::LoadUnload,
        Page::Calibration => RenderTarget::Calibration,
        Page::Network => RenderTarget::Network,
        Page::Faq => RenderTarget::Faq,
        Page::OnlineManual => RenderTarget::OnlineManual,
        Page::Contact => RenderTarget::Contact,
        Page::Error => RenderTarget::Error,
        Page::Unknown(id) => RenderTarget::Unknown(id),
    }
}

fn resolve_home_target(state: &AppState) -> RenderTarget {
    if state.printer.status == PrinterStatus::Error {
        return RenderTarget::Result(ResultMode::Failed);
    }

    RenderTarget::Home(resolve_home_mode(state.printer.status))
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

    use crate::app::state::FileSlot;

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
    fn home_error_resolves_to_failed_result() {
        let mut state = AppState::default();
        state.set_page(Page::Home);
        state.printer.status = PrinterStatus::Error;

        assert_eq!(
            resolve_render_target(&state),
            RenderTarget::Result(ResultMode::Failed)
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
        assert_eq!(RenderTarget::Home(HomeMode::Error).page_id(), 77);
        assert_eq!(RenderTarget::Home(HomeMode::Cancelled).page_id(), 77);
        assert_eq!(RenderTarget::Home(HomeMode::Complete).page_id(), 77);
        assert_eq!(RenderTarget::Faq.page_id(), 21);
        assert_eq!(RenderTarget::OnlineManual.page_id(), 52);
        assert_eq!(RenderTarget::Contact.page_id(), 53);
    }

    #[test]
    fn print_target_creates_print_thumbnail_request() {
        let mut state = AppState::default();
        state.print.filename = Some("cube.gcode".to_string());

        let request = RenderTarget::Print.thumbnail_request(&state).unwrap();

        assert_eq!(request.key(), &ThumbnailKey::print("cube.gcode"));
    }

    #[test]
    fn result_target_creates_result_thumbnail_request() {
        let mut state = AppState::default();
        state.print.filename = Some("cube.gcode".to_string());

        let request = RenderTarget::Result(ResultMode::Success)
            .thumbnail_request(&state)
            .unwrap();

        assert_eq!(request.key(), &ThumbnailKey::result("cube.gcode"));
    }

    #[test]
    fn non_thumbnail_target_has_no_thumbnail_request() {
        let mut state = AppState::default();
        state.print.filename = Some("cube.gcode".to_string());

        assert!(RenderTarget::Fans.thumbnail_request(&state).is_none());
    }

    #[test]
    fn thumbnail_target_without_filename_has_no_request() {
        let state = AppState::default();

        assert!(RenderTarget::Print.thumbnail_request(&state).is_none());
    }

    #[test]
    fn print_target_accepts_matching_print_thumbnail() {
        let mut state = AppState::default();
        state.print.filename = Some("cube.gcode".to_string());

        assert!(RenderTarget::Print.accepts_thumbnail(&state, &ThumbnailKey::print("cube.gcode")));
        assert!(
            !RenderTarget::Print.accepts_thumbnail(&state, &ThumbnailKey::print("other.gcode"))
        );
    }

    #[test]
    fn files_target_accepts_matching_visible_file_slot_thumbnail() {
        let mut state = AppState::default();
        state
            .files
            .set_visible_slot(0, Some(FileSlot::new("A.gcode", "A.gcode")));

        assert!(
            RenderTarget::Files.accepts_thumbnail(&state, &ThumbnailKey::file_slot("A.gcode", 0))
        );
        assert!(
            !RenderTarget::Files.accepts_thumbnail(&state, &ThumbnailKey::file_slot("A.gcode", 1))
        );
        assert!(
            !RenderTarget::Files.accepts_thumbnail(&state, &ThumbnailKey::file_slot("B.gcode", 0))
        );
    }

    #[test]
    fn result_target_accepts_matching_result_thumbnail() {
        let mut state = AppState::default();
        state.print.filename = Some("cube.gcode".to_string());

        assert!(
            RenderTarget::Result(ResultMode::Success)
                .accepts_thumbnail(&state, &ThumbnailKey::result("cube.gcode"))
        );
        assert!(
            !RenderTarget::Result(ResultMode::Success)
                .accepts_thumbnail(&state, &ThumbnailKey::result("other.gcode"))
        );
    }
}
