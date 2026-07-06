//! Physical constants and simulation unit scaling.

/// Newtonian gravitational constant in SI units (m³ kg⁻¹ s⁻²).
pub const G_SI: f64 = 6.674_30e-11;

/// Earth mass in kilograms.
pub const M_EARTH_KG: f64 = 5.972_2e24;

/// Earth mean equatorial radius in meters.
pub const R_EARTH_M: f64 = 6_371_000.0;

/// Converts game-scale multipliers into SI lengths and times for physics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationScale {
    /// Meters represented by one simulation distance unit (Earth radius by default).
    pub length_scale: f64,
    /// Real seconds represented by one simulation second.
    pub time_scale: f64,
}

impl Default for SimulationScale {
    fn default() -> Self {
        Self {
            length_scale: R_EARTH_M,
            time_scale: 1.0,
        }
    }
}

impl SimulationScale {
    /// One simulation distance unit equals one Earth radius.
    pub fn earth_radii() -> Self {
        Self::default()
    }

    /// SI length of one simulation distance unit.
    pub fn length_si(&self) -> f64 {
        self.length_scale
    }

    /// Computes the gravitational parameter μ in simulation units.
    ///
    /// Positions are in Earth-radii units; accelerations use `a = -μ r / |r|³`.
    pub fn gravitational_parameter(&self, mass_earth: f64) -> f64 {
        let mass_si = mass_earth * M_EARTH_KG;
        let length_si = self.length_si();
        G_SI * mass_si / (length_si * length_si * length_si)
    }
}

#[cfg(test)]
mod tests {
    use super::{G_SI, M_EARTH_KG, R_EARTH_M, SimulationScale};

    #[test]
    fn earth_surface_gravity_is_about_g() {
        let scale = SimulationScale::earth_radii();
        let mu = scale.gravitational_parameter(1.0);
        let surface_accel = mu / (1.0 * 1.0);
        let g = 9.806_65;
        let expected = G_SI * M_EARTH_KG / (R_EARTH_M * R_EARTH_M);
        assert!((surface_accel * scale.length_si() - expected).abs() < 0.05);
        assert!((expected - g).abs() < 0.2);
    }
}
