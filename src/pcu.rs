use dam::{channel::{ChannelElement, Receiver, Sender}, context::Context, dam_macros::context_macro, structures::Time, types::DAMType};

use crate::{alu::{ALUHwConfig, ALURtConfig}, pipeline_stage::PipelineStage, scalar::Scalar};

pub struct HwConfig {
    pub alu_configs: Vec<Vec<ALUHwConfig>>, // alu_configs[row][column]
    pub number_chained_counters: usize
}

pub struct RtConfig {
    pub alu_configs: Vec<Vec<ALURtConfig>>, // alu_configs[row][column]
    pub counter_max_values: Vec<usize>
}

pub struct PCURuntimeData {
    pipeline_stages: Vec<PipelineStage>,
    input: Receiver<PCUData>,
    output: Sender<PCUData>
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PCUData {
    data: Vec<Scalar>
}

impl DAMType for PCUData {
    fn dam_size(&self) -> usize {
        std::mem::size_of_val(&self.data)
    }
}

#[context_macro]
pub struct PCU {
    pub hw_config: HwConfig,
    pub rt_config: RtConfig,
    rt_data: PCURuntimeData
}

impl PCU {
    fn new(hw_cfg: HwConfig, rt_cfg: RtConfig, input: Receiver<PCUData>, output: Sender<PCUData>) -> PCU {
        PCU::verify_alu_ops(&hw_cfg.alu_configs, &rt_cfg.alu_configs);

        let rt_data = PCURuntimeData {
            pipeline_stages: rt_cfg.alu_configs.iter().map(
                |cfg| {PipelineStage::new(cfg.to_vec())}).collect(),
            input: input,
            output: output
            
        };

        let pcu = PCU {
            hw_config: hw_cfg,
            rt_config: rt_cfg,
            rt_data: rt_data,
            context_info: Default::default()
        };
        pcu.rt_data.input.attach_receiver(&pcu);
        pcu.rt_data.output.attach_sender(&pcu);
        pcu
    }

    fn verify_alu_ops(hw_alus: &Vec<Vec<ALUHwConfig>>, rt_alus: &Vec<Vec<ALURtConfig>>) -> () {
        assert_eq!(hw_alus.len(), rt_alus.len());
        for (hw_col, rt_col) in hw_alus.iter().zip(rt_alus.iter()) {
            assert_eq!(hw_col.len(), rt_col.len());
            for (hw_el, sw_el) in hw_col.iter().zip(rt_col.iter()) {
                assert!(hw_el.supported_ops.contains(&sw_el.op));
            }
        }
    }

    fn iter(&mut self, input: &Vec<Scalar>, time: Time) -> Time {
        // Run a pipeline iteration.
        let (data_out, t_fin) = self.rt_data.pipeline_stages.iter_mut().fold((input, time),
        |(data, time), stage| {
            stage.iter(data, time)
        });
        let data_out = data_out.clone();
        self.rt_data.output.enqueue(&self.time,  // TODO: Double-Check this. There might be a possible bug here. 
            ChannelElement::new(t_fin, PCUData { data: data_out })).unwrap();
        t_fin
    }

    fn iter_bubble(&mut self, time: Time) -> Time {
        let bubble = vec![Scalar::I32(0); self.rt_data.pipeline_stages.len()];
        self.iter(&bubble, time)
    }

    fn fill_with_bubbles_until_now(&mut self, mut last_iter_time: Time) -> () {
        let input_time = self.time.tick();
        while last_iter_time < input_time {
            self.iter_bubble(last_iter_time.clone());
            last_iter_time += 1;
        }
    }
}

impl Context for PCU {
    fn init(&mut self) {
    }

    fn run(&mut self) {
        loop {
            let old_time = self.time.tick();
            // Get next input, insert bubbles if there was a delay.
            let next = self.rt_data.input.dequeue(&self.time);
            match next {
                Ok(data) => {
                    let input = data.data.data;
                    self.fill_with_bubbles_until_now(old_time);
                    self.iter(&input, self.time.tick());
                    self.time.incr_cycles(1);
                }
                Err(_) => {
                    return; // We currently close the pipeline when the instream is closed.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use dam::{simulation::{InitializationOptionsBuilder, ProgramBuilder, RunOptions}, utility_contexts::{CheckerContext, GeneratorContext}};

    use crate::{alu::{ALUHwConfig, ALUInput, ALUOp, ALURtConfig}, pcu::PCUData, scalar::Scalar};

    use super::{HwConfig, RtConfig, PCU};

    #[test]
    fn simple_pcu_test() {
        let mut parent = ProgramBuilder::default();
        const CHAN_SIZE: usize = 8;
        
        let hw_config = HwConfig {
            alu_configs: vec![vec![ALUHwConfig {
                supported_ops: HashSet::from([ALUOp::ADD_I32, ALUOp::MUL_I32])
            };2];2],
            number_chained_counters: 0,
        };

        let rt_config = RtConfig {
            alu_configs: vec![vec![
                ALURtConfig{op: ALUOp::ADD_I32, in_a: ALUInput::PREV, in_b: ALUInput::PREV_BELOW}, 
                ALURtConfig{op: ALUOp::MUL_I32, in_a: ALUInput::NEXT, in_b: ALUInput::PREV}]
                ; 2],
            counter_max_values: vec![],
        };

        let (snd, i0) = parent.bounded(CHAN_SIZE);
        let (o0, rcv) = parent.bounded(CHAN_SIZE);

        let pcu = PCU::new(hw_config, rt_config, i0, o0);

        let snd_gen = (0..10).map(|x| {
            PCUData{data: vec![
                Scalar::I32(x), Scalar::I32(2*x)
                ]}
        });

        let rcv_gen = std::iter::once(0).chain(0..10).map(|x| {
            PCUData{data: vec![
                Scalar::I32(3*x), Scalar::I32(0)
            ]}
        });

        let gen = GeneratorContext::new(|| {snd_gen}, snd);
        let rcv = CheckerContext::new(|| {rcv_gen}, rcv);

        parent.add_child(gen);
        parent.add_child(rcv);
        parent.add_child(pcu);
        let executed = parent
            .initialize(InitializationOptionsBuilder::default().build().unwrap())
            .unwrap()
            .run(RunOptions::default());
        assert!(executed.passed());
    }
}