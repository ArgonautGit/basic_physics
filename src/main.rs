#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use physics::motion::{Motion, MotionState};
use physics::units::*;

fn main() -> eframe::Result {
    let mut motion = MotionState::default();
    motion.forces.push(Force::new(1.0, 1.0));

    for _ in 1..100 {
        motion.update_motion(1.0);
        // println!("Position: {} Velocity: {}", motion.position, motion.velocity);
    }

    physics::ui::run()
}
