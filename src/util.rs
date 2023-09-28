use bevy::prelude::*;

pub struct SegmentedJumpInitialVelocityCalculator {
    height: f32,
    kinetic_energy: f32,
}

impl SegmentedJumpInitialVelocityCalculator {
    pub fn new(total_height: f32) -> Self {
        Self {
            height: total_height,
            kinetic_energy: 0.0,
        }
    }

    pub fn add_segment(&mut self, gravity: f32, velocity_threshold: f32) -> &mut Self {
        if self.height <= 0.0 {
            // No more height to jump
            return self;
        }

        let kinetic_energy_at_velocity_threshold = 0.5 * velocity_threshold.powi(2);

        let transferred_energy = kinetic_energy_at_velocity_threshold - self.kinetic_energy;
        if transferred_energy <= 0.0 {
            // Already faster than that velocity
            return self;
        }

        let segment_height = transferred_energy / gravity;
        if self.height < segment_height {
            // This segment will be the last
            self.kinetic_energy += self.height * gravity;
            self.height = 0.0;
        } else {
            self.kinetic_energy += transferred_energy;
            self.height -= segment_height;
        }

        self
    }

    pub fn kinetic_energy(&self) -> f32 {
        self.kinetic_energy
    }
}

pub struct ProjectionPlaneForRotation {
    pub forward: Vec3,
    pub sideways: Vec3,
}

impl ProjectionPlaneForRotation {
    pub fn from_up_and_fowrard(up: Vec3, forward: Vec3) -> Self {
        Self {
            forward,
            sideways: up.cross(forward),
        }
    }

    pub fn from_up_using_default_forward(up: Vec3) -> Self {
        Self::from_up_and_fowrard(up, Vec3::NEG_Z)
    }

    pub fn project_and_normalize(&self, vector: Vec3) -> Vec2 {
        Vec2::new(vector.dot(self.forward), vector.dot(self.sideways)).normalize_or_zero()
    }

    pub fn rotation_to_set_forward(&self, current_forward: Vec3, desired_forward: Vec3) -> f32 {
        let rotation_to_set_forward = Quat::from_rotation_arc_2d(
            self.project_and_normalize(current_forward),
            self.project_and_normalize(desired_forward),
        );
        rotation_to_set_forward.xyz().z
    }
}
