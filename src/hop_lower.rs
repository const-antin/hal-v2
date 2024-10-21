use dam::{channel::ChannelID, types::DAMType};
use hop::hop::{function::Function, program_graph::ProgramGraph};
use hop::hop::program_graph::Node;
use hop::primitives::elem::Elem;

struct Lowered {}

impl Lowered {
    fn new() -> Self {
        Lowered {}
    }

    fn lower_hop_to_hwsim<ST: DAMType>(&mut self, graph: &ProgramGraph<ST>) {
        graph.nodes.iter().for_each(|node| self.lower_hop_node(node));
    }
    
    fn lower_hop_node<ST: DAMType>(&mut self, node: &hop::hop::program_graph::Node<ST>) {
        match node {
            Node::Accum(input, output, fold, init, rank) => 
                self.lower_accum(input, output, fold, init, *rank),
            Node::Bufferize(input, output, rank) => 
                self.lower_bufferize(input, output, *rank),
            Node::Enumerate(input, output, rank) =>
                self.lower_enumerate(input, output, *rank),
            Node::FlatMap(input, fn_snd, fn_rcv, output, rank) =>
                self.lower_flat_map(input, fn_snd, fn_rcv, output, rank),
            Node::Flatten(input, output, flatten_dims) =>
                self.lower_flatten(input, output, flatten_dims),
            Node::FnBlock(input, output, func, rank) =>
                self.lower_fn_block(input, output, func, *rank),
            Node::Map(input, output, func) =>
                self.lower_map(input, output, func),
            Node::Partition(input, select, output, rank) =>
                self.lower_partition(input, select, output, *rank),
            Node::Promote(input, output, rank) =>
                self.lower_promote(input, output, rank),
            Node::Reassemble(inputs, select, output, rank) =>
                self.lower_reassemble(inputs, select, output, rank),
            Node::Repeat(input, count, output) =>
                self.lower_repeat(input, count, output),
            Node::Reshape(input, output, split_dims, chunk_sizes, pad_val) =>
                self.lower_reshape(input, output, split_dims, chunk_sizes, pad_val),
            Node::Scan(input, output, fold, init_accum, rank) =>
                self.lower_scan(input, output, fold, init_accum, *rank),
            Node::Unzip(in_stream, out_stream_1, out_stream_2) =>
                self.lower_unzip(in_stream, out_stream_1, out_stream_2),
            Node::Zip(in_stream_1, in_stream_2, out_stream) =>
                self.lower_zip(in_stream_1, in_stream_2, out_stream)
        }
    }
    fn lower_accum(&mut self, input: &ChannelID, output: &ChannelID, fold: &Function<i32>, init: &Function<i32>, rank: usize) {todo!()}
    
    fn lower_bufferize(&mut self, input: &ChannelID, output: &ChannelID, rank: usize) {todo!()}
    
    fn lower_enumerate(&mut self, input: &ChannelID, output: &ChannelID, rank: usize) {todo!()}
    
    fn lower_flat_map<ST>(&mut self, input: &ChannelID, fn_snd: &ChannelID, fn_rcv: &ChannelID, output: &ChannelID, rank: &ST) {todo!()}
    
    fn lower_flatten(&mut self, input: &ChannelID, output: &ChannelID, flatten_dims: &Vec<usize>) {todo!()}
    
    fn lower_fn_block(&mut self, input: &ChannelID, output: &ChannelID, func: &Function<i32>, rank: usize) {}
    
    fn lower_map(&mut self, input: &ChannelID, output: &ChannelID, func: &Function<i32>) {todo!()}
    
    fn lower_partition(&mut self, input: &ChannelID, select: &ChannelID, output: &Vec<ChannelID>, rank: usize) {todo!()}
    
    fn lower_promote<ST>(&mut self, input: &ChannelID, output: &ChannelID, rank: &ST) {todo!()}
    
    fn lower_reassemble<ST>(&mut self, input: &Vec<ChannelID>, select: &ChannelID, output: &ChannelID, rank: &ST) {todo!()}
    
    fn lower_repeat(&mut self, input: &ChannelID, count: &ChannelID, output: &ChannelID) {todo!()}
    
    fn lower_reshape<ST>(&mut self, input: &ChannelID, output: &ChannelID, split_dims: &Vec<ST>, chunk_sizes: &Vec<ST>, pad_val: &Option<Elem<i32, ST>>) {todo!()}
    
    fn lower_scan(&mut self, input: &ChannelID, output: &ChannelID, fold: &Function<i32>, init: &Function<i32>, rank: usize) {todo!()}
    
    fn lower_unzip(&mut self, input: &ChannelID, output_1: &ChannelID, output_2: &ChannelID) {todo!()}
    
    fn lower_zip(&mut self, input_1: &ChannelID, input_2: &ChannelID, output: &ChannelID) {todo!()}
    
}

#[cfg(test)]
mod tests {
    use dam::utility_contexts::{CheckerContext, GeneratorContext};
    use hop::primitives::elem::Elem;
    use dam::simulation::ProgramBuilder;
    use hop::hop::program_graph::ProgramGraph;
    use hop::hop::function::Function;

    #[test]
    fn hop_lower_test() {
        let mut ctx = ProgramBuilder::default();
    
        let (in_snd, in_rcv) = ctx.unbounded();
        let (out_snd, out_rcv) = ctx.unbounded();
    
        let mut pgm = ProgramGraph::new();
    
        ctx.add_child(GeneratorContext::new(
            || {
                (0..100024i32)
                    .map(|x| Elem::Val(x))
                    .chain(std::iter::once(Elem::Stop(1)))
            },
            in_snd,
        ));
        
        ctx.add_child(CheckerContext::new(
            || {
                (10..100034i32)
                    .map(|x| Elem::Val(x))
                    .chain(std::iter::once(Elem::Stop(1)))
            },
            out_rcv,
        ));
    
        let map_node = pgm.add_map_node(in_rcv, out_snd, Function::Add(Box::new(Function::Variable), Box::new(Function::Constant(10))));
        ctx.add_child(map_node); 
    
        /*
        ctx.initialize(Default::default())
            .unwrap()
            .run(Default::default());
         */
    
        dbg!(pgm.nodes);
    }
}