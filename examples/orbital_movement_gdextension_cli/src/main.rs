//! Native command-line tour of `orbital_movement_gdextension` — no Godot required.
//!
//! Run everything:
//! ```text
//! cargo run -p orbital_movement_gdextension_cli
//! ```
//!
//! Run one section:
//! ```text
//! cargo run -p orbital_movement_gdextension_cli -- viability transfer
//! ```

use std::collections::HashSet;
use std::env;

use orbital_movement_gdextension::{
    LOW_EARTH_INCLINATION_RAD, OrbitParams, OrbitType, Simulation, StarConfig,
    TransferAvailability, TransferBurnStatus, TransferScenario, TransferViabilityConfig,
    assess_transfer_viability, orbital_period, run_transfer_scenario, specific_orbital_energy,
    thrust_flags,
};

const ROTATION_PERIOD_S: f64 = 86_400.0;
const LEO_ALT_R: f64 = 0.1;
const DEMO_THRUST: f64 = 0.001;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sections = parse_sections(env::args().skip(1).collect());
    print_banner(&sections);

    if sections.contains("orbit") {
        demo_orbit_propagation()?;
    }
    if sections.contains("visibility") {
        demo_visibility()?;
    }
    if sections.contains("star") {
        demo_star_motion()?;
    }
    if sections.contains("viability") {
        demo_transfer_viability()?;
    }
    if sections.contains("transfer") {
        demo_guided_transfer()?;
    }
    if sections.contains("instant") {
        demo_instant_transfer()?;
    }
    if sections.contains("thrust") {
        demo_manual_thrust()?;
    }

    println!("\nDone. Godot bindings mirror these APIs — see docs/godot.md.");
    Ok(())
}

fn print_banner(sections: &HashSet<&'static str>) {
    println!("Orbital Movement — native API demo (no Godot)");
    println!(
        "Sections: {}",
        sections.iter().copied().collect::<Vec<_>>().join(", ")
    );
    println!("Usage: cargo run -p orbital_movement_gdextension_cli -- [SECTION...]");
    println!("       orbit | visibility | star | viability | transfer | instant | thrust | all");
}

fn parse_sections(args: Vec<String>) -> HashSet<&'static str> {
    const ALL: &[&str] = &[
        "orbit",
        "visibility",
        "star",
        "viability",
        "transfer",
        "instant",
        "thrust",
    ];
    if args.is_empty() || args.iter().any(|a| a == "all") {
        return ALL.iter().copied().collect();
    }

    let mut selected = HashSet::new();
    for arg in args {
        let key = arg.to_ascii_lowercase();
        if ALL.contains(&key.as_str()) {
            selected.insert(match key.as_str() {
                "orbit" => "orbit",
                "visibility" => "visibility",
                "star" => "star",
                "viability" => "viability",
                "transfer" => "transfer",
                "instant" => "instant",
                "thrust" => "thrust",
                _ => unreachable!(),
            });
        } else {
            eprintln!("warning: unknown section {arg:?} (ignored)");
        }
    }
    if selected.is_empty() {
        ALL.iter().copied().collect()
    } else {
        selected
    }
}

fn demo_orbit_propagation() -> Result<(), Box<dyn std::error::Error>> {
    section("Orbit propagation");
    let mut sim = Simulation::earth_like(ROTATION_PERIOD_S)?;
    let id = sim.create_body_in_orbit(
        OrbitType::LowCircular,
        OrbitParams::low_circular(LEO_ALT_R),
        1.0,
    )?;

    let pos0 = sim.position(id)?;
    let vel0 = sim.velocity(id)?;
    let radius = pos0.length();
    let mu = sim.mu();
    let period = orbital_period(mu, radius)?;
    let energy0 = specific_orbital_energy(mu, pos0, vel0);

    println!("Spawned LowCircular (ISS-like inclination) at r = {radius:.4} R⊕");
    println!("  position {pos0:?}");
    println!("  velocity {vel0:?}");
    println!(
        "  orbital period ≈ {period:.1} s ({:.1} min)",
        period / 60.0
    );
    println!("  specific orbital energy = {energy0:.6e}");

    let dt = 60.0;
    let steps = (period / dt).round() as u32;
    for _ in 0..steps {
        sim.step(dt)?;
    }
    let pos1 = sim.position(id)?;
    let vel1 = sim.velocity(id)?;
    let energy1 = specific_orbital_energy(mu, pos1, vel1);
    let drift = (energy1 - energy0).abs();
    let separation = (pos1 - pos0).length();

    println!("After one revolution ({steps} × {dt:.0} s steps, unthrusted):");
    println!("  position {pos1:?}");
    println!("  |Δr| from start = {separation:.4} R⊕");
    println!("  |ΔE/E| = {:.3e}", drift / energy0.abs());
    Ok(())
}

