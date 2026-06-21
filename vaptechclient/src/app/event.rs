use crate::hmi::event::HmiEvent;
use crate::moonraker::event::MoonrakerEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    Hmi(HmiEvent),
    Moonraker(MoonrakerEvent),
    Tick,
    Shutdown,
}

impl AppEvent {
    pub fn hmi(event: HmiEvent) -> Self {
        Self::Hmi(event)
    }

    pub fn moonraker(event: MoonrakerEvent) -> Self {
        Self::Moonraker(event)
    }

    pub fn tick() -> Self {
        Self::Tick
    }

    pub fn shutdown() -> Self {
        Self::Shutdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmi_event_constructor() {
        let hmi_event = HmiEvent::touch(33, 7);

        assert_eq!(AppEvent::hmi(hmi_event.clone()), AppEvent::Hmi(hmi_event));
    }

    #[test]
    fn moonraker_event_constructor() {
        let event = MoonrakerEvent::connected();

        assert_eq!(
            AppEvent::moonraker(event.clone()),
            AppEvent::Moonraker(event)
        );
    }

    #[test]
    fn tick_constructor() {
        assert_eq!(AppEvent::tick(), AppEvent::Tick);
    }

    #[test]
    fn shutdown_constructor() {
        assert_eq!(AppEvent::shutdown(), AppEvent::Shutdown);
    }
}
