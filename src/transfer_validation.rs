//! Transfer scenario runner for manual validation and non-CI integration tests.
//!
//! Run all scenarios:
//! ```text
//! cargo run --example transfer_validation
//! cargo test transfer_validation -- --ignored --nocapture
//! ```

use crate::orbits::{
    HIGH_INCLINATION_RAD, LOW_EARTH_INCLINATION_RAD, OrbitParams, OrbitType, is_prograde_velocity,
    orbital_inclination_rad, target_circular_radius, target_orbit_inclination,
};
use crate::simulation::Simulation;
use crate::transfer_burn::TransferBurnStatus;

/// One transfer scenario from a settled source orbit to a target orbit.
#[derive(Debug, Clone)]
pub struct TransferScenario {
    /// Human-readable scenario name.
    pub name: &'static str,
    /// Initial orbit type.
    pub source_type: OrbitType,
    /// Initial orbit parameters.
    pub source_params: OrbitParams,
    /// Target orbit type.
    pub target_type: OrbitType,
    /// Target orbit parameters.
    pub target_params: OrbitParams,
    /// Simulation time step in seconds.
    pub dt_s: f64,
    /// Steps to settle on the source orbit before starting the transfer.
    pub settle_steps: u32,
    /// Body mass.
    pub mass: f64,
    /// Maximum thrust force for the transfer.
    pub max_thrust: f64,
    /// Maximum allowed position jump on the completion step (Earth radii).
    pub max_completion_jump_r: f64,
    /// Maximum allowed single-step position change while burning (Earth radii).
    pub max_burn_step_jump_r: f64,
    /// Upper bound on burn duration as a multiple of the thrust-limited minimum.
    pub max_time_factor: f64,
    /// Final inclination tolerance in radians (circular targets only).
    pub inclination_tol_rad: f64,
    /// Final radius tolerance as a fraction of target radius (circular targets only).
    pub radius_tol_frac: f64,
    /// Maximum simulation steps while burning before giving up.
    pub max_burn_steps: u32,
    /// Planet obliquity for ecliptic scenarios (`0` = equatorial reference frame only).
    pub obliquity_rad: f64,
}

/// Outcome of running one [`TransferScenario`].
#[derive(Debug, Clone)]
pub struct TransferRunReport {
    /// Scenario name.
    pub name: String,
    /// Whether the transfer reached `Finished`.
    pub completed: bool,
    /// Simulation steps after the transfer started.
    pub burn_steps: u32,
    /// Simulated burn duration in seconds.
    pub burn_time_s: f64,
    /// Corrective delta-v magnitude when the transfer started.
    pub initial_delta_v: f64,
    /// Minimum burn time from `initial_delta_v / (max_thrust / mass)`.
    pub theoretical_min_time_s: f64,
    /// Largest position change on a single step while status was `Burning`.
    pub max_burn_step_jump_r: f64,
    /// Position change on the step that transitioned `Burning` → `Finished`.
    pub completion_jump_r: f64,
    /// Final orbital inclination in radians.
    pub final_inclination_rad: f64,
    /// Final orbital radius in Earth radii.
    pub final_radius_r: f64,
    /// Target inclination in radians (circular targets).
    pub target_inclination_rad: f64,
    /// Target radius in Earth radii (circular targets).
    pub target_radius_r: f64,
    /// Whether all configured checks passed.
    pub passed: bool,
    /// Human-readable failure reasons (empty if passed).
    pub failures: Vec<String>,
}