fn demo_visibility() -> Result<(), Box<dyn std::error::Error>> {
    section("Surface visibility");
    let mut sim = Simulation::earth_like(ROTATION_PERIOD_S)?;
    let leo = sim.create_body_in_orbit(
        OrbitType::CircularEquatorial,
        OrbitParams::circular(LEO_ALT_R),
        1.0,
    )?;
    let geo_alt = sim.geostationary_altitude_earth_radii()?;
    let geo = sim.create_body_in_orbit(
        OrbitType::Geostationary,
        OrbitParams::geostationary(0.0),
        1.0,
    )?;

    for (label, id) in [("LEO", leo), ("GEO", geo)] {
        let horizon = sim.horizon_half_angle(id)?.to_degrees();
        let area = sim.visible_surface_area(id)?;
        let altitude = sim.position(id)?.length() - sim.planet_radius();
        println!(
            "{label} (altitude {altitude:.3} R⊕): horizon half-angle {horizon:.2}°, visible cap area {area:.4} sr"
        );
    }
    println!("GEO reference altitude from central body: {geo_alt:.3} R⊕");
    Ok(())
}

fn demo_star_motion() -> Result<(), Box<dyn std::error::Error>> {
    section("Star / ecliptic plane");
    let obliquity = 0.41_f64;
    let mut sim = Simulation::earth_like_with_obliquity(ROTATION_PERIOD_S, obliquity)?;
    sim.set_star(StarConfig::new(
        100.0,
        obliquity,
        std::f64::consts::FRAC_PI_2,
    ));

    let inertial = sim.star_inertial_position();
    let equinox = sim.star_apparent_position(0.0);
    let quarter = sim.star_apparent_position(std::f64::consts::FRAC_PI_2);

    println!(
        "Obliquity {:.1}° — sun at 100 R⊕ on the ecliptic",
        obliquity.to_degrees()
    );
    println!("  inertial position:      {inertial:?}");
    println!("  apparent @ spin 0:     {equinox:?}");
    println!("  apparent @ spin π/2:   {quarter:?}");
    Ok(())
}

fn demo_transfer_viability() -> Result<(), Box<dyn std::error::Error>> {
    section("Transfer viability (menu filtering)");
    let mut sim = Simulation::earth_like(ROTATION_PERIOD_S)?;
    let id = sim.create_body_in_orbit(
        OrbitType::CircularEquatorial,
        OrbitParams::circular(LEO_ALT_R),
        1.0,
    )?;
    sim.set_max_thrust(id, DEMO_THRUST)?;

    let body = (sim.position(id)?, sim.velocity(id)?, sim.max_thrust(id)?);
    let config = TransferViabilityConfig::default();

    let targets = [
        (
            "ISS-like LEO",
            OrbitType::LowCircular,
            OrbitParams::circular_inclined(LEO_ALT_R, LOW_EARTH_INCLINATION_RAD),
        ),
        (
            "GEO",
            OrbitType::Geostationary,
            OrbitParams::geostationary(0.0),
        ),
        (
            "Polar LEO",
            OrbitType::CircularPolar,
            OrbitParams::circular(LEO_ALT_R),
        ),
        (
            "Molniya",
            OrbitType::Molniya,
            OrbitParams::molniya(std::f64::consts::PI),
        ),
        (
            "Elliptical equatorial",
            OrbitType::EllipticalEquatorial,
            OrbitParams::elliptical(LEO_ALT_R, 0.5),
        ),
    ];

    println!(
        "From equatorial LEO with max_thrust = {DEMO_THRUST} (a = {:.4} R⊕/s²):",
        DEMO_THRUST
    );
    println!(
        "{:<22} {:<12} {:>10} {:>12} reason",
        "Target", "Status", "Δv", "est. time"
    );
    println!("{}", "-".repeat(72));

    for (label, orbit_type, params) in targets {
        let report = assess_transfer_viability(
            sim.central(),
            sim.scale(),
            body.0,
            body.1,
            body.2,
            1.0,
            orbit_type,
            params,
            &config,
        )?;
        let reason = report.reason.as_deref().unwrap_or("");
        println!(
            "{:<22} {:<12} {:>10.4} {:>10.1}s {}",
            label,
            availability_label(report.availability),
            report.initial_delta_v,
            report.estimated_guided_burn_time_s,
            reason
        );
    }
    Ok(())
}

