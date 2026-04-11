# 2026-04-11

- Investigated cross-platform packaging. Current repo only publishes macOS artifacts and relies on the macOS-only `tauri-nspanel` crate in runtime paths.
- Added a non-mac panel fallback design: tray click toggles the main window, centered, while macOS keeps the anchored floating panel.
- Extended the release workflow matrix toward Ubuntu and Windows packaging, with Ubuntu system packages aligned to current Tauri docs.
- Added a separate `package-desktop.yml` workflow for manual Windows/Ubuntu packaging attempts that upload artifacts without cutting a GitHub release.
- First `Package Desktop` run failed because `bun run tauri build -- --bundles ...` forwarded `--bundles` to Cargo instead of Tauri; changed it to `bun run tauri build --bundles ...`.