/// Runs a single transfer scenario and returns a detailed report.
pub fn run_transfer_scenario(scenario: &TransferScenario) -> TransferRunReport {
    let mut failures = Vec::new();
    let mut sim = if scenario.obliquity_rad.abs() > 1e-9 {
        Simulation::earth_like_with_obliquity(86_400.0, scenario.obliquity_rad)
    } else {
        Simulation::earth_like(86_400.0)
    }
    .expect("simulation should initialize");

    let id = sim
        .create_body_in_orbit(scenario.source_type, scenario.source_params, scenario.mass)
        .expect("source orbit should be valid");
    sim.set_max_thrust(id, scenario.max_thrust)
        .expect("max thrust should be valid");

    for _ in 0..scenario.settle_steps {
        sim.step(scenario.dt_s).expect("settle step should succeed");
    }

    let initial_delta_v = sim
        .required_delta_v_to_orbit(id, scenario.target_type, scenario.target_params)
        .expect("target orbit should be valid")
        .length();
    let accel = scenario.max_thrust / scenario.mass;
    let theoretical_min_time_s = if accel > f64::EPSILON {
        initial_delta_v / accel
    } else {
        f64::INFINITY
    };

    sim.begin_transfer_to_orbit(id, scenario.target_type, scenario.target_params)
        .expect("transfer should start");

    let mut burn_steps = 0_u32;
    let mut burn_time_s = 0.0;
    let mut max_burn_step_jump_r = 0.0_f64;
    let mut completion_jump_r = 0.0_f64;

    for _ in 0..scenario.max_burn_steps {
        let status_before = sim.transfer_burn_status(id);
        if status_before == TransferBurnStatus::Finished {
            break;
        }
        if status_before != TransferBurnStatus::Burning {
            failures.push(format!("unexpected status before step: {status_before:?}"));
            break;
        }

        let pos_before = sim.position(id).expect("position");
        sim.step(scenario.dt_s).expect("step should succeed");
        let pos_after = sim.position(id).expect("position");
        let status_after = sim.transfer_burn_status(id);

        let step_jump = (pos_after - pos_before).length();
        burn_steps += 1;
        burn_time_s += scenario.dt_s;

        if status_after == TransferBurnStatus::Burning {
            max_burn_step_jump_r = max_burn_step_jump_r.max(step_jump);
        }
        if status_before == TransferBurnStatus::Burning
            && status_after == TransferBurnStatus::Finished
        {
            completion_jump_r = step_jump;
        }
    }

    let completed = sim.transfer_burn_status(id) == TransferBurnStatus::Finished;
    let position = sim.position(id).expect("position");
    let velocity = sim.velocity(id).expect("velocity");
    let spin = sim.spin_axis();
    let final_inclination_rad = orbital_inclination_rad(position.cross(velocity), spin);
    let final_radius_r = position.length();

    let (target_inclination_rad, target_radius_r) = target_circular_elements(
        &sim,
        scenario.target_type,
        scenario.target_params,
        scenario.obliquity_rad,
    );

    if !completed {
        failures.push("transfer did not reach Finished".into());
    } else if !sim
        .orbit_matches_target(id, scenario.target_type, scenario.target_params)
        .unwrap_or(false)
    {
        failures.push("final orbit does not match target within tolerance".into());
    }
    if burn_steps > 0 && burn_time_s + f64::EPSILON < theoretical_min_time_s {
        failures.push(format!(
            "burn time {burn_time_s:.4} s is below thrust-limited minimum {theoretical_min_time_s:.4} s"
        ));
    }
    if burn_time_s > theoretical_min_time_s * scenario.max_time_factor
        && scenario.max_time_factor.is_finite()
    {
        failures.push(format!(
            "burn time {burn_time_s:.4} s exceeds {}× theoretical minimum",
            scenario.max_time_factor
        ));
    }
    if max_burn_step_jump_r > scenario.max_burn_step_jump_r {
        failures.push(format!(
            "max burn-step jump {max_burn_step_jump_r:.6} R⊕ exceeds limit {:.6}",
            scenario.max_burn_step_jump_r
        ));
    }
    if completion_jump_r > scenario.max_completion_jump_r {
        failures.push(format!(
            "completion jump {completion_jump_r:.6} R⊕ exceeds limit {:.6}",
            scenario.max_completion_jump_r
        ));
    }
    if target_radius_r.is_finite() {
        let radius_err = (final_radius_r - target_radius_r).abs() / target_radius_r;
        if radius_err > scenario.radius_tol_frac {
            failures.push(format!(
                "final radius {final_radius_r:.6} R⊕ off target {target_radius_r:.6} by {radius_err:.2}%"
            ));
        }
    }
    if target_inclination_rad.is_finite() {
        let inc_err = match scenario.target_type {
            OrbitType::RetrogradeEquatorial | OrbitType::EclipticRetrograde => {
                if is_prograde_velocity(spin, position, velocity) {
                    1.0
                } else {
                    0.0
                }
            }
            OrbitType::EclipticPrograde => {
                if !is_prograde_velocity(spin, position, velocity) {
                    1.0
                } else {
                    (final_inclination_rad - target_inclination_rad).abs()
                }
            }
            _ => (final_inclination_rad - target_inclination_rad).abs(),
        };
        if inc_err > scenario.inclination_tol_rad {
            failures.push(format!(
                "final inclination {:.2}° off target {:.2}°",
                final_inclination_rad.to_degrees(),
                target_inclination_rad.to_degrees()
            ));
        }
    }

    TransferRunReport {
        name: scenario.name.to_string(),
        completed,
        burn_steps,
        burn_time_s,
        initial_delta_v,
        theoretical_min_time_s,
        max_burn_step_jump_r,
        completion_jump_r,
        final_inclination_rad,
        final_radius_r,
        target_inclination_rad,
        target_radius_r,
        passed: failures.is_empty(),
        failures,
    }
}

