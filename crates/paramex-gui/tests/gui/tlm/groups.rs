use crate::common;
use eframe::egui;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use paramex_gui::workspaces::tlm::state::TlmState;

struct TlmGroupsHarnessApp {
    tlm: TlmState,
    size: egui::Vec2,
}

impl eframe::App for TlmGroupsHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(self.size, |ui| {
            paramex_gui::workspaces::tlm::panels::groups::show(ui, &mut self.tlm);
        });
    }
}

fn groups_harness(tlm: TlmState) -> Harness<'static, TlmGroupsHarnessApp> {
    let state = TlmGroupsHarnessApp {
        tlm,
        size: egui::Vec2::new(320.0, 420.0),
    };
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness
}

#[test]
fn weak_fit_groups_render_warning_badges() {
    let tlm = common::loaded_tlm_state();
    let groups = tlm.group_list().expect("loaded TLM groups");
    let warning_count = groups
        .groups
        .iter()
        .filter(|group| !group.warnings.is_empty())
        .count();
    let clean_count = groups.groups.len() - warning_count;
    assert!(
        warning_count > 0 && clean_count > 0,
        "fixture needs both states"
    );

    let harness = groups_harness(tlm);

    assert_eq!(harness.query_all_by_label("WARN").count(), warning_count);
    assert_eq!(harness.query_all_by_label("OK").count(), clean_count);
}

#[test]
fn clicking_a_process_group_row_updates_selection() {
    let tlm = common::loaded_tlm_state();
    let groups = tlm.group_list().expect("loaded TLM groups");
    let target = groups
        .groups
        .iter()
        .map(|group| group.group.clone())
        .find(|name| Some(name.as_str()) != groups.selected)
        .expect("at least two process groups in corpus");

    let mut harness = groups_harness(tlm);

    harness.get_by_label(&target).click();
    harness.run();

    assert_eq!(
        harness.state().tlm.selected_group_name(),
        Some(target.as_str())
    );
}

#[test]
fn process_group_rows_expose_radio_semantics_and_assistive_selection() {
    let tlm = common::loaded_tlm_state();
    let groups = tlm.group_list().expect("loaded TLM groups");
    let selected = groups.selected.expect("selected process group").to_string();
    let target = groups
        .groups
        .iter()
        .map(|group| group.group.clone())
        .find(|name| name != &selected)
        .expect("at least two process groups in corpus");
    let mut harness = groups_harness(tlm);

    let selected_label = format!("Select {selected}");
    let target_label = format!("Select {target}");
    {
        let selected_row =
            harness.get_by_role_and_label(egui::accesskit::Role::RadioButton, &selected_label);
        assert_eq!(
            selected_row.accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        let target_row =
            harness.get_by_role_and_label(egui::accesskit::Role::RadioButton, &target_label);
        assert_eq!(
            target_row.accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::False)
        );
        assert!(target_row
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click));
        target_row.click_accesskit();
    }
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().tlm.selected_group_name(),
        Some(target.as_str())
    );
}
