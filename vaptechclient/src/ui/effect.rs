use crate::hmi::command::HmiCommand;

/// Эффект, который UI хочет выполнить снаружи.
///
/// Action handler не пишет сам в UART/Moonraker, а возвращает эффекты. Runtime
/// уже решает, куда их доставить. Это держит UI-логику тестируемой.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEffect {
    Hmi(HmiCommand),
    Moonraker(MoonrakerRequest),
    None,
}

/// Потенциальные команды в Moonraker.
///
/// Runtime отправляет наружу только явно разрешённые варианты. Поэтому наличие
/// enum'а не означает, что команда уже физически уходит в принтер.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoonrakerRequest {
    SendGcode(String),

    PausePrint,
    ResumePrint,
    CancelPrint,
    ClearPrintResult,
    StartPrint { filename: String },

    SetNozzleTarget(i32),
    SetBedTarget(i32),

    SetPartFan(u8),
    SetSideFan(u8),
    SetFilterFan(u8),
    SetCaseLight(bool),
}

impl UiEffect {
    pub fn hmi(command: HmiCommand) -> Self {
        Self::Hmi(command)
    }

    pub fn gcode(command: impl Into<String>) -> Self {
        Self::Moonraker(MoonrakerRequest::SendGcode(command.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmi_effect_constructor() {
        assert_eq!(
            UiEffect::hmi(HmiCommand::page(0)),
            UiEffect::Hmi(HmiCommand::page(0))
        );
    }

    #[test]
    fn gcode_effect_constructor() {
        assert_eq!(
            UiEffect::gcode("G28"),
            UiEffect::Moonraker(MoonrakerRequest::SendGcode("G28".to_string()))
        );
    }

    #[test]
    fn effects_can_be_compared() {
        let a = UiEffect::hmi(HmiCommand::page(11));
        let b = UiEffect::hmi(HmiCommand::page(11));

        assert_eq!(a, b);
    }
}
