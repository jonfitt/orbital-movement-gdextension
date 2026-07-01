# Godot GDExtension API

<!-- TODO: Document classes, methods, and signals exposed from Rust. -->

## Classes

| Class | Description |
|-------|-------------|
| `OrbitalMovementGdextensionApi` | Example `RefCounted` wrapper around the core library |

## Methods

| Class | Method | Returns | Description |
|-------|--------|---------|-------------|
| `OrbitalMovementGdextensionApi` | `greet(name: String)` | `String` | Calls the core `greet` function |

## Building for Godot

```bash
cargo build -p orbital_movement_gdextension_gd
```

Open `godot/project.godot` in Godot 4.3 or later.

## Releasing

```bash
./scripts/linux/bump-version.sh
# or on Windows: scripts\windows\bump-version.cmd
```

That bumps `VERSION`, syncs `Cargo.toml` and `docs/description.md`, commits, and creates tag `vX.Y.Z`.

```bash
git push origin HEAD
git push origin vX.Y.Z
```

### If using GitHub

The tag push runs [`.github/workflows/release.yml`](../.github/workflows/release.yml), which builds
Linux/Windows/macOS GDExtension binaries and publishes addon/demo zip files to **GitHub Releases**.

### If using GitLab

The tag push runs the `release-*` jobs in [`.gitlab-ci.yml`](../.gitlab-ci.yml), which produce the
same zip layout and attach them to a **GitLab Release**.

See [`docs/ci.md`](ci.md) for runner requirements and other git hosts.
