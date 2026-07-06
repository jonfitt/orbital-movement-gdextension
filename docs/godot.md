# Godot GDExtension API

## Classes

| Class | Description |
|-------|-------------|
| `OrbitalSimulation` | Main simulation API (`RefCounted`) |

## Enums

Orbit types and body states are passed as **`i64` constants** on `OrbitalSimulation`:

| Constant | Value | Description |
|----------|-------|-------------|
| `ORBIT_CIRCULAR_EQUATORIAL` | 0 | Prograde circular equatorial |
| `ORBIT_CIRCULAR_POLAR` | 1 | Circular polar |
| `ORBIT_GEOSTATIONARY` | 2 | Geostationary |
| `ORBIT_LOW_CIRCULAR` | 3 | Low circular |
| `ORBIT_RETROGRADE_EQUATORIAL` | 4 | Retrograde equatorial |
| `ORBIT_ELLIPTICAL_EQUATORIAL` | 5 | Elliptical equatorial |
| `ORBIT_ECLIPTIC_PROGRADE` | 6 | Prograde in the ecliptic (planet–sun) plane |
| `ORBIT_ECLIPTIC_RETROGRADE` | 7 | Retrograde in the ecliptic plane |
| `ORBIT_ELLIPTICAL_INCLINED` | 8 | Elliptical with configurable inclination |
| `ORBIT_TUNDRA` | 9 | Inclined circular at GEO altitude |
| `ORBIT_MOLNIYA` | 10 | Highly elliptical inclined (12 h class) |
| `ORBIT_GRAVEYARD` | 11 | Supersynchronous circular above GEO |
| `TRANSFER_IDLE` | 0 | No transfer burn active |
| `TRANSFER_BURNING` | 1 | Gradual transfer burn in progress |
| `TRANSFER_FINISHED` | 2 | Transfer burn completed (awaiting `clear_transfer_burn`) |
| `TRANSFER_VIABILITY_AVAILABLE` | 0 | Guided transfer is supported and expected to complete in reasonable time |
| `TRANSFER_VIABILITY_IMPRACTICAL` | 1 | Physically possible but too slow for gameplay (gray out in UI) |
| `TRANSFER_VIABILITY_UNAVAILABLE` | 2 | Cannot start (escape trajectory, no thrust, invalid target, etc.) |
| `STATE_FLYING` | 0 | Free flight |
| `STATE_SURFACE_CONTACT` | 1 | On surface |
| `THRUST_PROGRADE` | 1 | Local prograde bit flag |
| `THRUST_RETROGRADE` | 2 | Local retrograde bit flag |
| `THRUST_LEFT` | 4 | Left of travel bit flag |
| `THRUST_RIGHT` | 8 | Right of travel bit flag |
| `THRUST_UP` | 16 | Radial outward bit flag |
| `THRUST_DOWN` | 32 | Radial inward bit flag |

Static accessors (no instance): `default_low_earth_inclination_rad()`, `default_high_inclination_rad()`,
`molniya_perigee_altitude()`, `molniya_apogee_altitude()`, `orbit_uses_*`, `get_orbit_ui_defaults()`.

## OrbitalSimulation methods

