# ParamEx

ParamEx is a Windows desktop app for characterizing thin-film transistors from
transfer and transmission-line measurements. Load the raw instrument files,
check the fits on screen, and export the results as CSV.

- **Transfer** turns gate sweeps into threshold voltage, saturation mobility,
  subthreshold swing, on/off ratio, and hysteresis, and fits attached output
  curves for output conductance and Early voltage.
- **TLM** fits total resistance against channel length and reports contact
  resistance at a selected gate voltage and across the whole sweep.

Failed files and weak fits stay visible with an explanation. Nothing is dropped
silently.

## Download

Get `ParamEx-vX.Y.Z-windows-x64.exe` from the
[latest release](https://github.com/TomGuo15/ParamEx/releases/latest) and run
it. It is a single portable executable with no installer. The binary is not
code-signed, so Windows may show a SmartScreen prompt on first launch; the
release page lists the SHA-256 digest of each asset.

## Quick start

1. Pick **Transfer** or **TLM** in the top bar.
2. **Transfer**: use **Load Transfer** for files or **Load Folder** for a
   directory. Set `W`, `L`, and `C_ox`, then read the results table or drag
   the fit window on the selected curve. **Load Output** attaches I_D–V_D
   files to their transfer file by name.
3. **TLM**: use **Load Folder** on a `root/group/<length_um>/*.xlsx` tree.
   Enter **Fallback V_D** first if the workbooks carry no `Setup(*)` sheet.
4. **Export CSV** (Transfer) or **Export TLM CSV** / **Export Sweep CSV**
   (TLM) opens a save dialog for the files listed below.

The **?** button opens the in-app Technical guide with the exact input
contract and the equations behind every number.

## Input files

| Workspace | Formats | Required columns |
|---|---|---|
| Transfer | `.csv`, `.tsv`, `.txt`, `.xls`, `.xlsx` | gate voltage (`Vg`, `Vgs`, `Gate`, `Gate Voltage`), drain current (`Id`, `Ids`, `Drain`, `Drain Current`) |
| Output curves | same as Transfer | the Transfer columns plus drain voltage (`Vd`, `Vds`, `Drain`, `Drain Voltage`) |
| TLM | `.xlsx` | a `List(*)` sheet with `vg`, `abs_id`, `abs_is`; an optional `Setup(*)` sheet with the drain bias |

Headers are case-insensitive and may carry units.

## Output files

| File | Contents |
|---|---|
| `paramex_report.csv` | Transfer results, forward and backward sections |
| `paramex_output_report.csv` | Output-curve fits per family and per gate line |
| `paramex_tlm_result.csv` | TLM fit at the selected gate voltage |
| `paramex_tlm_sweep.csv` | TLM fit at every measured gate voltage |

## Documentation

- [Technical guide](docs/technical-guide.md): input contracts, fit
  mathematics, validation rules, and CSV columns.
- [Architecture](docs/architecture.md): crate layout, ownership rules, and the
  testing strategy.
- [Contributing](CONTRIBUTING.md): toolchain, checks, snapshots, and
  conventions.
- [Release](docs/release.md): how a version is cut and published.
- [Changelog](CHANGELOG.md).

## Building from source

```powershell
cargo run -p paramex-gui
```

Requires Rust 1.87 or newer on Windows. `cargo test --workspace` runs the
full suite, including headless GUI snapshots.

## License

MIT. See [LICENSE](LICENSE).

## Acknowledgments

Development used the Cursor and OpenAI Codex coding assistants.
