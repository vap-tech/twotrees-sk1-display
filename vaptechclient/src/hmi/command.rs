use std::fmt;

pub const TERMINATOR: [u8; 3] = [0xFF, 0xFF, 0xFF];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmiCommand {
    Page(u16),
    Text {
        component: String,
        value: String,
    },
    Value {
        component: String,
        value: i32,
    },
    Picture {
        component: String,
        pic: u16,
    },
    PicturePressed {
        component: String,
        pic: u16,
    },
    Visible {
        component: String,
        visible: bool,
    },
    Raw(String),
}

impl HmiCommand {
    pub fn page(page: u16) -> Self {
        Self::Page(page)
    }

    pub fn text(component: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Text {
            component: component.into(),
            value: value.into(),
        }
    }

    pub fn value(component: impl Into<String>, value: i32) -> Self {
        Self::Value {
            component: component.into(),
            value,
        }
    }

    pub fn picture(component: impl Into<String>, pic: u16) -> Self {
        Self::Picture {
            component: component.into(),
            pic,
        }
    }

    pub fn picture_pressed(component: impl Into<String>, pic: u16) -> Self {
        Self::PicturePressed {
            component: component.into(),
            pic,
        }
    }

    pub fn visible(component: impl Into<String>, visible: bool) -> Self {
        Self::Visible {
            component: component.into(),
            visible,
        }
    }

    pub fn raw(command: impl Into<String>) -> Self {
        Self::Raw(command.into())
    }

    pub fn to_ascii(&self) -> String {
        match self {
            Self::Page(page) => {
                format!("page {}", page)
            }

            Self::Text { component, value } => {
                format!(r#"{}.txt="{}""#, component, escape_text(value))
            }

            Self::Value { component, value } => {
                format!("{}.val={}", component, value)
            }

            Self::Picture { component, pic } => {
                format!("{}.picc={}", component, pic)
            }

            Self::PicturePressed { component, pic } => {
                format!("{}.picc2={}", component, pic)
            }

            Self::Visible { component, visible } => {
                let value = if *visible { 1 } else { 0 };
                format!("vis {},{}", component, value)
            }

            Self::Raw(command) => {
                command.clone()
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.to_ascii().into_bytes();
        bytes.extend_from_slice(&TERMINATOR);
        bytes
    }
}

impl fmt::Display for HmiCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_ascii())
    }
}

fn escape_text(text: &str) -> String {
    text.replace('\\', r"\\").replace('"', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_command() {
        let cmd = HmiCommand::page(33);

        assert_eq!(cmd.to_ascii(), "page 33");
        assert_eq!(cmd.to_bytes(), b"page 33\xFF\xFF\xFF");
    }

    #[test]
    fn text_command() {
        let cmd = HmiCommand::text("t0", "Hello");

        assert_eq!(cmd.to_ascii(), r#"t0.txt="Hello""#);
        assert_eq!(cmd.to_bytes(), b"t0.txt=\"Hello\"\xFF\xFF\xFF");
    }

    #[test]
    fn value_command() {
        let cmd = HmiCommand::value("n0", 123);

        assert_eq!(cmd.to_ascii(), "n0.val=123");
        assert_eq!(cmd.to_bytes(), b"n0.val=123\xFF\xFF\xFF");
    }

    #[test]
    fn picture_command() {
        let cmd = HmiCommand::picture("q1", 79);

        assert_eq!(cmd.to_ascii(), "q1.picc=79");
        assert_eq!(cmd.to_bytes(), b"q1.picc=79\xFF\xFF\xFF");
    }

    #[test]
    fn picture_pressed_command() {
        let cmd = HmiCommand::picture_pressed("b6", 28);

        assert_eq!(cmd.to_ascii(), "b6.picc2=28");
        assert_eq!(cmd.to_bytes(), b"b6.picc2=28\xFF\xFF\xFF");
    }

    #[test]
    fn visible_command() {
        let cmd = HmiCommand::visible("t4", true);

        assert_eq!(cmd.to_ascii(), "vis t4,1");
        assert_eq!(cmd.to_bytes(), b"vis t4,1\xFF\xFF\xFF");
    }

    #[test]
    fn raw_command() {
        let cmd = HmiCommand::raw("heat_complete=1");

        assert_eq!(cmd.to_ascii(), "heat_complete=1");
        assert_eq!(cmd.to_bytes(), b"heat_complete=1\xFF\xFF\xFF");
    }

    #[test]
    fn display_trait() {
        let cmd = HmiCommand::page(0);

        assert_eq!(format!("{}", cmd), "page 0");
    }

    #[test]
    fn debug_trait() {
        let cmd = HmiCommand::page(0);

        assert!(format!("{:?}", cmd).contains("Page"));
    }

    #[test]
    fn clone_and_eq_traits() {
        let a = HmiCommand::page(0);
        let b = a.clone();

        assert_eq!(a, b);
    }
}
