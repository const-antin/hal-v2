#[derive(PartialEq, Debug, Clone)]
pub enum Scalar {
    I32(i32), FP32(f32), Bit(bool), DontCare, Empty
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

    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Scalar::DontCare, _) => true,
            (_, Scalar::DontCare) => true,
            (Scalar::Bit(b1), Scalar::Bit(b2)) => b1 == b2,
            (Scalar::I32(s1), Scalar::I32(s2)) => s1 == s2,
            (Scalar::FP32(f1), Scalar::FP32(f2)) => f1 == f2,
            _ => panic!("Incompatible types.")
        }
    }
}