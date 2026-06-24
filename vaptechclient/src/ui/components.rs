use crate::hmi::command::HmiCommand;
use crate::ui::render_target::{HomeMode, RenderTarget};

const CASE_LIGHT_OFF_PIC: u16 = 2;
const CASE_LIGHT_ON_PIC: u16 = 3;

/// Vendor mapping семантических виджетов в физические компоненты TJC.
///
/// В разных страницах одна и та же кнопка может называться по-разному
/// (`b5`, `b6`, ...), поэтому renderer работает с "case light icon", а не с
/// конкретным номером компонента.
pub fn render_case_light_icon(target: RenderTarget, enabled: bool) -> Vec<HmiCommand> {
    let Some(component) = case_light_component(target) else {
        return Vec::new();
    };

    let pic = if enabled {
        CASE_LIGHT_ON_PIC
    } else {
        CASE_LIGHT_OFF_PIC
    };

    vec![
        HmiCommand::picture(component, pic),
        HmiCommand::picture_pressed(component, pic),
    ]
}

fn case_light_component(target: RenderTarget) -> Option<&'static str> {
    match target {
        RenderTarget::Home(HomeMode::Idle) => Some("b5"),
        RenderTarget::Home(HomeMode::Printing) | RenderTarget::Print => Some("b6"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_idle_case_light_uses_b5() {
        assert_eq!(
            render_case_light_icon(RenderTarget::Home(HomeMode::Idle), true),
            vec![
                HmiCommand::picture("b5", 3),
                HmiCommand::picture_pressed("b5", 3),
            ]
        );
    }

    #[test]
    fn print_case_light_uses_b6() {
        assert_eq!(
            render_case_light_icon(RenderTarget::Print, false),
            vec![
                HmiCommand::picture("b6", 2),
                HmiCommand::picture_pressed("b6", 2),
            ]
        );
    }

    #[test]
    fn unsupported_target_has_no_case_light_icon() {
        assert!(render_case_light_icon(RenderTarget::Settings, true).is_empty());
    }
}
