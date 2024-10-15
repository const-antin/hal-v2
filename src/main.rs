mod alu;
mod pcu; 
mod pmu; 
mod interconnect;
mod scalar;
mod pipeline_stage;
mod switch;

fn main() {
    /*
    let pcu = pcu::PCU {
        hw_config: pcu::HwConfig { },
        sw_config: pcu::SwConfig { }
    };
     */
    println!("Hello, world!");
}

mod tests {
    use std::collections::HashSet;

    use dam::{channel::Receiver, shim::channel::channel, simulation::{Executed, InitializationOptionsBuilder, ProgramBuilder, RunOptions}, utility_contexts::{CheckerContext, GeneratorContext, PrinterContext}};

    use crate::{alu::{ALUHwConfig, ALUInput, ALUOp, ALURtConfig}, pcu::{self, HwConfig, PCUData, PCU}, scalar::Scalar, switch};

    #[test]
    fn switch_and_pcu_test() {
        let mut parent = ProgramBuilder::default();

        let (in_send_0, in_0) = parent.bounded(1);
        let (in_send_1, in_1) = parent.bounded(1);
        let (in_send_2, in_2) = parent.bounded(1);
        let (in_send_3, in_3) = parent.bounded(1);

        let gen0 = GeneratorContext::new(
            || { 0..10}.map(|x| PCUData { data: vec![Scalar::I32(x)] }), in_send_0);
        let gen1 = GeneratorContext::new(
            || {10..20}.map(|x| PCUData { data: vec![Scalar::I32(x)] }), in_send_1);
        let gen2 = GeneratorContext::new(
            || {20..30}.map(|x| PCUData { data: vec![Scalar::I32(x)] }), in_send_2);
        let gen3 = GeneratorContext::new(
            || {30..40}.map(|x| PCUData { data: vec![Scalar::I32(x)] }), in_send_3);
    
        let (pcu_3_out, checker_in) = parent.bounded(1);

        // Out := a*b + c*d
        let checker = CheckerContext::new(|| 
            {(0..10).map(|i| PCUData{data: vec![
                Scalar::I32((i)*(10+i) + (20+i)*(30+i))
            ]})
            }, checker_in);

        parent.add_child(gen0); parent.add_child(gen1); parent.add_child(gen2); parent.add_child(gen3);
        parent.add_child(checker);

        let pcu_hw_config = pcu::HwConfig {
            alu_configs: vec![ALUHwConfig {
                supported_ops: HashSet::from([ALUOp::ADD_I32, ALUOp::MUL_I32])
            };1],
            num_vector_input_ports: 2,
            num_simd_lanes: 1
        };

        let pcu_rt_config_1 = pcu::RtConfig {
            alu_configs: vec![
                ALURtConfig {op:ALUOp::MUL_I32, in_a:ALUInput::PREV(0), in_b:ALUInput::PREV(1), target: 0 }
                ]
        };

        let pcu_rt_config_2 = pcu_rt_config_1.clone();

        let pcu_rt_config_3 = pcu::RtConfig {
            alu_configs: vec![
                ALURtConfig {op:ALUOp::ADD_I32, in_a:ALUInput::PREV(0), in_b:ALUInput::PREV(1), target: 0 }
                ]
        };


        let (pcu_1_out, switch_in_0) = parent.bounded(1);
        let (pcu_2_out, switch_in_1) = parent.bounded(1);
        let (switch_out_0, pcu_3_in_0) = parent.bounded(1);
        let (switch_out_1, pcu_3_in_1) = parent.bounded(1);

        let pcu_1 = pcu::PCU::new(pcu_hw_config.clone(), pcu_rt_config_1, vec![in_0, in_1], vec![pcu_1_out]);
        let pcu_2 = pcu::PCU::new(pcu_hw_config.clone(), pcu_rt_config_2, vec![in_2, in_3], vec![pcu_2_out]);
        let pcu_3 = pcu::PCU::new(pcu_hw_config.clone(), pcu_rt_config_3, vec![pcu_3_in_0, pcu_3_in_1], vec![pcu_3_out]);

        let switch_hw_config = switch::HwConfig {
            simd: 1,
            datatype_width: Scalar::I32(0).width(),
            num_inputs: 2,
            num_outputs: 2,
            mode: switch::SwitchMode::SingleEnqueueSingleDequeue
        };

        let switch_rt_config = switch::RtConfig {
            routing_table: [(0, vec![0]), (1, vec![1])].into_iter().collect()
        };

        let switch = switch::Switch::new(
            switch_hw_config,
            switch_rt_config,
            vec![switch_in_0, switch_in_1],
            vec![switch_out_0, switch_out_1]
        );

        parent.add_child(pcu_1); parent.add_child(pcu_2); parent.add_child(pcu_3); parent.add_child(switch);

        let executed = parent
            .initialize(InitializationOptionsBuilder::default().build().unwrap())
            .unwrap()
            .run(RunOptions::default());

        println!("dump_failures: {:?}", executed.dump_failures());
        assert!(executed.passed());
    }
}