| Method | Returns | Description |
|--------|---------|-------------|
| `create_earth_like(rotation_period_s)` | `bool` | Initialize Earth-like simulation |
| `create_earth_like_with_obliquity(rotation_period_s, obliquity_rad)` | `bool` | Earth-like planet (+Y spin) with sun on a tilted ecliptic plane |
| `create_custom(mass_earth, radius_earth, rotation_period_s, length_scale, time_scale)` | `bool` | Custom central body and scale |
| `set_star(distance_earth_radii, obliquity_rad, orbital_longitude_rad)` | `void` | Sun on the ecliptic plane (tilted from XZ by obliquity about +X) |
| `step(delta)` | `bool` | Advance simulation by `delta` seconds |
| `create_body(position, velocity, mass)` | `int` | Create body; returns id or `-1` |
| `create_body_circular(orbit_type, altitude_earth_radii, mass)` | `int` | Create body; `orbit_type` is an `ORBIT_*` constant |
| `create_body_elliptical(perigee_altitude, apogee_altitude, mass)` | `int` | Create body in elliptical orbit |
| `get_position(body_id)` | `Vector3` | Current position |
| `get_velocity(body_id)` | `Vector3` | Current velocity |
| `get_state(body_id)` | `int` | `STATE_FLYING` or `STATE_SURFACE_CONTACT` |
| `clear_surface_contact(body_id)` | `bool` | Clear surface contact state |
| `apply_force(body_id, direction, magnitude)` | `bool` | Apply thrust for next step (cancels active transfer) |
| `apply_delta_v(body_id, delta_v)` | `bool` | Instantaneous burn (cancels active transfer) |
| `set_max_thrust(body_id, force)` | `bool` | Set maximum thrust force for transfers and manual thrust clamping |
| `get_max_thrust(body_id)` | `float` | Maximum thrust force configured for the body |
| `get_delta_v_to_orbit(body_id, orbit_type, altitude_earth_radii)` | `Vector3` | Required delta-v to target orbit |
| `get_thrust_direction_to_orbit(body_id, orbit_type, altitude_earth_radii)` | `Vector3` | Unit burn direction |
| `get_visible_surface_area(body_id)` | `float` | Visible planetary cap area |
| `get_horizon_half_angle(body_id)` | `float` | Horizon half-angle in radians |
| `get_planet_radius()` | `float` | Planet surface radius in sim units |
| `get_geostationary_altitude()` | `float` | GEO altitude above surface (Earth radii); derived from μ and rotation period |
| `get_graveyard_altitude()` | `float` | Graveyard orbit altitude above surface (Earth radii) |
| `get_spin_axis()` | `Vector3` | Normalized planet spin axis (always +Y for Earth-like bodies) |
| `get_rotation_period_s()` | `float` | Sidereal rotation period in simulation seconds |
| `get_angular_rate_rad_s()` | `float` | Planet spin rate (rad/s); use for integrating spin angle |
| `get_position_planet_fixed(body_id, spin_angle_rad)` | `Vector3` | Body position in planet-fixed frame |
| `get_star_apparent_position(spin_angle_rad)` | `Vector3` | Star position in co-rotating frame |
| `get_star_inertial_position()` | `Vector3` | Fixed inertial star position |
| `get_time()` | `float` | Simulation time in seconds |
| `reset_simulation()` | `bool` | Clear bodies and reset time |
| `spawn_body_in_orbit(orbit_type, altitude, perigee, apogee, inclination_rad, mass)` | `int` | Create body in selected orbit |
| `apply_transfer_to_orbit(body_id, orbit_type, altitude, perigee, apogee, inclination_rad)` | `bool` | Instantaneous transfer delta-v burn |
| `begin_transfer_to_orbit(body_id, orbit_type, altitude, perigee, apogee, inclination_rad)` | `bool` | Start guided transfer (requires `max_thrust` > 0) |
| `begin_transfer_burn(...)` | `bool` | Alias for `begin_transfer_to_orbit` |
| `get_transfer_burn_status(body_id)` | `int` | `TRANSFER_IDLE`, `TRANSFER_BURNING`, or `TRANSFER_FINISHED` |
| `get_transfer_burn_progress(body_id)` | `float` | Burn completion fraction 0.0–1.0 |
| `get_transfer_burn_remaining(body_id)` | `float` | Remaining corrective delta-v magnitude |
| `clear_transfer_burn(body_id)` | `bool` | Clear finished or idle transfer state |
| `assess_transfer_viability(body_id, orbit_type, altitude, perigee, apogee, inclination_rad, max_practical_burn_time_s)` | `Dictionary` | Classify guided transfer as `TRANSFER_VIABILITY_*` |
| `orbit_uses_elliptical_params(orbit_type)` | `bool` | Whether UI should show perigee/apogee fields |
| `orbit_uses_inclination_param(orbit_type)` | `bool` | Whether UI should show inclination |
| `orbit_uses_computed_altitude(orbit_type)` | `bool` | Whether altitude is derived from planet physics |
| `get_orbit_ui_defaults(orbit_type)` | `Dictionary` | Default altitude/perigee/apogee/inclination for an orbit type |
| `get_thrust_direction_from_flags(body_id, direction_flags)` | `Vector3` | Unit vector from local thrust flags |
| `apply_force_from_flags(body_id, magnitude, direction_flags)` | `bool` | Apply thrust along combined local direction (cancels active transfer) |

