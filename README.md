# Image Suite

Image Suite is a multi-feature desktop application for working with images. It includes tools such as batch renaming, basic editing, format conversion, and more. This repository contains a Tauri-based desktop frontend and Rust backend.

## Features (planned)
- Batch image renamer
- Image editor (crop, resize)
- Format converter (PNG/JPG/WebP)
- Plugins/extension points for more image utilities

## Quick start (dev)
Requirements: Rust toolchain, Node.js (for frontend), Tauri prerequisites.

```powershell
# from repo root
cd image-suite
# build and run (example)
cargo tauri dev
```

## Contributing
Please open issues for features/bugs and create a branch per feature. See `CONTRIBUTING.md` for guidelines (coming soon).

## Repository layout

This repository is organized as a multi-crate workspace and Tauri app. High-level layout:

- `src-tauri/` — Tauri backend and configuration (existing)
- `crates/renamer/` — core renaming logic (library)
- `crates/editor/` — image editing utilities (library)
- `crates/converter/` — image format conversion utilities (library)

Each crate contains a minimal `Cargo.toml` and `src/` with library entrypoints. The `README`s inside each folder describe planned features.

## Next steps for contributors

- Follow `CONTRIBUTING.md` to create feature branches and PRs.
- Run tests with `cargo test` and use the provided CI workflow.

