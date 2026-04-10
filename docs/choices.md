# 2026-04-11

- Packaging: keep the existing macOS menu bar panel behavior unchanged, and use a simpler regular-window fallback on Windows/Linux so those targets can build without `tauri-nspanel`.
- Packaging: use native GitHub Actions runners for `ubuntu-22.04` and `windows-latest` instead of trying to cross-bundle from macOS, because Tauri bundles are documented around native runner packaging.
- Packaging: separate "manual package artifacts" from "tagged release publish" so Windows/Ubuntu builds can be tried in Actions without creating a release tag first.
