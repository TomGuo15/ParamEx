# Changelog

All notable changes to ParamEx are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The workspace
version in `Cargo.toml` and the git tag always match.

## [Unreleased]

## [1.0.0] - 2026-09-12

First public release: a portable, unsigned Windows x64 executable.

### Transfer

- Loads `.csv`, `.tsv`, `.txt`, `.xls`, and `.xlsx` gate sweeps.
- Extracts threshold voltage, saturation mobility, subthreshold swing, on/off
  ratio, and hysteresis, and fits attached output curves.

### TLM

- Loads a `root/group/<length_um>/*.xlsx` tree. The length folder is a number
  in µm (`50`, `80.5`).
- Reports `intercept (2R_c)` and `R_c/contact` at a selected gate voltage and
  across the sweep.

### Application

- In-app Technical guide. The **?** button is labeled "Technical guide".
- Failed files and weak fits stay visible.
- Single portable `ParamEx.exe`, unsigned, under 15 MiB.

[Unreleased]: https://github.com/TomGuo15/ParamEx/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/TomGuo15/ParamEx/releases/tag/v1.0.0
