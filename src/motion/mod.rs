use crate::units::*;

pub mod vector;

#[allow(dead_code)]
pub trait Motion {
    fn update_motion(&mut self, dt: Time) -> &mut Self;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MotionState {
    pub position: Position,
    pub velocity: Velocity,
    pub acceleration: Acceleration,
    pub forces: Vec<Force>,

    pub mass: Mass,
}

impl Motion for MotionState {
    fn update_motion(&mut self, dt: Time) -> &mut Self {
        self.update_acceleration().update_velocity(dt).update_position(dt)
    }
}

impl Default for MotionState {
    fn default() -> Self {
        MotionState {
            acceleration: Position::default(),
            mass: 1.0,
            position: Position::default(),
            forces: Vec::default(),
            velocity: Velocity::default(),
        }
    }
}

impl MotionState {
    fn update_acceleration(&mut self) -> &mut Self {
        self.acceleration = Acceleration::default();
        for force in &self.forces {
            self.acceleration += *force / self.mass;
        }

        dbg!(self)
    }

    fn update_velocity(&mut self, dt: Time) -> &mut Self {
        self.velocity += self.acceleration * dt;
        self
    }

    fn update_position(&mut self, dt: Time) -> &mut Self {
        self.position += self.velocity * dt;
        self
    }
}

