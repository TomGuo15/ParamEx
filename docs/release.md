# Release

ParamEx ships as one unsigned, portable Windows x64 executable attached to a
GitHub release. There is no installer and no CI publisher; the release is cut
from a clean local checkout by the steps below.

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
   size-optimized binary, enforces the 15 MiB size gate, and writes
   `target/dist/ParamEx/ParamEx.exe`. The second proves that a failed build
   leaves the previous distribution untouched.
5. Smoke-test `target/dist/ParamEx/ParamEx.exe`: it starts, both workspaces
   load a sample folder, an export writes, and the **?** guide opens.
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
   Copy-Item target\dist\ParamEx\ParamEx.exe ParamEx-vX.Y.Z-windows-x64.exe
   gh release create vX.Y.Z ParamEx-vX.Y.Z-windows-x64.exe `
       --title "ParamEx X.Y.Z" --notes-file <release-notes.md>
   Remove-Item ParamEx-vX.Y.Z-windows-x64.exe
   ```

   The release notes are the changelog section for this version.

## Size gate

`build-local-dist.ps1` fails when `paramex-gui.exe` exceeds 15 MiB. The
release profile already uses `opt-level = "z"`, fat LTO, one codegen unit, and
symbol stripping. If the gate trips, look first at new dependencies and at
assets embedded with `include_bytes!`.

## Unsigned executable

The binary is not code-signed. Windows SmartScreen may warn on first launch;
the release notes should say so and point to the SHA-256 digest GitHub shows
on the release asset.
