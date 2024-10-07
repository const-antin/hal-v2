// PipelineStage consisting out of ALU's and Registers
use dam::structures::Time;

use crate::{alu::{ALUInput, ALURtConfig}, scalar::Scalar};

pub struct PipelineStage {
    pub alu_configs: Vec<ALURtConfig>,
    pub data: Vec<Scalar>,
    pub delay: usize
}

impl PipelineStage {
    pub fn new(alu_configs: Vec<ALURtConfig>) -> PipelineStage {
        let len = alu_configs.len();
        let delay = alu_configs
            .iter()
            .map(|c| c.op.delay())
            .chain(std::iter::once(0))
            .max()
            .unwrap();

        PipelineStage {
            alu_configs: alu_configs,
            data: vec![Scalar::I32(0); len],
            delay: delay
        }
    }

    // Returns time after pipeline stage completion
    pub fn iter(&mut self, prev_stage: &Vec<Scalar>, time: Time) -> (&Vec<Scalar>, Time) {
        assert_eq!(prev_stage.len(), self.data.len());

        let mut next_data = vec![Scalar::I32(0); self.data.len()];

        for (idx, alu_config) in 
                                                self.alu_configs.iter()
                                                .enumerate() {

            let lhs = self.get_input(&alu_config.in_a, prev_stage, idx);
            let rhs = self.get_input(&alu_config.in_b, prev_stage, idx);

            next_data[idx] = alu_config.op.apply(&lhs, &rhs)
        }
        self.data = next_data;
        (&self.data, time + self.delay as u64)
    }

    fn get_input(&self, alu_input: &ALUInput, prev_stage: &Vec<Scalar>, idx: usize) -> Scalar {
        match alu_input {
            ALUInput::NEXT => self.data[idx].clone(),
            ALUInput::PREV => prev_stage[idx].clone(),
            ALUInput::PREV_BELOW => {
                assert!(idx + 1 < prev_stage.len(), r#"Selected ALU Input "PREV_BELOW" does not exist."#);
                prev_stage[idx+1].clone()
            }
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
            op: ALUOp::MUL_I32,
            in_a: ALUInput::PREV,
            in_b: ALUInput::PREV_BELOW
        };

        let alu_rt_config_2 = ALURtConfig {
            op: ALUOp::ADD_I32,
            in_a: ALUInput::PREV,
            in_b: ALUInput::NEXT
        };

        PipelineStage::new(vec![alu_rt_config_1, alu_rt_config_2])
    }

    #[test]
    fn correct_delay_test() {
        let t = Time::new(0);
        let mut pl = prepare();
        let input = vec![Scalar::I32(1), Scalar::I32(2)];

        let (_, t_d) = pl.iter(&input, t);
        assert_eq!(t_d.time(), pl.alu_configs.iter().map(|x| x.op.delay()).max().unwrap() as u64);
    }
    
    #[test]
    fn pipeline_holds_state_test() {
        let t_0 = Time::new(0);
        let mut pl = prepare();
        let input = vec![Scalar::I32(1), Scalar::I32(2)];

        let (_, t_1) = pl.iter(&input, t_0);
        let _ = pl.iter(&input, t_1);

        assert_eq!(pl.data[0], Scalar::I32(2));
        assert_eq!(pl.data[1], Scalar::I32(4));
    }

}