# Architecture

ParamEx is a Cargo workspace with two crates. `paramex-core` computes;
`paramex-gui` presents. The tests keep that boundary in place.

```text
paramex-core                         paramex-gui
┌──────────────────────────┐         ┌──────────────────────────────┐
│ shared/  primitives      │         │ app/        shell, brand bar │
│ transfer/ product        │ ◄────── │ workspaces/ transfer, tlm    │
│ tlm/      product        │         │ ui_kit, theme, layout, kits  │
└──────────────────────────┘         └──────────────────────────────┘
   science, sessions, CSV            widgets, ingest effects, pixels
```

## Crates

### `paramex-core`

GUI-free. Owns parsing, scientific validation, extraction, session state, and
CSV serialization. It has no dependency on egui and never touches the file
dialog, threads, or the display.

| Module | Responsibility |
|---|---|
| `shared` | Product-agnostic primitives: numerics, `numpy_compat`, grid ingest for CSV/TSV/TXT/XLS/XLSX, file identity, and the CSV writer. Nothing here knows about transfer curves or TLM. |
| `transfer` | Transfer-curve parsing, metric extraction (V_TH, µ_sat, SS, on/off, hysteresis), output-curve fits, the results report, and `transfer::Session`. |
| `tlm` | Workbook-tree parsing, current selection and length fits, the result/sweep/status reports, and `tlm::Session`. |

Each product module has the same internal shape: `types` (domain values with
validating constructors), `parse`, the science (`metrics`/`extract` or
`methods`), `report`, `service` (load and analyze entry points), and `session`.

The crate denies `unreachable_pub` and warns on `missing_docs`, so every
public item is reachable from the crate root and documented.

### `paramex-gui`

The only runnable binary. `main.rs` is Windows glue (single-instance mutex,
log file, fatal error box); `lib.rs` exposes the application so integration
tests drive the real `ParamExApp` headlessly.

| Module | Responsibility |
|---|---|
| `app` | The `eframe::App`: brand bar, workspace switch, Technical guide modal, toast host. |
| `workspaces/transfer`, `workspaces/tlm` | One folder per product. Each wraps its core `Session`, owns widget state, runs ingest and export workers, and renders its panels. |
| `state` | Transient UI state shared by both workspaces: edit buffers, the active workspace, the easter egg. |
| `io_tasks` | Generic threading around blocking dialogs and file writes. Product messages live under the owning workspace. |
| `ui_kit`, `table_kit`, `plot_kit`, `richtext`, `format_ui` | The design system: cards, rails, segmented controls, numeric fields, quiet tables and plots, `<sub>`/`<sup>` markup, engineering notation. |
| `theme`, `layout` | Palette, semantic tokens, type scale, and the 1280×800 reference grid every panel lays out against. |

## Ownership

**Core owns the truth.** A `Session` holds the loaded files, the settings, and
the selection, and it is the only thing that changes them. Every mutation is a
method on the session (`install`, `remove_workbook`, `set_fallback_vd`,
`recompute_at_vg`, and so on) that validates its input and returns an error the
GUI can show. Sessions expose a generation counter; a display cache compares
generations to decide when to rebuild.

**The GUI owns the pixels.** Workspace state wraps the session and adds what
the session must not know: pre-formatted rows, measured column widths, the
active tab, pending edits typed but not yet committed, and load errors waiting
for dismissal. A GUI wrapper never reimplements a session rule; it delegates.

**Effects are deferred.** Panels do not mutate state while they render. They
push commands into a per-frame queue; the workspace applies the queue after
the frame, so two widgets cannot disagree about the state within one frame.
Blocking work (dialogs, parsing, file writes) runs on `io_tasks` worker
threads and reports back through messages that the workspace applies the same
way.

**Nothing is hidden.** Invalid inputs, failed files, and weak fits stay in
the tables with a status and a message. The core carries fit diagnostics
through a rejected fit so a weak fit remains inspectable, and the GUI must
render every row the core produces.

## Data flow

```text
file bytes ─► shared::grid_ingest ─► product::parse ─► product::types
                                                            │
                                                            ▼
                     Session ◄── install / commands ◄── product::service
                        │
                        ├─► projections (rows, plots, CSV bytes)
                        │
                        ▼
              workspace state ─► panels ─► egui ─► screen
                        ▲                     │
                        └── deferred commands ┘
```

Characterization plots show `|I_D|`; the signed core data and every CSV export
remain unchanged.

## Testing strategy

| Layer | Where | Guards |
|---|---|---|
| Core unit | `crates/paramex-core/src/**` (`#[cfg(test)]`) | constructor validation, local invariants |
| Core integration | `crates/paramex-core/tests/{shared,transfer,tlm}.rs` | parsing, metrics, sessions, reports |
| Numeric helpers | `tests/shared/numpy_compat/` against `tests/reference/numpy_compat/` | exact tie-breaks, rounding, NaN handling |
| TLM oracle | `tests/tlm/` against `tests/reference/tlm/` | end-to-end CSV parity with the regenerable corpus |
| Repo contract | `tests/shared/repo_contract/` | committed reference data contains no absolute paths |
| GUI behavior | `crates/paramex-gui/tests/gui_{shared,transfer,tlm}.rs` | pointer/keyboard tests for hand-painted widgets, layout stability, overflow guards |
| GUI source lints | `tests/gui/shared/source_lints.rs` | panels route widgets, text voices, colors, and copy through the kits; GPU renders take the serializing lock |
| Snapshots | `crates/paramex-gui/tests/app_snapshot.rs` and `tests/snapshots/*.png` | pixel-exact empty and loaded states of every scene |

Test-data vocabulary: a *fixture* is an ordinary raw input, a *reference* is
committed expected data, an *oracle* is expected data regenerated by an in-tree
Rust generator (`cargo run -p paramex-core --example gen_tlm_corpus`). All
committed test data is synthetic.

Headless GPU renders share one process-wide wgpu device on Windows, so every
render in the GUI test binaries goes through `tests/common/mod.rs`, which holds
a mutex for the duration of the render. The source lint enforces this.

## Conventions

- Product vocabulary stays in the product. `shared` may not mention transfer
  curves or TLM.
- American spelling in identifiers, comments, and documentation.
- Comments state present-tense invariants and the reason for a choice. They do
  not cite files outside the repository, tickets, dates, or conversations.
- The `Transfer` and `TLM` workspace names are user-facing and fixed.
- TLM labels the fitted intercept `intercept (2R_c)` and the derived value
  `R_c/contact`.
- The palette constants in `theme` carry the names of the reference colors they
  were taken from. Chrome and text read the semantic `tokens()`; plots and the
  controls color-coded to plot lines (forward/backward, fit bands) read the
  palette constants directly so a control and its line can never drift apart.
