use std::collections::HashSet;

use crate::value::Value;

// This should maybe be moved to pcu.rs or to pipeline_stage.rs.
// TODO: Change the names instead of supressing the warnings.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum ALUInput {
    PREV, PREV_BELOW, NEXT
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum ALUOp {
    ADD_I32, SUB_I32, MUL_I32, DIV_I32,
    ADD_FP32, SUB_FP32, MUL_FP32, DIV_FP32
}

#[derive(Clone, Copy)]
pub struct ALURtConfig {
    pub op: ALUOp,
    pub in_a: ALUInput,
    pub in_b: ALUInput
}

#[derive(Clone)]
pub struct ALUHwConfig {
    pub supported_ops: HashSet<ALUOp>
}

impl ALUOp { 
    pub fn delay(&self) -> usize { 
        match self { // TODO: These values are guessed.
            Self::ADD_I32 => 1,
            Self::SUB_I32 => 1,
            Self::MUL_I32 => 2,
            Self::DIV_I32 => 4,
            Self::ADD_FP32 => 2,
            Self::SUB_FP32 => 2,
            Self::MUL_FP32 => 3,
            Self::DIV_FP32 => 5
        }
    }

    pub fn apply(&self, lhs: &Value, rhs: &Value) -> Value {
        match (self, lhs, rhs) {
            (Self::ADD_I32, Value::I32(x), Value::I32(y)) => Value::I32(x+y),
            (Self::SUB_I32, Value::I32(x), Value::I32(y)) => Value::I32(x-y),
            (Self::MUL_I32, Value::I32(x), Value::I32(y)) => Value::I32(x*y),
            (Self::DIV_I32, Value::I32(x), Value::I32(y)) => Value::I32(x/y),
            (Self::SUB_FP32, Value::FP32(x), Value::FP32(y)) => Value::FP32(x+y),
            (Self::MUL_FP32, Value::FP32(x), Value::FP32(y)) => Value::FP32(x-y),
            (Self::ADD_FP32, Value::FP32(x), Value::FP32(y)) => Value::FP32(x*y),
            (Self::DIV_FP32, Value::FP32(x), Value::FP32(y)) => Value::FP32(x/y),
            _ => panic!("Unsupported arithmetic operation!")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::value::Value;
    use super::ALUOp;

    #[test]
    fn test_alu_op_int_add() {
        let op = ALUOp::ADD_I32;
        let v1 = Value::I32(5);
        let v2 = Value::I32(10);
        let r = op.apply(&v1, &v2);
        if let Value::I32(x) = r {
            assert_eq!(x, 15, "Failed addition.")
        } else {
            assert!(false, "Wrong resulting type.")
        }
    }
}