fn demo_guided_transfer() -> Result<(), Box<dyn std::error::Error>> {
    section("Guided transfer (begin_transfer_to_orbit)");
    let scenario = TransferScenario {
        name: "cli_equatorial_leo_to_iss",
        source_type: OrbitType::CircularEquatorial,
        source_params: OrbitParams::circular(LEO_ALT_R),
        target_type: OrbitType::LowCircular,
        target_params: OrbitParams::circular_inclined(LEO_ALT_R, LOW_EARTH_INCLINATION_RAD),
        settle_steps: 30,
        max_thrust: DEMO_THRUST,
        max_completion_jump_r: 0.012,
        max_time_factor: f64::INFINITY,
        ..default_transfer_scenario()
    };

    let report = run_transfer_scenario(&scenario);
    println!("Scenario: {}", report.name);
    println!("  completed: {}", report.completed);
    println!(
        "  burn: {} steps, {:.1} s (min {:.2} s at thrust limit)",
        report.burn_steps, report.burn_time_s, report.theoretical_min_time_s
    );
    println!(
        "  initial Δv: {:.4} R⊕/s  finish jump: {:.4} R⊕",
        report.initial_delta_v, report.completion_jump_r
    );
    println!(
        "  final: inc {:.1}° (target {:.1}°), r {:.4} R⊕ (target {:.4} R⊕)",
        report.final_inclination_rad.to_degrees(),
        report.target_inclination_rad.to_degrees(),
        report.final_radius_r,
        report.target_radius_r
    );
    if !report.passed {
        println!("  checks failed: {}", report.failures.join("; "));
    }
    Ok(())
}

fn demo_instant_transfer() -> Result<(), Box<dyn std::error::Error>> {
    section("Instant transfer (apply_transfer_to_orbit)");
    let mut sim = Simulation::earth_like(ROTATION_PERIOD_S)?;
    let id = sim.create_body_in_orbit(
        OrbitType::CircularEquatorial,
        OrbitParams::circular(LEO_ALT_R),
        1.0,
    )?;

    const MID_ALT_R: f64 = 0.3;
    let target = OrbitParams::circular(MID_ALT_R);
    let dv = sim.required_delta_v_to_orbit(id, OrbitType::CircularEquatorial, target)?;
    println!("Coplanar LEO → mid-altitude equatorial:");
    println!("  required Δv: {dv:?} (|Δv| = {:.4})", dv.length());

    sim.apply_transfer_to_orbit(id, OrbitType::CircularEquatorial, target)?;

    for _ in 0..60 {
        sim.step(60.0)?;
    }
    let radius = sim.position(id)?.length() - sim.planet_radius();
    println!("  after 1 h flight: altitude {radius:.3} R⊕ (target {MID_ALT_R:.3} R⊕)");
    Ok(())
}

fn demo_manual_thrust() -> Result<(), Box<dyn std::error::Error>> {
    section("Manual thrust cancels guided transfer");
    let mut sim = Simulation::earth_like(ROTATION_PERIOD_S)?;
    let id = sim.create_body_in_orbit(
        OrbitType::CircularEquatorial,
        OrbitParams::circular(LEO_ALT_R),
        1.0,
    )?;
    sim.set_max_thrust(id, DEMO_THRUST)?;
    let target = OrbitParams::circular_inclined(LEO_ALT_R, LOW_EARTH_INCLINATION_RAD);
    sim.begin_transfer_to_orbit(id, OrbitType::LowCircular, target)?;
    assert_eq!(sim.transfer_burn_status(id), TransferBurnStatus::Burning);
    sim.step(60.0)?;
    let progress = sim.transfer_burn_progress(id);

    let direction = sim.thrust_direction_from_flags(id, thrust_flags::PROGRADE)?;
    sim.apply_force(id, direction, DEMO_THRUST)?;
    assert_eq!(sim.transfer_burn_status(id), TransferBurnStatus::Idle);

    println!("Active transfer cleared when apply_force is used.");
    println!("  progress was {:.0}% before cancel", progress * 100.0);
    Ok(())
}

fn default_transfer_scenario() -> TransferScenario {
    TransferScenario {
        name: "",
        source_type: OrbitType::CircularEquatorial,
        source_params: OrbitParams::circular(LEO_ALT_R),
        target_type: OrbitType::CircularEquatorial,
        target_params: OrbitParams::circular(LEO_ALT_R),
        dt_s: 1.0,
        settle_steps: 120,
        mass: 1.0,
        max_thrust: DEMO_THRUST,
        max_completion_jump_r: 0.02,
        max_burn_step_jump_r: 0.05,
        max_time_factor: 3.0,
        inclination_tol_rad: 0.1,
        radius_tol_frac: 0.01,
        max_burn_steps: 500_000,
        obliquity_rad: 0.0,
    }
}

fn availability_label(availability: TransferAvailability) -> &'static str {
    match availability {
        TransferAvailability::Available => "available",
        TransferAvailability::Impractical => "impractical",
        TransferAvailability::Unavailable => "unavailable",
    }
}

fn section(title: &str) {
    println!("\n--- {title} ---");
}