fn target_circular_elements(
    sim: &Simulation,
    orbit_type: OrbitType,
    mut params: OrbitParams,
    scenario_obliquity_rad: f64,
) -> (f64, f64) {
    if scenario_obliquity_rad.abs() > 1e-9
        && matches!(
            orbit_type,
            OrbitType::EclipticPrograde | OrbitType::EclipticRetrograde
        )
    {
        params.obliquity_rad = scenario_obliquity_rad;
    }
    let target_radius_r =
        target_circular_radius(sim.central(), sim.scale(), orbit_type, params).unwrap_or(f64::NAN);
    let target_inclination_rad = match orbit_type {
        OrbitType::EclipticRetrograde => std::f64::consts::PI - params.obliquity_rad.abs(),
        _ => target_orbit_inclination(orbit_type, params),
    };
    (target_inclination_rad, target_radius_r)
}

/// Typical LEO altitude in Earth radii (~640 km).
const LEO_ALT_R: f64 = 0.1;
/// Mid-altitude circular orbit in Earth radii (~1,900 km).
const MID_ALT_R: f64 = 0.3;
/// Moderate inclination for plane-change matrix entries (~28.6°).
const MODERATE_INCLINATION_RAD: f64 = 0.5;
/// Obliquity used for ecliptic transfer scenarios (~23.5°).
const ECLIPTIC_OBLIQUITY_RAD: f64 = 0.41;
/// Shared elliptical LEO-class shape (perigee / apogee altitudes in Earth radii).
const ELLIPTICAL_LEO_SHAPE: (f64, f64) = (0.05, 0.25);
/// Shared elliptical mid-apogee shape for inclined transfers.
const ELLIPTICAL_INCLINED_SHAPE: (f64, f64) = (0.05, 0.3);

/// Shared defaults for catalog scenarios; override per entry with struct update syntax.
fn transfer_scenario_defaults() -> TransferScenario {
    TransferScenario {
        name: "",
        source_type: OrbitType::CircularEquatorial,
        source_params: OrbitParams::circular(LEO_ALT_R),
        target_type: OrbitType::CircularEquatorial,
        target_params: OrbitParams::circular(LEO_ALT_R),
        dt_s: 1.0,
        settle_steps: 120,
        mass: 1.0,
        max_thrust: 0.001,
        max_completion_jump_r: 0.02,
        max_burn_step_jump_r: 0.05,
        max_time_factor: 3.0,
        inclination_tol_rad: 0.1,
        radius_tol_frac: 0.01,
        max_burn_steps: 500_000,
        obliquity_rad: 0.0,
    }
}

/// Plane-change-only transfers (same altitude): tight finish jump, bounded burn time.
fn plane_change_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        max_completion_jump_r: 0.012,
        max_time_factor: f64::INFINITY,
        ..transfer_scenario_defaults()
    }
}

/// Slow descent transfers: lower thrust and extended step budget.
#[allow(dead_code)]
fn descent_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        max_thrust: 0.0002,
        max_burn_steps: 2_000_000,
        max_time_factor: f64::INFINITY,
        ..transfer_scenario_defaults()
    }
}