For `ORBIT_GEOSTATIONARY`, the `altitude` argument is **ignored**. The orbital radius is computed as
`r = (μ / ω²)^(1/3)` where `ω = 2π / rotation_period_s`. Orbits use the Y-up frame: equatorial orbits
lie in the XZ plane, polar orbits pass through the Y axis (poles). **Low circular** uses
`inclination_rad` from the UI or API (`0` = equatorial; use ~51.6° / `0.901` rad for ISS-like).
Use `OrbitParams::low_circular(altitude)` in Rust for the ISS default without passing an angle.
**Elliptical inclined** and **Molniya** use perigee/apogee and inclination. **Tundra** is circular at GEO altitude with high inclination (~63.4°).
**Graveyard** is circular at GEO + offset (altitude computed). **Ecliptic** orbits use the star's obliquity for the
orbital plane. Axial tilt is modeled by the sun's ecliptic plane, not by tilting the planet.

Use `begin_transfer_to_orbit` for strategic automated movement. Before offering a transfer in the UI,
call `assess_transfer_viability` with the body's current `max_thrust` and mass (implicit in the
simulation). The returned dictionary includes `availability` (`TRANSFER_VIABILITY_*`), estimated burn time,
plane-change angle, and a `reason` string when not available. Set `max_thrust` on the body
first (force in simulation units; acceleration is `max_thrust / mass`). Each step recomputes
guidance toward the target orbit and applies up to `(max_thrust / mass) * delta` of corrective
delta-v. When close enough, the body **snaps** to the target orbit's canonical initial state
(from `initial_state_for_orbit`) for a deterministic end result. Poll `get_transfer_burn_status`
and `get_transfer_burn_progress` each frame; call `clear_transfer_burn` after `TRANSFER_FINISHED`
to reset the indicator. Manual thrust or `apply_delta_v` **cancels** an active transfer.

**Snap phase angle:** on completion the body snaps to the target orbit at its **current**
true anomaly (preserved from position), not the template default. Set `true_anomaly_rad` in
`OrbitParams` only for `spawn_body_in_orbit` / initial placement. A future option may expose
explicit phase choice for transfers.

**Thrust vs transfer:** `max_thrust` is a force cap, not a burn-rate constant. Mass is constant
(no propellant mass simulation). When `max_thrust` > 0, manual `apply_force` magnitudes are
clamped to `max_thrust`.

## Example (GDScript)

```gdscript
var sim := OrbitalSimulation.new()
sim.create_earth_like(86400.0)
sim.set_star(100.0, 0.41, PI * 0.5)
var body_id := sim.create_body_circular(OrbitalSimulation.ORBIT_LOW_CIRCULAR, 0.05, 1.0)
sim.set_max_thrust(body_id, 0.002)  # Mk1 thruster force; a = F / mass

func _physics_process(delta: float) -> void:
    sim.step(delta)
    $Satellite.position = sim.get_position(body_id)
    $Sun.look_at(Vector3.ZERO, Vector3.UP)

func move_to_geo() -> void:
    var v := sim.assess_transfer_viability(
        body_id,
        OrbitalSimulation.ORBIT_GEOSTATIONARY,
        0.0, 0.0, 0.0, 0.0,
        0.0,  # use default max practical burn time (5 sim-days)
    )
    if v["availability"] != OrbitalSimulation.TRANSFER_VIABILITY_AVAILABLE:
        return  # gray out or show v["reason"]
    sim.begin_transfer_to_orbit(
        body_id,
        OrbitalSimulation.ORBIT_GEOSTATIONARY,
        0.0, 0.0, 0.0, 0.0,
    )
```

## Building for Godot

```bash
cargo build -p orbital_movement_gdextension_gd
```

Open `godot/project.godot` in Godot 4.3 or later.

## Releasing

```bash
./scripts/linux/bump-version.sh
```

```bash
git push origin HEAD
git push origin vX.Y.Z
```

See [`docs/ci.md`](ci.md) for CI details.
