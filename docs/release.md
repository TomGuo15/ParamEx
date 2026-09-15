# Release

ParamEx ships as one unsigned Windows x64 executable on a GitHub release. A
release is cut from a clean local checkout by the steps below.

## Version and tag

The workspace version in `Cargo.toml` (`[workspace.package] version`) and the
git tag `vX.Y.Z` must match. Patch releases fix behavior without changing
inputs or outputs; minor releases add capability or change a CSV column; a
major release changes the input contract.

## Checklist

Run everything from the repository root on the release commit.

1. Update `CHANGELOG.md`: move the `Unreleased` entries under a new
   `## [X.Y.Z] - YYYY-MM-DD` heading.
2. Set `version = "X.Y.Z"` in `Cargo.toml` and let `cargo build` refresh
   `Cargo.lock`.
3. Gates:

   ```powershell
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --no-fail-fast
   cargo doc --workspace --no-deps
   ```

   The test run includes the snapshot suite. A snapshot failure means a pixel
   changed; review the `.diff.png` next to the baseline and either fix the
   regression or refresh the baseline deliberately (see `CONTRIBUTING.md`).
4. Package:

   ```powershell
   .\packaging\windows\build-local-dist.ps1
   .\packaging\windows\test-build-local-dist-failure.ps1
   ```

   The first script deletes any leftover `paramex-gui.exe`, builds a new
   size-optimized binary, and writes `target/dist/ParamEx/ParamEx.exe`,
   removing any other files under `target/dist`. The second proves that a
   failed build leaves `target/dist` untouched and that a successful build
   leaves only `ParamEx/ParamEx.exe`.
5. Smoke-test `target/dist/ParamEx/ParamEx.exe`: it starts, both workspaces
   load a sample folder, an export writes, and the Technical guide opens.
6. Confirm the tree is clean: `git status` shows nothing, and no `.old.png`,
   `.new.png`, or `.diff.png` files exist under `crates/paramex-gui/tests/`.
7. Commit as `ParamEx X.Y.Z`, then tag and push:

   ```powershell
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

8. Publish the release with the packaged executable renamed to carry the
   version:

   ```powershell
   $asset = Join-Path $env:TEMP "ParamEx-vX.Y.Z-windows-x64.exe"
   Copy-Item target\dist\ParamEx\ParamEx.exe $asset
   gh release create vX.Y.Z $asset `
       --title "ParamEx X.Y.Z" --notes-file <release-notes.md>
   Remove-Item $asset
   ```

   The release notes are the changelog section for this version.

## Release binary

The release profile uses `opt-level = "z"`, fat LTO, one codegen unit, and
symbol stripping so the portable exe stays small. Prefer not adding
dependencies or `include_bytes!` assets that bloat it.

## Unsigned executable

The binary is not code-signed. Windows SmartScreen may warn on first launch;
the release notes should say so and point to the SHA-256 digest GitHub shows
on the release asset.
