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


/// This function converts a 32bits float into 4 bytes
/// as per the native platform.
/// You might need to move to `to_le_bytes` if you are going cross-platform.
pub fn f32_u8_serialize(f: &f32) -> Vec<u8> {
    let bytes = f.to_ne_bytes();
    bytes.to_vec()
}
fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let src = fg.add_block(Oscillator::new(440.0, 1.0)); // Generates a 440Hz signal at 48kHz
    let throttle = fg.add_block(Throttle::<f32>::new(48_000.0)); // Limit rate at 48kHZ
    let serializer = fg.add_block(ApplyIntoIter::<f32, Vec<u8>>::new(&f32_u8_serialize)); // Convert stream of float into stream of u8
    let snk = fg.add_block( // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(1)
            .address("tcp://127.0.0.1:50001")
            .build(),
    );

    fg.connect_stream(src, "out", throttle, "in")?;
    fg.connect_stream(throttle, "out", serializer, "in")?;
    fg.connect_stream_with_type(
        serializer, "out",
        snk, "in",
        Slab::with_size(4*1024) // Because GNU Radio is expecting exactly 4 bytes per message we force the buffer to be a multiple of 4.
    )?;

    Runtime::new().run(fg)?;

    Ok(())
}
