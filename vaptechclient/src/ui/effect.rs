use crate::hmi::command::HmiCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEffect {
    Hmi(HmiCommand),
    Moonraker(MoonrakerRequest),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoonrakerRequest {
    SendGcode(String),

    PausePrint,
    ResumePrint,
    CancelPrint,

    SetNozzleTarget(i32),
    SetBedTarget(i32),

    SetPartFan(u8),
    SetSideFan(u8),
    SetFilterFan(u8),
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
