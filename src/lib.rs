//! Orbital Movement Gdextension — core library.
//!
//! Provides 3D two-body orbital mechanics in Earth-normalized units with a
//! planet-centered inertial frame, orbit helpers, star apparent motion, and
//! surface visibility calculations.

#![deny(missing_docs)]

/// Error types for library operations.
pub mod error;

mod central_body;
mod collision;
mod ground_track;
mod gravity;
mod integrator;
mod math;
mod orbits;
mod simulation;
mod small_body;
mod star;
mod surface_geometry;
mod thrust_frame;
mod transfer_burn;
mod transfer_validation;
mod transfer_viability;
mod units;
mod visibility;

pub use central_body::CentralBody;
pub use error::ProjectError;
pub use ground_track::{
    OrbitalSurfaceTrack, is_ephemeral_trajectory, orbital_surface_track, project_to_surface,
};
pub use math::Vec3;
pub use orbits::{
    HIGH_INCLINATION_RAD, LOW_EARTH_INCLINATION_RAD, MOLNIYA_APOGEE_ALTITUDE_R,
    MOLNIYA_PERIGEE_ALTITUDE_R, OrbitParams, OrbitType, OrbitUiDefaults,
    build_orbit_params_from_ui, geostationary_altitude_earth_radii, geostationary_radius,
    is_elliptical_orbit_type, orbit_plane_change_rad, orbit_ui_defaults,
    orbit_uses_computed_altitude, orbit_uses_elliptical_params, orbit_uses_inclination_param,
    orbital_inclination_rad, orbital_period, specific_orbital_energy, target_orbit_inclination,
    target_orbit_reference_radius,
};
pub use simulation::Simulation;
pub use small_body::{BodyId, BodyState};
pub use surface_geometry::{
    SurfaceMesh, SurfaceTessellationConfig, angular_distance, ground_track_is_closed,
    slerp_direction, tessellate_spherical_corridor, tessellate_spherical_polyline,
};
pub use star::{StarConfig, ecliptic_plane_normal};
pub use thrust_frame::thrust_flags;
pub use thrust_frame::{
    LocalThrustFrame, ThrustDirection, local_thrust_frame, thrust_direction_from_flags,
    thrust_direction_unit,
};
pub use transfer_burn::{
    GuidedTransfer, TRANSFER_SNAP_EPSILON, TRANSFER_SNAP_POSITION_EPSILON, TransferBurnStatus,
    TransferBurnTracker,
};
pub use transfer_validation::{
    TransferRunReport, TransferScenario, run_transfer_scenario, standard_transfer_scenarios,
};
pub use transfer_viability::{
    TransferAvailability, TransferViabilityConfig, TransferViabilityReport,
    assess_transfer_viability,
};
pub use units::{G_SI, M_EARTH_KG, R_EARTH_M, SimulationScale};
pub use visibility::{horizon_half_angle, visibility_cap_mesh, visible_surface_area};
