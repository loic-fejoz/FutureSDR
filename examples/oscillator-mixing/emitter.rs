use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::Oscillator;
use futuresdr::blocks::zeromq::PubSinkBuilder;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::blocks::Head;
use futuresdr::blocks::NullSource;
use futuresdr::blocks::Throttle;
use futuresdr::runtime::buffer::slab::Slab;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;

fn main() -> Result<()> {
    // The original mp3 is sampled at 44.1kHz while we force the audio output to be 48kHz
    // thus we upsample by 1:480
    // and downsample by 441:1
    // NB: Obviously some filters should have been used to avoid some artifacts.
    // Overall it thus converts a 44.1kHz stream into a 48kHz one.
    let interpolation = 480;
    let decimation = 441;

    let mut fg = Flowgraph::new();

    let src = fg.add_block(Oscillator::new(440.0, 1.0)); // Generates a 440Hz signal at 48kHz
    let throttle1 = fg.add_block(Throttle::<f32>::new(48_000.0)); // Limit rate at 48kHZ
    let snk1 = fg.add_block(
        // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(std::mem::size_of::<f32>())
            .address("tcp://127.0.0.1:50001")
            .build(),
    );
    let throttle2 = fg.add_block(Throttle::<f32>::new(44_100.0)); // Limit rate at 44.1kHZ
    let snk2 = fg.add_block(
        // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(std::mem::size_of::<f32>())
            .address("tcp://127.0.0.1:50002")
            .build(),
    );

    // Linear interpolation and keep one out of <decimation> samples.
    let mut counter: usize = 0;
    let mut previous = 0.0;
    let rational_resampler = fg.add_block(ApplyIntoIter::new(move |current: &f32| -> Vec<f32> {
        let mut vec = Vec::<f32>::with_capacity(interpolation);
        for i in 0..interpolation {
            if counter == 0 {
                vec.push(previous + (i as f32) * (current - previous) / (interpolation as f32));
            }
            counter = (counter + 1) % decimation;
        }
        previous = *current;
        vec
    }));

    fg.connect_stream(src, "out", throttle1, "in")?;
    fg.connect_stream(throttle1, "out", snk1, "in")?;
    fg.connect_stream(src, "out", rational_resampler, "in")?;
    fg.connect_stream(rational_resampler, "out", throttle2, "in")?;
    fg.connect_stream(throttle2, "out", snk2, "in")?;

    Runtime::new().run(fg)?;

    Ok(())
}
