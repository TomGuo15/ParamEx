//! `ParamExApp` — the `eframe::App`. Owns committed state (`core::Session`) plus
//! transient sibling structs. Page modules render from it each frame.

mod brand_bar;
mod help_guide;
mod ingest;
mod page_dispatch;
mod shell;

use eframe::egui;
use egui_notify::{Anchor, Toasts};
use paramex_core::transfer::Session;

use crate::state::{EasterEgg, EditBuffers, Workspace};
use crate::workspaces::tlm::state::TlmState;
use crate::workspaces::tlm::TlmWorkspace;
use crate::workspaces::transfer::TransferWorkspace;

pub struct ParamExApp {
    pub(crate) transfer: TransferWorkspace,
    pub(crate) tlm: TlmWorkspace,
    pub(crate) edits: EditBuffers,
    pub(crate) egg: EasterEgg,
    pub(crate) toasts: Toasts,
    pub(crate) active_workspace: Workspace,
    pub(crate) show_help: bool,
    pub(crate) help_workspace: Workspace,
}

impl ParamExApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::theme::install(&cc.egui_ctx);
        Self::from_session(Session::new())
    }
}

impl ParamExApp {
    /// Build an app around a pre-loaded `Session` without a `CreationContext`,
    /// for headless render/snapshot tests. The caller installs the theme on the
    /// egui `Context` itself (e.g. `crate::theme::install(ui.ctx())`).
    pub fn from_session(session: Session) -> Self {
        ParamExApp {
            transfer: TransferWorkspace::from_session(session),
            tlm: TlmWorkspace::default(),
            edits: EditBuffers::default(),
            egg: EasterEgg::default(),
            toasts: Toasts::default().with_anchor(Anchor::TopRight),
            active_workspace: Workspace::default(),
            show_help: false,
            help_workspace: Workspace::default(),
        }
    }

    // The accessors below exist for the integration tests, which seed and
    // inspect the real app through the public crate surface.

    pub fn transfer_mut(&mut self) -> &mut TransferWorkspace {
        &mut self.transfer
    }

    pub fn tlm(&self) -> &TlmState {
        &self.tlm.state
    }

    pub fn tlm_mut(&mut self) -> &mut TlmState {
        &mut self.tlm.state
    }

    pub fn set_tlm_state(&mut self, tlm: TlmState) {
        self.tlm.state = tlm;
    }

    pub fn set_active_workspace(&mut self, workspace: Workspace) {
        self.active_workspace = workspace;
    }
}

impl eframe::App for ParamExApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}
