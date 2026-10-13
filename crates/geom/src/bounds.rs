//! Axis-aligned bounding boxes.

use glam::Vec3;

/// An axis-aligned bounding box. `EMPTY` is the absorbing identity for [`Bounds::union`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Bounds {
    /// The empty box (min at +inf, max at -inf).
    pub const EMPTY: Self = Self { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) };

    /// Build a box from a single point.
    pub fn from_point(point: Vec3) -> Self {
        Self { min: point, max: point }
    }

    /// Build a box tightly containing every point. Returns [`Bounds::EMPTY`] for an empty slice.
    pub fn from_points(points: &[Vec3]) -> Self {
        points.iter().copied().fold(Self::EMPTY, Self::include_point)
    }

    /// The box containing both inputs.
    pub fn union(self, other: Self) -> Self {
        Self { min: self.min.min(other.min), max: self.max.max(other.max) }
    }

    /// Grow the box to include a point.
    pub fn include_point(mut self, point: Vec3) -> Self {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
        self
    }

    /// Whether the box is empty (no point has been included).
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// The box centre, or the origin when empty.
    pub fn center(&self) -> Vec3 {
        if self.is_empty() { Vec3::ZERO } else { (self.min + self.max) * 0.5 }
    }

    /// The box extents (max - min), or zero when empty.
    pub fn size(&self) -> Vec3 {
        if self.is_empty() { Vec3::ZERO } else { self.max - self.min }
    }

    /// The radius of the bounding sphere around the centre.
    pub fn radius(&self) -> f32 {
        if self.is_empty() { 0.0 } else { self.size().length() * 0.5 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_has_no_volume() {
        let empty = Bounds::EMPTY;
        assert!(empty.is_empty());
        assert_eq!(empty.center(), Vec3::ZERO);
        assert_eq!(empty.radius(), 0.0);
    }

    #[test]
    fn from_points_is_tight() {
        let bounds = Bounds::from_points(&[Vec3::new(-1.0, 2.0, 0.0), Vec3::new(3.0, -2.0, 5.0)]);
        assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, 0.0));
        assert_eq!(bounds.max, Vec3::new(3.0, 2.0, 5.0));
        assert_eq!(bounds.center(), Vec3::new(1.0, 0.0, 2.5));
    }

    #[test]
    fn union_of_empty_and_box_is_the_box() {
        let boxx = Bounds::from_point(Vec3::ONE);
        assert_eq!(Bounds::EMPTY.union(boxx), boxx);
    }
}
