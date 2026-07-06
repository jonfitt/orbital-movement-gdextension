# Transfer validation (non-CI)

Guided transfer scenarios are validated outside the default CI test suite because they are
slow, matrix-style, and intended for manual regression runs before releases or after transfer
logic changes.

## Quick run

```bash
# Human-readable report
cargo run --example transfer_validation

# Same scenarios as ignored integration tests
cargo test transfer_validation -- --ignored --nocapture
```

## What each scenario does

1. Create an Earth-like simulation and spawn a body in a **source orbit**.
2. **Settle** for `settle_steps × dt_s` so the body is not at epoch-only conditions.
3. Record **initial corrective Δv** to the target orbit (`required_delta_v_to_orbit` at current position).
4. Start **`begin_transfer_to_orbit`** with configured `max_thrust` and `mass`.
5. Step until status is **`Finished`** or a step limit is hit.

## Checks performed

| Check | Purpose |
|-------|---------|
| Reaches `Finished` | Transfer completes |
| `burn_time ≥ initial_Δv / (max_thrust/mass)` | Not faster than thrust physics allows |
| `burn_time ≤ max_time_factor × minimum` | Not unreasonably slow / stuck |
| Max step position change while `Burning` | No mid-burn teleports |
| Position change on `Burning → Finished` step | **Catches the 100% snap skip** |
| Final inclination vs target | Orbit plane matches (circular types) |
| Final radius vs target | Altitude matches (circular types) |

The **completion jump** check is the one that catches “progress hits 100% then the satellite skips”.
Mid-burn jumps are bounded by `(max_thrust/mass) × dt` plus orbital motion; a large finish jump
means snap or guidance still discontinuous.

## Theoretical minimum burn time

Thrust-limited minimum time (single continuous burn lower bound):

```text
t_min = |Δv_initial| / (max_thrust / mass)
```

`Δv_initial` is measured once at transfer start. Hohmann-style altitude changes need
multiple orbit revolutions; those scenarios set `max_time_factor = ∞` so only the
**minimum** time and **final orbit** checks apply.

## Standard scenario catalog

Defined in `src/transfer_validation.rs` → `standard_transfer_scenarios()` (45 scenarios).

| Category | Scenarios |
|----------|-----------|
| **Plane change (same altitude)** | equatorial ↔ inclined (ISS, moderate), equatorial ↔ polar, polar ↔ inclined, moderate ↔ ISS |
| **Altitude only** | equatorial LEO ↔ mid, inclined LEO ↔ mid (same inclination) |
| **GEO class** | LEO ↔ GEO, inclined LEO ↔ GEO, GEO ↔ graveyard, LEO ↔ tundra |
| **Combined alt + plane** | equatorial/inclined/polar mixes across LEO and mid altitudes |
| **Retrograde** | equatorial prograde ↔ retrograde at LEO |
| **Elliptical / Molniya** | LEO ↔ elliptical equatorial/inclined, LEO ↔ Molniya (equatorial and inclined), Molniya → LEO |
| **Ecliptic** | LEO ↔ ecliptic prograde/retrograde, prograde ↔ retrograde flip, inclined LEO → ecliptic prograde (with obliquity) |

Override patterns:

| Kind | `max_time_factor` | `max_completion_jump_r` |
|------|-------------------|-------------------------|
| Plane change | 4× | 0.005 R⊕ |
| Altitude / Hohmann | ∞ | 0.02 R⊕ |
| GEO / graveyard / tundra | ∞ | 0.05 R⊕ |
| Combined | ∞ | 0.02 R⊕ |

Add scenarios by extending helpers in that module.

## Intermediate-point checks (future)

The harness records per-step jumps today. To add explicit mid-burn checkpoints:

1. Sample at 25 / 50 / 75% progress (`get_transfer_burn_progress`).
2. Assert inclination monotonic for pure plane-change cases.
3. Assert radius between start and target for Hohmann raises.
4. Store `(time, position, velocity)` traces for offline plotting.

## CI vs non-CI

- **CI** (`cargo test`): fast unit tests only.
- **Non-CI** (this doc): full matrix before release or after transfer changes.

Optionally wire `cargo test transfer_validation -- --ignored` into a manual GitLab job or
nightly pipeline later.

## Related constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `TRANSFER_SNAP_EPSILON` | 1e-6 | Finish when corrective Δv below this |
| `TRANSFER_SNAP_POSITION_EPSILON` | 1e-4 R⊕ | Only adjust position on finish if closer than this |

Finish uses **velocity snap always**; position is adjusted only when already within
`TRANSFER_SNAP_POSITION_EPSILON` of the target orbit point (avoids large 100% jumps).
