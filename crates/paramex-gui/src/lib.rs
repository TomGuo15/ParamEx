//! `paramex-gui` as a library. The runnable binary is `main.rs`; this crate root
//! exposes the app, its state, and the shared UI kits so the integration tests
//! under `tests/` can construct and drive the real `ParamExApp` headlessly
//! instead of a reconstruction of it.
pub mod app;
pub mod format_ui;
pub(crate) mod io_tasks;
pub mod layout;
pub mod platform;
pub mod plot_kit;
pub mod richtext;
pub mod state;
pub mod table_kit;
pub mod theme;
pub mod ui_kit;
pub mod workspaces;
