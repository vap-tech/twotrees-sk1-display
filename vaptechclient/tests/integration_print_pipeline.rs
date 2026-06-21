use vaptechclient::app::reducers::moonraker::reduce_moonraker_event;
use vaptechclient::app::state::{AppState, Page};
use vaptechclient::hmi::command::HmiCommand;
use vaptechclient::moonraker::event::MoonrakerEvent;
use vaptechclient::moonraker::parser::parse_moonraker_message;
use vaptechclient::ui::render_diff::render_diff;

fn apply_events(state: &mut AppState, events: Vec<MoonrakerEvent>) {
    for event in events {
        reduce_moonraker_event(state, event);
    }
}

#[test]
fn printing_update_produces_expected_hmi_commands() {
    let old_state = AppState::default();
    let raw = include_str!("../fixtures/moonraker/printing_update.json");

    let events = parse_moonraker_message(raw).unwrap();

    let mut new_state = old_state.clone();
    new_state.set_page(Page::Printing);

    apply_events(&mut new_state, events);

    let commands = render_diff(&old_state, &new_state);

    assert!(commands.contains(&HmiCommand::page(Page::Printing.id())));

    assert!(commands.contains(&HmiCommand::value("n6", 32)));

    assert!(commands.contains(&HmiCommand::value("n0", 210)));
    assert!(commands.contains(&HmiCommand::value("n1", 50)));

    assert!(commands.contains(&HmiCommand::text("t8", "210")));
    assert!(commands.contains(&HmiCommand::text("t9", "50")));

    assert!(commands.contains(&HmiCommand::value("n4", 2)));
    assert!(commands.contains(&HmiCommand::value("n5", 45)));

    assert!(commands.contains(&HmiCommand::value("n7", 5)));
    assert!(commands.contains(&HmiCommand::value("n8", 52)));
}
