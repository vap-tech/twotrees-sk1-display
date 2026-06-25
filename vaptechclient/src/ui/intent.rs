use crate::app::state::{FanKind, MoveDistance, Screen};

/// Намерение пользователя после разбора touch-события.
///
/// Это граница между сырым HMI `page/component` и логикой приложения. Route
/// только создает intent, а уже следующие слои решают, меняет ли он HMI state
/// или превращается в команду Moonraker.
#[derive(Debug, Clone, PartialEq)]
pub enum UiIntent {
    Navigate(Screen),

    OpenPrintControls,
    SelectMoveDistance(MoveDistance),

    ToggleCaseLight,
    ToggleFan(FanKind),
    SetFanPercent { fan: FanKind, percent: u8 },
    SetNozzleTarget { celsius: i32 },
    SetBedTarget { celsius: i32 },

    HomeAllAxes,
    MoveAxis { axis: Axis, distance: f32 },
    LoadFilament,
    UnloadFilament,

    StartPrint,
    TogglePauseResumePrint,
    PausePrint,
    ResumePrint,
    StopPrint,

    UnknownTouch { page: u8, component: u8 },
    UnknownNumeric { page: u8, component: u8, value: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
    E,
}

impl std::fmt::Display for Axis {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::E => "E",
        };

        formatter.write_str(value)
    }
}

impl UiIntent {
    pub fn is_navigation(&self) -> bool {
        matches!(self, Self::Navigate(_))
    }

    pub fn is_global_stop(&self) -> bool {
        matches!(self, Self::StopPrint)
    }
}

// Старое имя оставляем как совместимость для тестов и внешних модулей.
pub type UiAction = UiIntent;
