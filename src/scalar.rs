#[derive(PartialEq, Debug, Clone)]
pub enum Scalar {
    I32(i32), FP32(f32), Bit(bool), Empty
}

impl Scalar {
    pub fn width(&self) -> usize {
        match self {
            Scalar::Bit(_) => 1,
            Scalar::I32(_) => 32,
            Scalar::FP32(_) => 32,
            _ => panic!("No idea what width that is.")
        }
    }
}