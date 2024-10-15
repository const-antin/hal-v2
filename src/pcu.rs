use dam::{channel::{ChannelElement, Receiver, Sender}, context::Context, dam_macros::context_macro, structures::Time, types::DAMType};

use crate::{alu::{ALUHwConfig, ALURtConfig}, pipeline_stage::{self, PipelineStage}, scalar::Scalar};

#[derive(Clone)]
pub struct HwConfig {
    pub alu_configs: Vec<ALUHwConfig>, // alu_configs[row][column]
    pub num_simd_lanes: usize,
    // pub num_registers_per_stage: usize,
    // pub num_scalar_inputs: usize,
    // pub num_scalar_outputs: usize,
    pub num_vector_input_ports: usize
}

#[derive(Clone)]
pub struct RtConfig {
    pub alu_configs: Vec<ALURtConfig>, // alu_configs[row][column]
}

pub struct PCURuntimeData {
    pipeline_stages: Vec<PipelineStage>,
    input: Vec<Receiver<PCUData>>,
    output: Vec<Sender<PCUData>>
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PCUData {
    pub data: Vec<Scalar>
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
    pub fn new(hw_cfg: HwConfig, rt_cfg: RtConfig, input: Vec<Receiver<PCUData>>, output: Vec<Sender<PCUData>>) -> PCU {
        PCU::verify_alu_ops(&hw_cfg.alu_configs, &rt_cfg.alu_configs);

        let rt_data = PCURuntimeData {
            pipeline_stages: rt_cfg.alu_configs.iter().map(
                |cfg| {PipelineStage::new(cfg.clone(), hw_cfg.num_simd_lanes, 1)}).collect(),
            input: input,
            output: output
            
        };

        let pcu = PCU {
            hw_config: hw_cfg,
            rt_config: rt_cfg,
            rt_data: rt_data,
            context_info: Default::default()
        };
        pcu.rt_data.input .iter().for_each(|i| i.attach_receiver(&pcu));
        pcu.rt_data.output.iter().for_each(|i| i.attach_sender(&pcu));
        pcu
    }

    fn verify_alu_ops(hw_alus: &Vec<ALUHwConfig>, rt_alus: &Vec<ALURtConfig>) -> () {
        assert_eq!(hw_alus.len(), rt_alus.len());
        for (hw_el, sw_el) in hw_alus.iter().zip(rt_alus.iter()) {
            assert!(hw_el.supported_ops.contains(&sw_el.op));
        }
    }

    fn iterate(&mut self, input: &Vec<Vec<Scalar>>, time: Time) -> Time {
        // Run a pipeline iteration.
        let (data_out, t_fin) = self.rt_data.pipeline_stages.iter_mut().fold((input, time),
        |(data, time), stage| {
            stage.iterate(data, time)
        });

        // Enqueue the outputs.
        let data_out = data_out.clone();
        self.rt_data.output
            .iter()
            .zip(data_out.iter())
            .filter(|(sender, data)| {data.len() > 0})
            .for_each(|(sender, data)| {
                sender.enqueue(&self.time, ChannelElement::new(t_fin, PCUData { data: data.clone() })).unwrap();
        });
        t_fin
    }

    /*
    fn iter_bubble(&mut self, time: Time) -> Time {
        let bubble = vec![Scalar::I32(0); self.hw_config.alu_configs[0].len()];
        self.iter(&bubble, time)
    }
     */

    fn fill_with_bubbles_until_now(&mut self, mut last_iter_time: Time) -> () {
        let input_time = self.time.tick();
        while last_iter_time < input_time {
            // self.iter_bubble(last_iter_time.clone());
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
            // TODO: Only get input from selected input channels. 

            // Dequeue from every ALU selected input:
            let selected_inputs = self.rt_config.alu_configs[0].get_input_regs();

            // Fill an input vector with all zeros except for the selected inputs.
            let mut input = 
                vec![
                    vec![Scalar::I32(0); self.hw_config.num_simd_lanes]; 
                self.hw_config.num_vector_input_ports];

            for i in selected_inputs {
                let next = self.rt_data.input[i].dequeue(&self.time);
                match next {
                    Ok(data) => {
                        input[i] = data.data.data;
                    }
                    Err(_) => {
                        return; // We currently close the pipeline when the instream is closed.
                    }
                }
            }
            self.iterate(&input, self.time.tick());
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
        const CHAN_SIZE: usize = 1;
        
        let hw_config = HwConfig {
            alu_configs: vec![ALUHwConfig {
                supported_ops: HashSet::from([ALUOp::ADD_I32, ALUOp::MUL_I32])
            };1],
            num_simd_lanes: 1,
            num_vector_input_ports: 2,
        };

        let rt_config = RtConfig {
            alu_configs: vec![
                ALURtConfig{op:ALUOp::ADD_I32,in_a:ALUInput::PREV(0),in_b:ALUInput::PREV(1), target: 0}
                ;1]
        };

        let (snd0, i0) = parent.bounded(CHAN_SIZE);
        let (snd1, i1) = parent.bounded(CHAN_SIZE);
        let (o0, rcv) = parent.bounded(CHAN_SIZE);

        let pcu = PCU::new(hw_config, rt_config, vec![i0, i1], vec![o0]);

        let snd0_gen = (0..10).map(|x| {
            PCUData{data: vec![Scalar::I32(x)]}
        });

        let snd1_gen = (0..10).map(|x| {
            PCUData{data: vec![Scalar::I32(2*x)]}
        });

        let rcv_gen = (0..10).map(|x| {
            PCUData{data: vec![
                Scalar::I32(3*x)
            ]}
        });

        let gen0 = GeneratorContext::new(|| {snd0_gen}, snd0);
        let gen1 = GeneratorContext::new(|| {snd1_gen}, snd1);
        let rcv = CheckerContext::new(|| {rcv_gen}, rcv);

        parent.add_child(gen0);
        parent.add_child(gen1);
        parent.add_child(rcv);
        parent.add_child(pcu);
        let executed = parent
            .initialize(InitializationOptionsBuilder::default().build().unwrap())
            .unwrap()
            .run(RunOptions::default());
        println!("dump_failures: {:?}", executed.dump_failures());
        assert!(executed.passed());
    }
}