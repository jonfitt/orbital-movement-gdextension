# Orbital Movement Gdextension — project guide for Python developers

<!-- TODO: Expand this guide as the project grows. -->

| Python | Rust (this project) |
|--------|---------------------|
| `pyproject.toml` | `Cargo.toml` |
| `import mypackage` | `use orbital_movement_gdextension::...` |
| `pytest` | `cargo test` |
| Virtualenv | `target/` build directory |

## Repository layout

```text
orbital-movement-gdextension/
├── VERSION                         # single source of truth for release version
├── Cargo.toml                      # workspace + orbital_movement_gdextension library package
├── src/                            # orbital_movement_gdextension library
├── extensions/orbital_movement_gdextension_gd/
├── examples/
├── godot/
└── docs/                           # architecture, Godot API, CI, project guide
```

## Cargo.toml files

### Root `Cargo.toml` (workspace + library)

```toml
[workspace.package]
version = "0.1.0"

license-file = "LICENSE"


[package]
name = "orbital_movement_gdextension"
version.workspace = true
edition = "2024"
description = "..."

license-file.workspace = true

```

Release version lives in root **`VERSION`** (not duplicated by hand in each crate). Root
`[workspace.package] version` and this document are synced from it via
`scripts/linux/sync-version.sh` or `scripts/windows/sync-version.cmd`.
Workspace members set `version.workspace = true`.

## Common commands

```bash
cargo build
cargo test
cargo run -p orbital_movement_gdextension_cli
cargo build -p orbital_movement_gdextension_gd
```
