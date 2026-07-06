//! Surface contact handling for small bodies.

use crate::math::Vec3;
use crate::small_body::{BodyState, SmallBody};

/// Result of resolving surface contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceResolution {
    /// Body remains in free flight above the surface.
    AboveSurface,
    /// Body was clamped to the surface and stopped.
    ClampedToSurface,
}

/// Clamps a body to the spherical surface if it penetrates or reaches it.
pub fn resolve_surface_contact(body: &mut SmallBody, surface_radius: f64) -> SurfaceResolution {
    let position = body.position();
    let radius = position.length();
    if radius >= surface_radius {
        return SurfaceResolution::AboveSurface;
    }

    let radial = if radius > f64::EPSILON {
        position / radius
    } else {
        Vec3::Y
    };

    body.set_position(radial * surface_radius);

    let velocity = body.velocity();
    let radial_speed = velocity.dot(radial);
    if radial_speed < 0.0 {
        body.set_velocity(velocity - radial * radial_speed);
    }

    body.set_state(BodyState::SurfaceContact);
    SurfaceResolution::ClampedToSurface
}

#[cfg(test)]
mod tests {
    use super::resolve_surface_contact;
    use crate::math::Vec3;
    use crate::small_body::{BodyId, BodyState, SmallBody};

    #[test]
    fn clamps_inward_trajectory() {
        let mut body = SmallBody::new(
            BodyId(1),
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            1.0,
        );
        resolve_surface_contact(&mut body, 1.0);
        assert_eq!(body.state(), BodyState::SurfaceContact);
        assert!((body.position().length() - 1.0).abs() < 1e-12);
        assert!(body.velocity().x >= 0.0);
    }
}
