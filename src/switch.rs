use std::collections::HashMap;

use dam::{channel::{ChannelElement, PeekResult, Receiver, Sender}, context::Context, dam_macros::context_macro, structures::{ContextInfo, Time}};

use crate::pcu::PCUData;

pub enum SwitchMode {
    SingleEnqueueSingleDequeue, // 1. Dequeue and enqueue exactly one element per clock cycle.
    MultiEnqueueSingleDequeue,  // 2. Dequeue one element per clock cycle, enqueue it to all selected outputs.
    MultiEnqueueMultiDequeue    // 3. Dequeue from all inputs, enqueue to all outputs.
}

pub struct HwConfig {
    pub simd: usize, 
    pub datatype_width: usize, 
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub mode: SwitchMode
    // todo: Add parameterizable routing restructions? 
}

pub struct RtConfig {
    pub routing_table: HashMap<usize, Vec<usize>> // routing_table[in] -> out
}

pub struct RtData {
    receivers: Vec<Receiver<PCUData>>,
    senders: Vec<Sender<PCUData>>
}

#[context_macro]
pub struct Switch {
    hw_config: HwConfig,
    rt_config: RtConfig,
    rt_data: RtData
}

// Cycle-accurate, has backpressure
impl Switch {
    pub fn new(hw_config: HwConfig, rt_config: RtConfig, receivers: Vec<Receiver<PCUData>>, senders: Vec<Sender<PCUData>>) -> Switch {

        assert_eq!(hw_config.num_inputs, receivers.len());
        assert_eq!(hw_config.num_outputs, senders.len());

        let switch = Switch { 
            hw_config: hw_config, 
            rt_config: rt_config, 
            rt_data: RtData { 
                receivers: receivers,
                senders: senders
            }, 
            context_info: ContextInfo::default() 
        };

        switch.rt_data.receivers.iter().for_each(|r| {r.attach_receiver(&switch);});
        switch.rt_data.senders.iter().for_each(|s| {s.attach_sender(&switch);});
        switch
    }

    fn get_first_available_receiver_inputs(&self) -> Vec<usize>{
        // TODO: implement additional behavior: if all channels are empty at the current timestamp, advance time and continue.
        let minimal_input;
        loop {
            let mut peek_results = self.rt_data.receivers.iter()
                .enumerate()
                .map(|(i, r)|(i, r.peek()))
                .filter(|(_, x)| !matches!(x, PeekResult::Closed))
                .peekable();

            if peek_results.peek().is_none() {
                return vec![];
            }

            let timing_of_inputs = peek_results.map(|(r, x)| match x {
                PeekResult::Something(ChannelElement { time: t, data: _ }) => (r, x, t),
                PeekResult::Nothing(t) => (r, x, t),
                _ => panic!("This should always be something.")
            });

            let (_, first_channels) = timing_of_inputs.fold((Time::infinite(), Vec::new()), |(min, mut vec), el| {
                let (r, pr, t) = el;
                if t < min {
                    vec.clear();
                    vec.push((r, pr));
                    (t.clone(), vec)
                } else if t == min {
                    vec.push((r, pr));
                    (min, vec)
                } else {
                    (min, vec)
                }
            });

            let avail_channels: Vec<_> = first_channels
                .iter()
                .filter(|(_, x)| matches!(x, PeekResult::Something(_)))
                .map(|(x, _)| {*x})
                .collect();

            if !avail_channels.is_empty() {
                minimal_input = avail_channels;
                break;
            } else if first_channels.len() == self.hw_config.num_inputs {
                // All channels have no data at the current clock cycle. Advance clock. 
                // This is needed because DAM's peek() caches the last result.
                self.time.incr_cycles(1);
            }
        }
        return minimal_input
    }

    fn single_deque(&self, multi_enqueue: bool) -> Result<(), &str> {
        let rdy_idx = *self.get_first_available_receiver_inputs().split_first().ok_or("All inputs closed.")?.0;
        let input = self.rt_data.receivers[rdy_idx].dequeue(&self.time).unwrap();
        let data = input.data;
        
        let targets = self.rt_config.routing_table.get(&rdy_idx).expect("Received data from unrouted input!");
        for o_idx in targets {
            let target = &self.rt_data.senders[*o_idx];
            target.enqueue(&self.time, ChannelElement::new(self.time.tick() /* todo: add routing delay here */, data.clone())).unwrap();
            if multi_enqueue {
                self.time.incr_cycles(1);
            }
        }

        if !multi_enqueue {
            self.time.incr_cycles(1);
        }
        Ok(())
    }

    fn single_dequeue_single_enqueue_iter(&self) -> Result<(), &str> {
        self.single_deque(false)
    }

    fn single_dequeue_multi_enqueue_iter(&self) -> Result<(), &str> {
        self.single_deque(true)
    }

    fn multi_dequeue_multi_enqueue_iter(&self) -> Result<(), &str> {
        todo!(); // What, if two inputs route to the same output? Then we must serialize between them.
    }
}

