# Orbital Movement Gdextension

Orbital Movement Gdextension — Rust library with Godot 4 GDExtension bindings

## Workspace

| Crate / path | Role |
|--------------|------|
| [`orbital_movement_gdextension`](./) | Core library |
| [`examples/orbital_movement_gdextension_cli`](./examples/orbital_movement_gdextension_cli/) | Native API demo — run without Godot |
| [`extensions/orbital_movement_gdextension_gd`](./extensions/orbital_movement_gdextension_gd/) | Godot 4 GDExtension |
| [`godot/`](./godot/) | Godot 4 demo project |

## Quick start

```bash
cargo build -p orbital_movement_gdextension_gd
cargo test
cargo run -p orbital_movement_gdextension_cli
```

The CLI walks through orbit propagation, visibility, star motion, transfer viability,
guided transfers, instantaneous burns, and manual thrust — the same concepts exposed in Godot
as `OrbitalSimulation`. Run individual sections with, for example:
`cargo run -p orbital_movement_gdextension_cli -- viability transfer`.

Open `godot/project.godot` in Godot 4.3+ and run the demo scene.

## API overview

Rust core (`orbital_movement_gdextension`):

```rust
use orbital_movement_gdextension::{
    OrbitParams, OrbitType, Simulation, SimulationScale,
};

let mut sim = Simulation::earth_like(86_400.0)?;
let id = sim.create_body_in_orbit(
    OrbitType::LowCircular,
    OrbitParams::low_circular(0.05),
    1.0,
)?;
sim.step(60.0)?;
let pos = sim.position(id)?;
```

Godot GDExtension class: `OrbitalSimulation` — see [`docs/godot.md`](./docs/godot.md).

## Development

```bash
./scripts/linux/ci-check.sh
./scripts/linux/setup-git-hooks.sh
```

See [`docs/ci.md`](./docs/ci.md) for GitHub Actions and GitLab CI details.

## Releasing

Bump the version, then push the commit and tag:

```bash
./scripts/linux/bump-version.sh
git push origin HEAD
git push origin v0.1.0
```

Pushing a `vX.Y.Z` tag triggers the **release pipeline** on your git host (if configured):

- **GitHub:** [`.github/workflows/release.yml`](./.github/workflows/release.yml) — builds per-platform GDExtension binaries and publishes addon/demo zips to GitHub Releases.
- **GitLab:** [`.gitlab-ci.yml`](./.gitlab-ci.yml) — same packaging, published as a GitLab Release.

Remove the CI config you do not use. Full details: [`docs/ci.md`](./docs/ci.md).

### Manual packaging

To assemble release zips locally (after building all platform binaries):

```bash
./scripts/linux/package-godot-release.sh 0.1.0 \
  path/to/liborbital_movement_gdextension_gd.so \
  path/to/orbital_movement_gdextension_gd.dll \
  path/to/liborbital_movement_gdextension_gd.dylib
```
