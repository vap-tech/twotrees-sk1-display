use crate::hmi::event::HmiEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Hmi(HmiEvent),

    Tick,

    Shutdown,
}

impl AppEvent {
    pub fn hmi(event: HmiEvent) -> Self {
        Self::Hmi(event)
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

        assert_eq!(
            AppEvent::hmi(hmi_event.clone()),
            AppEvent::Hmi(hmi_event)
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

    #[test]
    fn debug_output_contains_variant_name() {
        let event = AppEvent::Tick;

        assert!(format!("{:?}", event).contains("Tick"));
    }
}