impl Context for Switch {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            // Okay, so we'll have to discuss how exactly we're going to structure our interconnect. 
            // There are several possibilities...
            let res;
            match self.hw_config.mode {
                SwitchMode::SingleEnqueueSingleDequeue => res = self.single_dequeue_single_enqueue_iter(),
                SwitchMode::MultiEnqueueSingleDequeue => res = self.single_dequeue_multi_enqueue_iter(),
                SwitchMode::MultiEnqueueMultiDequeue => res = self.multi_dequeue_multi_enqueue_iter()
            }
            if let Err(_) = res {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use dam::{simulation::{InitializationOptionsBuilder, ProgramBuilder, RunOptions}, utility_contexts::{CheckerContext, GeneratorContext}};

    use crate::{pcu::PCUData, scalar::Scalar, switch::{Switch, SwitchMode}};

    use super::{HwConfig, RtConfig};

    #[test]
    fn test_passthrough() {
        let mut parent = ProgramBuilder::default();
        const CHAN_SIZE: usize = 8;

        let (snd, input) = parent.bounded(CHAN_SIZE);
        let (output, rcv) = parent.bounded(CHAN_SIZE);

        let map: HashMap<_, _> = [(0, vec![0])].into_iter().collect();

        let switch = Switch::new(
            HwConfig {
                simd: 1,
                datatype_width: Scalar::I32(0).width(),
                num_inputs: 1,
                num_outputs: 1,
                mode: SwitchMode::SingleEnqueueSingleDequeue
            }, 
            RtConfig {
                routing_table: map,
            },
            vec![input],
            vec![output]
        );
        
        let gen = GeneratorContext::new(
            || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), snd);
        let rcv = CheckerContext::new(
             || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), rcv);

        parent.add_child(gen);
        parent.add_child(rcv);
        parent.add_child(switch);
        let executed = parent
            .initialize(InitializationOptionsBuilder::default().build().unwrap())
            .unwrap()
            .run(RunOptions::default());
        assert!(executed.passed());
    }

    #[test]
    fn test_route() {
        let mut parent = ProgramBuilder::default();
        let CHAN_SIZE: usize = 8;

        let (inputs_snd0, inputs_rcv0) = parent.bounded(CHAN_SIZE);
        let (inputs_snd1, inputs_rcv1) = parent.bounded(CHAN_SIZE);
        
        let (outputs_snd0, outputs_rcv0) = parent.bounded(CHAN_SIZE);
        let (outputs_snd1, outputs_rcv1) = parent.bounded(CHAN_SIZE);

        let table: HashMap<_, _> = [(0,vec![1])].into_iter().collect(); // TODO: Should this thing be able to route multiple things at once? 
        
        let switch = Switch::new(
            HwConfig {
                simd: 1,
                datatype_width: Scalar::I32(0).width(),
                num_inputs: 2,
                num_outputs: 2,
                mode: SwitchMode::SingleEnqueueSingleDequeue
            }, 
            RtConfig {
                routing_table: table,
            },
            vec![inputs_rcv0, inputs_rcv1],
            vec![outputs_snd0, outputs_snd1]
        );
        
        let gen0 = GeneratorContext::new(
            || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), inputs_snd0);
        let gen1 = GeneratorContext::new(|| std::iter::empty(), inputs_snd1);
        let rcv0 = CheckerContext::new(|| std::iter::empty(), outputs_rcv0);
        let rcv1 = CheckerContext::new(
            || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), outputs_rcv1);

        parent.add_child(gen0);
        parent.add_child(gen1);
        parent.add_child(rcv0);
        parent.add_child(rcv1);
        parent.add_child(switch);
        let executed = parent
            .initialize(InitializationOptionsBuilder::default().build().unwrap())
            .unwrap()
            .run(RunOptions::default());
        assert!(executed.passed());
    }

    #[test]
    fn test_broadcast() {
        let mut parent = ProgramBuilder::default();
        let CHAN_SIZE: usize = 8;

        let (snd, input) = parent.bounded(CHAN_SIZE);
        let (output0, rcv0) = parent.bounded(CHAN_SIZE);
        let (output1, rcv1) = parent.bounded(CHAN_SIZE);

        let table: HashMap<_, _> = [(0,vec![0,1])].into_iter().collect(); // TODO: Should this thing be able to route multiple things at once? 

        let switch = Switch::new(
            HwConfig {
                simd: 1,
                datatype_width: Scalar::I32(0).width(),
                num_inputs: 1,
                num_outputs: 2,
                mode: SwitchMode::SingleEnqueueSingleDequeue
            }, 
            RtConfig {
                routing_table: table,
            },
            vec![input],
            vec![output0, output1]
        );
        
        let gen = GeneratorContext::new( 
            // TODO: There is something weird happening here: After _some_ runs, the Generator just does not generate. 
            // Seemingly, it never is scheduled or something? peek() always just returns time(0).
            || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), snd);

        let rcv0 = CheckerContext::new(
            || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), rcv0);
        let rcv1 = CheckerContext::new(
            || {0..10}.map(|x| PCUData {data: vec![Scalar::I32(x)]}), rcv1);

        parent.add_child(gen);
        parent.add_child(rcv0);
        parent.add_child(rcv1);
        parent.add_child(switch);
        let executed = parent
            .initialize(InitializationOptionsBuilder::default().build().unwrap())
            .unwrap()
            .run(RunOptions::default());
        executed.dump_failures();
        assert!(executed.passed());
    }
}