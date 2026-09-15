//! TlmTab state: default tab, index round-trip, and the install/clear tab resets.
//! (File statuses are no longer a tab — they live on the always-visible right-column
//! FILES card — so there is no error auto-switch to test anymore.)

use crate::common;
use paramex_core::tlm::AnalyzedDataset;
use paramex_core::tlm::TlmDataset;
use paramex_gui::workspaces::tlm::state::{TlmState, TlmTab};

fn corpus_dataset() -> TlmDataset {
    common::load_tlm_corpus()
}

fn corpus_analyzed() -> AnalyzedDataset {
    AnalyzedDataset::analyze(corpus_dataset())
}

#[test]
fn results_tab_defaults_to_results() {
    assert_eq!(TlmState::default().results_tab(), TlmTab::Results);
}

#[test]
fn tab_index_round_trips() {
    for (i, tab) in [TlmTab::Results, TlmTab::Sweep, TlmTab::Lengths]
        .into_iter()
        .enumerate()
    {
        assert_eq!(tab.index(), i);
        assert_eq!(TlmTab::from_index(i), tab);
    }
    assert_eq!(TlmTab::from_index(99), TlmTab::Results);
}

#[test]
fn any_load_lands_on_results_even_from_a_stale_tab() {
    // The corpus contains an error workbook; a fresh load must STILL land on
    // Results (failures are surfaced by the FILES card, not a tab switch).
    // Mutate-after-default (not struct-update): TlmState carries a private
    // rows_generation field, so `..TlmState::default()` no longer compiles here.
    let mut tlm = TlmState::default();
    tlm.set_results_tab(TlmTab::Lengths);
    tlm.install_analyzed(corpus_analyzed());
    assert_eq!(tlm.results_tab(), TlmTab::Results);
}

#[test]
fn clear_resets_the_tab() {
    let mut tlm = TlmState::default();
    tlm.set_results_tab(TlmTab::Lengths);
    tlm.clear();
    assert_eq!(tlm.results_tab(), TlmTab::Results);
}
