#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmiEvent {
    Touch {
        page: u8,
        component: u8,
    },

    Numeric(u32),

    Text(String),

    Startup,

    Ack,

    Unknown(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_events_are_equal() {
        let a = HmiEvent::Touch {
            page: 33,
            component: 7,
        };

        let b = HmiEvent::Touch {
            page: 33,
            component: 7,
        };

        assert_eq!(a, b);
    }

    #[test]
    fn unknown_events_are_equal() {
        let a = HmiEvent::Unknown(vec![1, 2, 3]);
        let b = HmiEvent::Unknown(vec![1, 2, 3]);

        assert_eq!(a, b);
    }

    #[test]
    fn debug_output_contains_variant_name() {
        let event = HmiEvent::Startup;

        assert!(format!("{:?}", event).contains("Startup"));
    }
}
