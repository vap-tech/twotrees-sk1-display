#[derive(Debug, Clone, PartialEq)]
pub enum MoonrakerEvent {
    Connected,
    Disconnected,

    KlippyStateChanged(KlippyState),

    PrinterStatusChanged(PrinterStatus),

    Temperatures {
        nozzle_current: f32,
        nozzle_target: f32,
        bed_current: f32,
        bed_target: f32,
    },

    PrintProgress {
        filename: Option<String>,
        progress_percent: Option<u8>,
        elapsed_seconds: Option<u32>,
        remaining_seconds: Option<u32>,
    },

    HeaterUpdate {
        heater: HeaterKind,
        current: Option<f32>,
        target: Option<f32>,
    },

    CaseLightChanged(bool),

    FileListChanged,

    GcodeResponse(String),

    Error(String),

    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KlippyState {
    Unknown,
    Ready,
    Busy,
    Error,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterStatus {
    Unknown,
    Standby,
    Printing,
    Paused,
    Complete,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaterKind {
    Nozzle,
    Bed,
}

impl MoonrakerEvent {
    pub fn connected() -> Self {
        Self::Connected
    }

    pub fn disconnected() -> Self {
        Self::Disconnected
    }

    pub fn klippy_state(state: KlippyState) -> Self {
        Self::KlippyStateChanged(state)
    }

    pub fn printer_status(status: PrinterStatus) -> Self {
        Self::PrinterStatusChanged(status)
    }

    pub fn temperatures(
        nozzle_current: f32,
        nozzle_target: f32,
        bed_current: f32,
        bed_target: f32,
    ) -> Self {
        Self::Temperatures {
            nozzle_current,
            nozzle_target,
            bed_current,
            bed_target,
        }
    }

    pub fn gcode_response(value: impl Into<String>) -> Self {
        Self::GcodeResponse(value.into())
    }

    pub fn error(value: impl Into<String>) -> Self {
        Self::Error(value.into())
    }
}

impl KlippyState {
    pub fn from_moonraker(value: &str) -> Self {
        match value {
            "ready" => Self::Ready,
            "busy" => Self::Busy,
            "error" => Self::Error,
            "shutdown" => Self::Shutdown,
            _ => Self::Unknown,
        }
    }
}

impl PrinterStatus {
    pub fn from_print_stats(value: &str) -> Self {
        match value {
            "standby" => Self::Standby,
            "printing" => Self::Printing,
            "paused" => Self::Paused,
            "complete" => Self::Complete,
            "cancelled" => Self::Cancelled,
            "error" => Self::Error,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_constructor() {
        assert_eq!(MoonrakerEvent::connected(), MoonrakerEvent::Connected);
    }

    #[test]
    fn disconnected_constructor() {
        assert_eq!(MoonrakerEvent::disconnected(), MoonrakerEvent::Disconnected);
    }

    #[test]
    fn klippy_state_constructor() {
        assert_eq!(
            MoonrakerEvent::klippy_state(KlippyState::Ready),
            MoonrakerEvent::KlippyStateChanged(KlippyState::Ready)
        );
    }

    #[test]
    fn printer_status_constructor() {
        assert_eq!(
            MoonrakerEvent::printer_status(PrinterStatus::Printing),
            MoonrakerEvent::PrinterStatusChanged(PrinterStatus::Printing)
        );
    }

    #[test]
    fn temperatures_constructor() {
        assert_eq!(
            MoonrakerEvent::temperatures(215.0, 220.0, 59.5, 60.0),
            MoonrakerEvent::Temperatures {
                nozzle_current: 215.0,
                nozzle_target: 220.0,
                bed_current: 59.5,
                bed_target: 60.0,
            }
        );
    }

    #[test]
    fn gcode_response_constructor() {
        assert_eq!(
            MoonrakerEvent::gcode_response("ok"),
            MoonrakerEvent::GcodeResponse("ok".to_string())
        );
    }

    #[test]
    fn error_constructor() {
        assert_eq!(
            MoonrakerEvent::error("connection failed"),
            MoonrakerEvent::Error("connection failed".to_string())
        );
    }

    #[test]
    fn klippy_state_from_moonraker_ready() {
        assert_eq!(KlippyState::from_moonraker("ready"), KlippyState::Ready);
    }

    #[test]
    fn klippy_state_from_moonraker_busy() {
        assert_eq!(KlippyState::from_moonraker("busy"), KlippyState::Busy);
    }

    #[test]
    fn klippy_state_from_moonraker_unknown() {
        assert_eq!(
            KlippyState::from_moonraker("whatever"),
            KlippyState::Unknown
        );
    }

    #[test]
    fn printer_status_from_print_stats_printing() {
        assert_eq!(
            PrinterStatus::from_print_stats("printing"),
            PrinterStatus::Printing
        );
    }

    #[test]
    fn printer_status_from_print_stats_standby() {
        assert_eq!(
            PrinterStatus::from_print_stats("standby"),
            PrinterStatus::Standby
        );
    }

    #[test]
    fn printer_status_from_print_stats_unknown() {
        assert_eq!(
            PrinterStatus::from_print_stats("idle"),
            PrinterStatus::Unknown
        );
    }

    #[test]
    fn copy_enums_can_be_reused_after_assignment() {
        let a = KlippyState::Ready;
        let b = a;

        assert_eq!(a, b);
    }

    #[test]
    fn debug_output_contains_variant_name() {
        let event = MoonrakerEvent::connected();

        assert!(format!("{:?}", event).contains("Connected"));
    }
}
