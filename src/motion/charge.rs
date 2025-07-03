use crate::units::Force;

pub type Coulomb = f32;

#[allow(dead_code)]
pub struct Charge {
    strength: Coulomb,
}

#[allow(dead_code)]
impl Charge {
    pub fn force(&self, other: Self) -> Force {
        todo!()
    }
}