/// Altitude-only or Hohmann-style transfers: no upper time bound.
fn altitude_change_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        max_time_factor: f64::INFINITY,
        max_burn_steps: 2_000_000,
        ..transfer_scenario_defaults()
    }
}

/// Transfers involving GEO, graveyard, or tundra: allow larger finish nudge.
fn geo_class_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        max_completion_jump_r: 0.05,
        max_time_factor: f64::INFINITY,
        max_burn_steps: 2_000_000,
        ..transfer_scenario_defaults()
    }
}

/// Elliptical or Molniya targets: extended burn budget, relaxed finish jump.
fn elliptical_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        max_completion_jump_r: 0.05,
        max_time_factor: f64::INFINITY,
        max_burn_steps: 2_000_000,
        ..transfer_scenario_defaults()
    }
}

/// Ecliptic-plane transfers (requires non-zero obliquity).
fn ecliptic_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        obliquity_rad: ECLIPTIC_OBLIQUITY_RAD,
        max_completion_jump_r: 0.012,
        max_time_factor: f64::INFINITY,
        max_burn_steps: 500_000,
        ..transfer_scenario_defaults()
    }
}

/// Combined altitude and plane change: no upper time bound, moderate finish jump.
fn combined_scenario(
    name: &'static str,
    source_type: OrbitType,
    source_params: OrbitParams,
    target_type: OrbitType,
    target_params: OrbitParams,
) -> TransferScenario {
    TransferScenario {
        name,
        source_type,
        source_params,
        target_type,
        target_params,
        max_completion_jump_r: 0.02,
        max_time_factor: f64::INFINITY,
        max_burn_steps: 2_000_000,
        ..transfer_scenario_defaults()
    }
}

