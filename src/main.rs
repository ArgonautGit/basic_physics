use units::*;
use crate::motion::{Motion, MotionState};

mod motion;
pub mod units;

mod ui;

fn main() -> eframe::Result {
    let mut motion = MotionState::default();
    motion.forces.push(Force::new(1.0, 1.0));

    for _ in 1..100 {
        (motion.update_motion(1.0));
        // println!("Position: {} Velocity: {}", motion.position, motion.velocity);
    }

    ui::run()
}
