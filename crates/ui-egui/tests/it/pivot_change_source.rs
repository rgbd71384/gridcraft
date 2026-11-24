//! PivotTable Analyze › Change Data Source points the report at another range.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_engine::core::{CellRef, Value};
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

fn session() -> Session {
    let mut session = Session::new();
    session.new_workbook();
    session
        .execute(
            "range.setValues",
            json!({"range": "A1", "values": [["Region", "Sales"], ["East", 100], ["West", 80], ["East", 30]]}),
        )
        .unwrap();
    session
        .execute(
            "insert.pivotTable",
            json!({"source": "Sheet1!A1:B4", "destination": "Sheet1!D3", "rows": ["Region"], "values": ["Sales"]}),
        )
        .unwrap();
    // The anchor is inside the report, so the contextual tab is available.
    session.execute("selection.set", json!({"cell": "D3"})).unwrap();
    session
}

fn harness(session: Session) -> egui_kittest::Harness<'static, SheetApp> {
    crate::support::harness(session, 4)
}

fn key(h: &mut egui_kittest::Harness<'static, SheetApp>, k: Key, m: Modifiers) {
    h.input_mut().events.push(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    h.input_mut().events.push(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
    h.run_steps(2);
}

fn report_total(h: &egui_kittest::Harness<'_, SheetApp>) -> f64 {
    let doc = h.state().session.active().unwrap();
    let sheet = doc.wb.active().unwrap();
    let end = sheet.pivots.last().unwrap().last_range.unwrap().end;
    match sheet.value(end) {
        Value::Number(n) => n,
        other => panic!("the report's corner should be its grand total, got {other:?}"),
    }
}

fn source_of(h: &egui_kittest::Harness<'_, SheetApp>) -> String {
    h.state().session.active().unwrap().wb.active().unwrap().pivots.last().unwrap().source.clone()
}

#[test]
fn the_analyze_tab_follows_the_report() {
    let mut h = harness(session());
    h.state_mut().ui.ribbon_tab = "PivotTable Analyze".into();
    h.run_steps(2);
    assert_eq!(h.state().ui.ribbon_tab, "PivotTable Analyze");
    assert!(h.query_by_role_and_label(Role::Button, "Change Data Source").is_some());
    h.state_mut().session.execute("selection.set", json!({"cell": "A1"})).unwrap();
    h.run_steps(2);
    assert_eq!(h.state().ui.ribbon_tab, "Home", "leaving the report drops the contextual tab");
    assert!(h.query_by_role_and_label(Role::Button, "Change Data Source").is_none());
}

#[test]
fn a_saved_analyze_tab_without_a_pivot_opens_on_home() {
    let mut session = Session::new();
    session.new_workbook();
    let mut app = crate::common::app(session);
    app.ui.ribbon_tab = "PivotTable Analyze".into();
    let h = crate::support::app_harness(app, 4);
    assert_eq!(h.state().ui.ribbon_tab, "Home");
}

#[test]
fn change_data_source_applies_the_range_from_the_dialog() {
    let mut h = harness(session());
    // East 100, West 80 and East 30.
    assert_eq!(report_total(&h), 210.0);
    let old = source_of(&h);
    h.state_mut().ui.ribbon_tab = "PivotTable Analyze".into();
    h.run_steps(2);
    h.get_by_role_and_label(Role::Button, "Change Data Source").click();
    h.run_steps(3);
    h.check_accessibility();
    let dialog = h.state().dialog.as_ref().expect("the button opens the dialog");
    assert_eq!(dialog.title, "Change PivotTable Data Source");
    assert_eq!(dialog.values["source"], json!(old));
    // Drop the last source row, the one that holds 30.
    h.state_mut().dialog.as_mut().unwrap().values.insert("source".into(), json!("Sheet1!$A$1:$B$3"));
    h.run_steps(1);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(h.state().dialog.is_none(), "OK closes the dialog: {:?}", h.state().message);
    let source = source_of(&h);
    assert!(source.contains("$B$3"), "{source}");
    assert_eq!(report_total(&h), 180.0, "removing the row that holds 30 leaves 180");
    // The cell the dialog was opened on is still the anchor, so the tab stays.
    assert_eq!(h.state().session.active().unwrap().selection.active, CellRef::parse("D3").unwrap());
}
