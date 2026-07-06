# Architecture

## Workspace overview

```text
orbital_movement_gdextension (core library)
    └── extensions/orbital_movement_gdextension_gd (Godot GDExtension cdylib)

examples/orbital_movement_gdextension_cli (native API demo — no Godot)
godot/ (Godot 4 demo project — presentation and input only)
```

The Godot demo should call into the library (via `OrbitalSimulation`) for orbit metadata,
parameter mapping, planet-fixed coordinates, transfer viability, and physics. Avoid
duplicating orbit defaults or frame transforms in GDScript.

## Core library modules

| Module | Responsibility |
|--------|----------------|
| `math` | `Vec3`, rotations, projections |
| `units` | SI constants, `SimulationScale`, gravitational parameter |
| `central_body` | Planet mass, radius, spin axis, rotation period |
| `small_body` | Body id, state, position, velocity, thrust |
| `gravity` | Point-mass gravitational acceleration |
| `integrator` | Velocity Verlet integration |
| `collision` | Surface clamp and `SurfaceContact` state |
| `orbits` | `OrbitType`, `OrbitParams`, UI metadata, initial conditions, transfer delta-v |
| `star` | Fixed inertial star and apparent co-rotating position |
| `visibility` | Horizon angle and visible spherical cap area |
| `thrust_frame` | Local prograde/normal/radial thrust directions and bit flags |
| `transfer_burn` | Guided transfer tracker, snap-to-target completion |
| `transfer_viability` | Classify transfers as available / impractical / unavailable |
| `transfer_validation` | Scenario harness for regression checks (45 scenarios) |
| `simulation` | World state and public API |

## Data flow

```text
UI fields (altitude, perigee, apogee, inclination)
    → build_orbit_params_from_ui / orbit_ui_defaults (orbits)
    → OrbitParams + OrbitType
    → Simulation::create_body_in_orbit / begin_transfer_to_orbit
    → transfer_burn (per-step guidance) + integrator + gravity
```

Transfer viability runs before starting a guided burn: `assess_transfer_viability` estimates
delta-v, plane change, and burn duration against `max_thrust` and mass.

## Coordinate frame

- **Origin**: planet center.
- **+Y**: spin axis (north pole) by default.
- **Equatorial plane**: X–Z.
- **Prograde**: right-hand rule about the spin axis.

The planet does **not** rotate in simulation coordinates. Godot rotates the planet mesh
for display and uses `body_position_planet_fixed(spin_angle)` / `star_apparent_position(spin_angle)`
so objects and the light source appear correctly in the surface-fixed frame.

## Unit system

| Quantity | Unit |
|----------|------|
| Central mass | Earth masses (`1.0` = Earth) |
| Distance | Earth radii |
| Time | seconds (with optional `time_scale`) |
| Gravitational parameter | derived from real `G` and scale multipliers |

Surface contact clamps bodies to `radius_earth` and sets `BodyState::SurfaceContact` until a new delta-v is applied.

See also [`godot.md`](godot.md) for the Godot-facing API.
