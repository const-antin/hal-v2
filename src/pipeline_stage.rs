// PipelineStage consisting out of ALU's and Registers
use dam::structures::Time;

use crate::{alu::{ALUInput, ALURtConfig}, scalar::Scalar};

pub struct PipelineStage {
    pub alu_config: ALURtConfig, 
    pub simd: usize,             // number of ALUs in the pipeline stage
    pub register_depth: usize,   // number of target registers for each ALU
    pub data: Vec<Vec<Scalar>>,  
}

impl PipelineStage {
    pub fn new(alu_config: ALURtConfig, simd: usize, register_depth: usize) -> PipelineStage {

        PipelineStage {
            alu_config: alu_config,
            data: vec![vec![Scalar::I32(0); register_depth]; simd],
            register_depth: register_depth,
            simd: simd,
        }
    }

    // Returns time after pipeline stage completion
    // The input is a Vec<Vec<Scalar>> because the PCU has multiple inputs. 
    // For the 2nd..nth pipeline stage, the outer Vec is always of length 1.

    // TODO: There are still many open questions here. For instance, we currently move values in the pipeline
    // that are actually computed on by the ALU. We could, however, also move every value in the pipeline
    // regardless of if it is computed on or not. The Plasticine paper does not describe how this is done. 
    pub fn iterate(&mut self, prev_stage: &Vec<Vec<Scalar>>, time: Time) -> (&Vec<Vec<Scalar>>, Time) {
        let mut next_data = vec![vec![Scalar::I32(0); self.data.len()]];

        for idx in 0..self.simd {

            let lhs = self.get_input(&self.alu_config.in_a, prev_stage, idx);
            let rhs = self.get_input(&self.alu_config.in_b, prev_stage, idx);

            next_data[idx][self.alu_config.target] = self.alu_config.op.apply(&lhs, &rhs)
        }
        self.data = next_data;
        (&self.data, time + self.alu_config.op.delay() as u64)
    }

    fn get_input(&self, alu_input: &ALUInput, prev_stage: &Vec<Vec<Scalar>>, idx: usize) -> Scalar {
        match alu_input {
            ALUInput::NEXT(register_sel) => self.data
                .get(*register_sel).expect("Error: Selected Pipeline Register Set does not exist.")
                .get(idx).expect("Error: ALU Input NEXT({register_sel}) does not exist.")
                .clone(),
            ALUInput::PREV(register_sel) => prev_stage
                .get(*register_sel).expect("Error: Selected Pipeline Register Set does not exist.")
                .get(idx).expect("Error: ALU Input NEXT({register_sel}) does not exist.")
                .clone(),
            ALUInput::PREV_BELOW(register_sel) => prev_stage[*register_sel].get(idx+1)
                .expect(r#"Selected ALU Input "PREV_BELOW" does not exist."#).clone(),
            ALUInput::CONSTANT(x) => x.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use dam::structures::Time;

    use crate::scalar::Scalar;
    use crate::alu::{ALUInput, ALUOp, ALURtConfig};
    use super::PipelineStage;

    fn prepare() -> PipelineStage {
        let alu_rt_config_1 = ALURtConfig {
            op: ALUOp::ADD_I32,
            in_a: ALUInput::PREV(0),
            in_b: ALUInput::NEXT(0),
            target: 0
        };
        PipelineStage::new(alu_rt_config_1, 1, 1)
    }
    
    #[test]
    fn pipeline_holds_state_test() {
        let t_0 = Time::new(0);
        let mut pl = prepare();
        let input = vec![vec![Scalar::I32(1)]];

        let (_, t_1) = pl.iterate(&input, t_0);
        assert_eq!(pl.data[0][0], Scalar::I32(1));
        let _ = pl.iterate(&input, t_1);
        assert_eq!(pl.data[0][0], Scalar::I32(2));
    }

}