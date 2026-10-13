//! An orbit camera around a target point.

use glam::{Mat4, Vec3};
use substancecraft_geom::Bounds;

/// A turntable camera that orbits a target, pans, and dollies along the view direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    /// The point the camera looks at.
    pub target: Vec3,
    /// Distance from the target.
    pub distance: f32,
    /// Horizontal angle in radians.
    pub yaw: f32,
    /// Vertical angle in radians, clamped away from the poles.
    pub pitch: f32,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    /// Near plane.
    pub znear: f32,
    /// Far plane.
    pub zfar: f32,
}

/// The pitch clamp, just short of the poles so `look_at` stays well-defined.
const MAX_PITCH: f32 = 1.553_343; // ~89 degrees

impl Default for OrbitCamera {
    fn default() -> Self {
        Self { target: Vec3::ZERO, distance: 4.0, yaw: 0.7, pitch: 0.45, fov_y: 45.0f32.to_radians(), znear: 0.01, zfar: 1000.0 }
    }
}

impl OrbitCamera {
    /// The camera position derived from target, distance, yaw and pitch.
    pub fn position(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let direction = Vec3::new(cos_pitch * sin_yaw, sin_pitch, cos_pitch * cos_yaw);
        self.target + direction * self.distance
    }

    /// The view matrix (right-handed, looking at the target).
    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.position(), self.target, Vec3::Y)
    }

    /// The projection matrix for a given aspect ratio (wgpu NDC: Z in `0..=1`, Y up).
    pub fn projection(&self, aspect: f32) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(self.fov_y, aspect.max(1e-3), self.znear, self.zfar)
    }

    /// The combined view-projection matrix.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Orbit by a screen-space delta in radians-scaled units.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.01;
        self.pitch = (self.pitch + delta_y * 0.01).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Pan the target in the camera's screen plane, keeping the same distance.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32, viewport_height: f32) {
        let forward = (self.target - self.position()).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let world_per_pixel = 2.0 * self.distance * (self.fov_y * 0.5).tan() / viewport_height.max(1.0);
        self.target += (-right * delta_x + up * delta_y) * world_per_pixel;
    }

    /// Dolly toward or away from the target. Positive `amount` moves closer.
    pub fn dolly(&mut self, amount: f32) {
        self.distance = (self.distance * (1.0 - amount)).clamp(0.01, 1.0e6);
    }

    /// Frame a bounding box, setting the target to its centre and the distance to fit its radius.
    pub fn focus(&mut self, bounds: Bounds) {
        if bounds.is_empty() {
            return;
        }
        self.target = bounds.center();
        let radius = bounds.radius().max(1e-3);
        self.distance = (radius / (self.fov_y * 0.5).tan()) * 1.4;
        self.znear = (self.distance * 0.001).max(1e-4);
        self.zfar = (self.distance * 200.0).max(10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_is_distance_from_target() {
        let camera = OrbitCamera::default();
        assert!((camera.position().distance(camera.target) - camera.distance).abs() < 1e-4);
    }

    #[test]
    fn focus_centres_and_fits() {
        let mut camera = OrbitCamera::default();
        camera.focus(Bounds::from_points(&[Vec3::splat(-1.0), Vec3::splat(1.0)]));
        assert!(camera.target.length() < 1e-6);
        assert!(camera.distance > 1.0);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut camera = OrbitCamera::default();
        for _ in 0..1000 {
            camera.orbit(0.0, 10.0);
        }
        assert!(camera.pitch <= MAX_PITCH);
    }

    #[test]
    fn dolly_stays_positive() {
        let mut camera = OrbitCamera::default();
        for _ in 0..100 {
            camera.dolly(1.0);
        }
        assert!(camera.distance > 0.0);
    }

    #[test]
    fn view_projection_is_finite() {
        let camera = OrbitCamera::default();
        let vp = camera.view_projection(16.0 / 9.0);
        assert!(vp.to_cols_array().iter().all(|value| value.is_finite()));
    }
}
