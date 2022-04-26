use futuresdr::anyhow::Result;
use futuresdr::blocks::zeromq::PubSinkBuilder;
use futuresdr::blocks::Head;
use futuresdr::blocks::NullSource;
use futuresdr::blocks::audio::Oscillator;
use futuresdr::blocks::Throttle;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::runtime::buffer::slab::Slab;

fn main() -> Result<()> {

    let mut fg = Flowgraph::new();

    let src = fg.add_block(Oscillator::new(440.0, 1.0)); // Generates a 440Hz signal at 48kHz
    let throttle = fg.add_block(Throttle::<f32>::new(48_000.0)); // Limit rate at 48kHZ
    let snk = fg.add_block( // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(std::mem::size_of::<f32>())
            .address("tcp://127.0.0.1:50001")
            .build(),
    );

    fg.connect_stream(src, "out", throttle, "in")?;
    fg.connect_stream(throttle, "out", snk, "in")?;


    Runtime::new().run(fg)?;

    Ok(())
}
