use orbital_movement_gdextension::{OrbitParams, OrbitType, Simulation, specific_orbital_energy};

#[test]
fn simulation_leo_orbit_integration() {
    let mut sim = Simulation::earth_like(86_400.0).expect("simulation should initialize");
    let id = sim
        .create_body_in_orbit(OrbitType::LowCircular, OrbitParams::low_circular(0.05), 1.0)
        .expect("body should be created");

    let mu = sim.mu();
    let initial_energy = specific_orbital_energy(
        mu,
        sim.position(id).expect("position"),
        sim.velocity(id).expect("velocity"),
    );

    for _ in 0..100 {
        sim.step(60.0).expect("step should succeed");
    }

    let final_energy = specific_orbital_energy(
        mu,
        sim.position(id).expect("position"),
        sim.velocity(id).expect("velocity"),
    );

    assert!((final_energy - initial_energy).abs() < 1e-5);
    assert!(sim.visible_surface_area(id).expect("area") > 0.0);
}

#[test]
fn star_apparent_motion_is_periodic() {
    let sim = Simulation::earth_like(86_400.0).expect("simulation should initialize");
    let start = sim.star_apparent_position(0.0);
    let end = sim.star_apparent_position(std::f64::consts::TAU);
    assert!((start - end).length() < 1e-10);
}
