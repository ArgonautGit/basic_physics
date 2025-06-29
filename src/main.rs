use units::*;
use crate::motion::{Motion, MotionState};

mod motion;
pub mod units;

fn main() {
    let mut motion = MotionState::default();
    motion.forces.push(Force::new(1.0, 1.0));

    for _ in 1..100 {
        dbg!(motion.update_motion(1.0));
        // println!("Position: {} Velocity: {}", motion.position, motion.velocity);
    }
}