/// Standard catalog of transfer scenarios for manual / non-CI validation.
pub fn standard_transfer_scenarios() -> Vec<TransferScenario> {
    let leo_eq = OrbitParams::circular(LEO_ALT_R);
    let mid_eq = OrbitParams::circular(MID_ALT_R);
    let leo_iss = OrbitParams::circular_inclined(LEO_ALT_R, LOW_EARTH_INCLINATION_RAD);
    let leo_mod = OrbitParams::circular_inclined(LEO_ALT_R, MODERATE_INCLINATION_RAD);
    let mid_iss = OrbitParams::circular_inclined(MID_ALT_R, LOW_EARTH_INCLINATION_RAD);
    let geo = OrbitParams::geostationary(0.0);
    let elliptical_eq = OrbitParams::elliptical(ELLIPTICAL_LEO_SHAPE.0, ELLIPTICAL_LEO_SHAPE.1);
    let elliptical_inc = OrbitParams::elliptical_inclined(
        ELLIPTICAL_INCLINED_SHAPE.0,
        ELLIPTICAL_INCLINED_SHAPE.1,
        LOW_EARTH_INCLINATION_RAD,
    );
    let molniya = OrbitParams::molniya(std::f64::consts::PI);
    let ecliptic_leo = OrbitParams::circular(LEO_ALT_R);

    vec![
        // --- Plane change only (same altitude) ---
        plane_change_scenario(
            "equatorial_leo_to_inclined_leo_same_altitude",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::LowCircular,
            leo_iss,
        ),
        TransferScenario {
            name: "inclined_leo_to_equatorial_leo",
            source_type: OrbitType::LowCircular,
            source_params: leo_iss,
            target_type: OrbitType::CircularEquatorial,
            target_params: leo_eq,
            max_completion_jump_r: 0.005,
            max_time_factor: f64::INFINITY,
            ..transfer_scenario_defaults()
        },
        plane_change_scenario(
            "equatorial_leo_to_polar",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::CircularPolar,
            leo_eq,
        ),
        TransferScenario {
            name: "polar_leo_to_equatorial_leo",
            source_type: OrbitType::CircularPolar,
            source_params: leo_eq,
            target_type: OrbitType::CircularEquatorial,
            target_params: leo_eq,
            max_completion_jump_r: 0.012,
            max_time_factor: f64::INFINITY,
            max_burn_steps: 600_000,
            ..transfer_scenario_defaults()
        },
        plane_change_scenario(
            "polar_leo_to_inclined_leo",
            OrbitType::CircularPolar,
            leo_eq,
            OrbitType::LowCircular,
            leo_iss,
        ),
        TransferScenario {
            name: "inclined_leo_to_polar",
            source_type: OrbitType::LowCircular,
            source_params: leo_iss,
            target_type: OrbitType::CircularPolar,
            target_params: leo_eq,
            inclination_tol_rad: 0.08,
            max_completion_jump_r: 0.005,
            max_time_factor: f64::INFINITY,
            ..transfer_scenario_defaults()
        },
        plane_change_scenario(
            "inclined_leo_moderate_to_iss_same_altitude",
            OrbitType::LowCircular,
            leo_mod,
            OrbitType::LowCircular,
            leo_iss,
        ),
        plane_change_scenario(
            "inclined_leo_iss_to_moderate_same_altitude",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::LowCircular,
            leo_mod,
        ),
        plane_change_scenario(
            "equatorial_leo_to_moderate_inclination",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::LowCircular,
            leo_mod,
        ),
        plane_change_scenario(
            "moderate_inclination_leo_to_equatorial",
            OrbitType::LowCircular,
            leo_mod,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        // --- Altitude only (same plane / equatorial) ---
        altitude_change_scenario(
            "equatorial_leo_raise_to_mid",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::CircularEquatorial,
            mid_eq,
        ),
        altitude_change_scenario(
            "equatorial_mid_lower_to_leo",
            OrbitType::CircularEquatorial,
            mid_eq,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        altitude_change_scenario(
            "inclined_leo_raise_same_inclination",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::LowCircular,
            mid_iss,
        ),
        altitude_change_scenario(
            "inclined_leo_lower_same_inclination",
            OrbitType::LowCircular,
            mid_iss,
            OrbitType::LowCircular,
            leo_iss,
        ),
        // --- GEO class ---
        geo_class_scenario(
            "equatorial_leo_to_geo",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::Geostationary,
            geo,
        ),
        geo_class_scenario(
            "geo_to_equatorial_leo",
            OrbitType::Geostationary,
            geo,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        geo_class_scenario(
            "inclined_leo_to_geo",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::Geostationary,
            geo,
        ),
        geo_class_scenario(
            "geo_to_inclined_leo",
            OrbitType::Geostationary,
            geo,
            OrbitType::LowCircular,
            leo_iss,
        ),
        geo_class_scenario(
            "geo_to_graveyard",
            OrbitType::Geostationary,
            geo,
            OrbitType::Graveyard,
            OrbitParams::geostationary(0.0),
        ),
        geo_class_scenario(
            "graveyard_to_geo",
            OrbitType::Graveyard,
            OrbitParams::geostationary(0.0),
            OrbitType::Geostationary,
            geo,
        ),
        geo_class_scenario(
            "equatorial_leo_to_tundra",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::Tundra,
            OrbitParams::tundra(HIGH_INCLINATION_RAD),
        ),
        geo_class_scenario(
            "tundra_to_equatorial_leo",
            OrbitType::Tundra,
            OrbitParams::tundra(HIGH_INCLINATION_RAD),
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        // --- Combined altitude + inclination ---
        combined_scenario(
            "equatorial_leo_to_inclined_mid",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::LowCircular,
            mid_iss,
        ),
        combined_scenario(
            "inclined_mid_to_equatorial_leo",
            OrbitType::LowCircular,
            mid_iss,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        combined_scenario(
            "equatorial_mid_to_inclined_leo",
            OrbitType::CircularEquatorial,
            mid_eq,
            OrbitType::LowCircular,
            leo_iss,
        ),
        combined_scenario(
            "inclined_leo_to_equatorial_mid",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::CircularEquatorial,
            mid_eq,
        ),
        combined_scenario(
            "equatorial_leo_to_polar_mid",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::CircularPolar,
            mid_eq,
        ),
        TransferScenario {
            name: "polar_mid_to_equatorial_leo",
            source_type: OrbitType::CircularPolar,
            source_params: mid_eq,
            target_type: OrbitType::CircularEquatorial,
            target_params: leo_eq,
            max_completion_jump_r: 0.02,
            max_time_factor: f64::INFINITY,
            max_burn_steps: 2_000_000,
            ..transfer_scenario_defaults()
        },
        combined_scenario(
            "inclined_leo_raise_to_moderate_inclination",
            OrbitType::LowCircular,
            leo_mod,
            OrbitType::LowCircular,
            mid_iss,
        ),
        combined_scenario(
            "inclined_mid_iss_to_moderate_leo",
            OrbitType::LowCircular,
            mid_iss,
            OrbitType::LowCircular,
            leo_mod,
        ),
        // --- Retrograde (equatorial) ---
        altitude_change_scenario(
            "equatorial_leo_to_retrograde",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::RetrogradeEquatorial,
            leo_eq,
        ),
        altitude_change_scenario(
            "retrograde_leo_to_equatorial",
            OrbitType::RetrogradeEquatorial,
            leo_eq,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        // --- Elliptical / Molniya / ecliptic ---
        elliptical_scenario(
            "equatorial_leo_to_elliptical_equatorial",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::EllipticalEquatorial,
            elliptical_eq,
        ),
        elliptical_scenario(
            "elliptical_equatorial_to_leo",
            OrbitType::EllipticalEquatorial,
            elliptical_eq,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        elliptical_scenario(
            "inclined_leo_to_elliptical_inclined",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::EllipticalInclined,
            elliptical_inc,
        ),
        elliptical_scenario(
            "elliptical_inclined_to_leo",
            OrbitType::EllipticalInclined,
            elliptical_inc,
            OrbitType::LowCircular,
            leo_iss,
        ),
        elliptical_scenario(
            "equatorial_leo_to_molniya",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::Molniya,
            molniya,
        ),
        elliptical_scenario(
            "inclined_leo_to_molniya",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::Molniya,
            molniya,
        ),
        elliptical_scenario(
            "molniya_to_equatorial_leo",
            OrbitType::Molniya,
            molniya,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        ecliptic_scenario(
            "equatorial_leo_to_ecliptic_prograde",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::EclipticPrograde,
            ecliptic_leo,
        ),
        ecliptic_scenario(
            "ecliptic_prograde_to_equatorial_leo",
            OrbitType::EclipticPrograde,
            ecliptic_leo,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        ecliptic_scenario(
            "equatorial_leo_to_ecliptic_retrograde",
            OrbitType::CircularEquatorial,
            leo_eq,
            OrbitType::EclipticRetrograde,
            ecliptic_leo,
        ),
        ecliptic_scenario(
            "ecliptic_retrograde_to_equatorial_leo",
            OrbitType::EclipticRetrograde,
            ecliptic_leo,
            OrbitType::CircularEquatorial,
            leo_eq,
        ),
        ecliptic_scenario(
            "ecliptic_prograde_to_retrograde",
            OrbitType::EclipticPrograde,
            ecliptic_leo,
            OrbitType::EclipticRetrograde,
            ecliptic_leo,
        ),
        ecliptic_scenario(
            "inclined_leo_to_ecliptic_prograde",
            OrbitType::LowCircular,
            leo_iss,
            OrbitType::EclipticPrograde,
            ecliptic_leo,
        ),
    ]
}

impl TransferRunReport {
    /// Prints a one-line summary suitable for test logs.
    pub fn summary_line(&self) -> String {
        format!(
            "{status} {name}: steps={steps} t={time:.1}s (min={min:.2}s) jump_finish={jump:.4}R⊕ inc={inc:.1}° r={r:.3}R⊕",
            status = if self.passed { "PASS" } else { "FAIL" },
            name = self.name,
            steps = self.burn_steps,
            time = self.burn_time_s,
            min = self.theoretical_min_time_s,
            jump = self.completion_jump_r,
            inc = self.final_inclination_rad.to_degrees(),
            r = self.final_radius_r,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{run_transfer_scenario, standard_transfer_scenarios};

    #[test]
    #[ignore = "manual / non-CI transfer validation matrix"]
    fn run_standard_transfer_matrix() {
        let mut failed = 0;
        for scenario in standard_transfer_scenarios() {
            let report = run_transfer_scenario(&scenario);
            println!("{}", report.summary_line());
            for failure in &report.failures {
                println!("  - {failure}");
            }
            if !report.passed {
                failed += 1;
            }
        }
        assert_eq!(failed, 0, "{failed} transfer scenario(s) failed");
    }
}
