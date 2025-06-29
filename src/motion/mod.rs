mod units;
mod vector;

use units::*;

#[allow(dead_code)]
pub trait Motion {
    fn update_motion(state: &mut MotionState, dt: Time) -> &mut MotionState {
        state.update_acceleration().update_velocity().update_position()
    }
}

#[allow(dead_code)]
pub struct MotionState {
    position: Position,
    velocity: Velocity,
    acceleration: Acceleration,
    forces: Vec<Force>,

    mass: Mass,
}

impl MotionState {
    fn update_acceleration(&mut self) -> &mut Self {
        let mut total_acceleration = Acceleration::default();
        for force in &self.forces {
            total_acceleration += (*force) / self.mass;
        }

        self
    }

    fn update_velocity(&mut self) -> &mut Self {
        
    }

    fn update_position(&mut self) -> &mut Self {
        todo!()
    }
}

