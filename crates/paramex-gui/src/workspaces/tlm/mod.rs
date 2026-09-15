//! TLM workspace: the core `tlm::Session` wrapped in page state, plus the
//! grid measurement cache and the ingest queue.

mod ingest;
pub mod layout;
pub mod page;
pub mod panels;
pub mod state;

pub use page::show;

use crate::io_tasks::IoQueue;
use panels::tables::TlmGridCache;

/// TLM's complete runtime aggregate.
#[derive(Default)]
pub struct TlmWorkspace {
    pub(crate) state: state::TlmState,
    pub(crate) grid_cache: TlmGridCache,
    pub(crate) io: IoQueue<ingest::Msg>,
}

impl TlmWorkspace {
    pub fn from_state(state: state::TlmState) -> Self {
        Self {
            state,
            grid_cache: TlmGridCache::default(),
            io: IoQueue::default(),
        }
    }

    pub fn state(&self) -> &state::TlmState {
        &self.state
    }

    pub(crate) fn drain_ingest(&mut self, toasts: &mut egui_notify::Toasts) {
        ingest::drain(self, toasts);
    }

    pub(crate) fn is_idle(&self) -> bool {
        self.io.is_idle()
    }
}
