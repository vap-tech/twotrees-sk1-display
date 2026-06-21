#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmiEvent {
    /// Display sent startup/init signal.
    Startup,

    /// Touch event from display.
    Touch {
        page: u8,
        component: u8,
    },

    /// Numeric response, usually from `get ...`.
    Numeric(u32),

    /// String response, usually from `get ...txt`.
    Text(String),

    /// Status/ack frame from display.
    Status(HmiStatus),

    /// Valid frame, but parser does not know it yet.
    Unknown(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HmiStatus {
    Ack,
    Error,
    Code(u8),
}

impl HmiEvent {
    pub fn touch(page: u8, component: u8) -> Self {
        Self::Touch { page, component }
    }

    pub fn numeric(value: u32) -> Self {
        Self::Numeric(value)
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn unknown(frame: impl Into<Vec<u8>>) -> Self {
        Self::Unknown(frame.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_constructor() {
        assert_eq!(
            HmiEvent::touch(33, 7),
            HmiEvent::Touch {
                page: 33,
                component: 7
            }
        );
    }

    #[test]
    fn numeric_constructor() {
        assert_eq!(HmiEvent::numeric(123), HmiEvent::Numeric(123));
    }

    #[test]
    fn text_constructor() {
        assert_eq!(
            HmiEvent::text("hello"),
            HmiEvent::Text("hello".to_string())
        );
    }

    #[test]
    fn unknown_constructor() {
        assert_eq!(
            HmiEvent::unknown(vec![0x01, 0x02]),
            HmiEvent::Unknown(vec![0x01, 0x02])
        );
    }

    #[test]
    fn status_is_copy() {
        let a = HmiStatus::Ack;
        let b = a;

        assert_eq!(a, b);
    }

    #[test]
    fn debug_output_contains_variant_name() {
        let event = HmiEvent::Startup;

        assert!(format!("{:?}", event).contains("Startup"));
    }
}
