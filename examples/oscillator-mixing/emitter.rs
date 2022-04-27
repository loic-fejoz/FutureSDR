use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::Oscillator;
use futuresdr::blocks::zeromq::PubSinkBuilder;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::blocks::Combine;
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
    let interpolation1 = 480;
    let decimation1 = 441;

    let interpolation2 = decimation1;
    let decimation2 = interpolation1;

    let mut fg = Flowgraph::new();

    let src = fg.add_block(Oscillator::new(440.0, 1.0)); // Generates a 440Hz signal at 48kHz
    let throttle = fg.add_block(Throttle::<f32>::new(48_000.0)); // Limit rate at 48kHZ
    let snk1 = fg.add_block(
        // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(std::mem::size_of::<f32>())
            .address("tcp://127.0.0.1:50001")
            .build(),
    );
    let snk2 = fg.add_block(
        // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(std::mem::size_of::<f32>())
            .address("tcp://127.0.0.1:50002")
            .build(),
    );
    let snk3 = fg.add_block(
        // Finally publish this using ZeroMQ protocol
        PubSinkBuilder::new(std::mem::size_of::<f32>())
            .address("tcp://127.0.0.1:50003")
            .build(),
    );

    // Linear interpolation and keep one out of <decimation> samples.
    let mut counter: usize = 0;
    let mut previous = 0.0;
    let rational_downsampler = fg.add_block(ApplyIntoIter::new(move |current: &f32| -> Vec<f32> {
        let mut vec = Vec::<f32>::with_capacity(interpolation1);
        for i in 0..interpolation1 {
            if counter == 0 {
                vec.push(previous + (i as f32) * (current - previous) / (interpolation1 as f32));
            }
            counter = (counter + 1) % decimation1;
        }
        previous = *current;
        vec
    }));

    // Linear interpolation and keep one out of <decimation> samples.
    let mut counter2: usize = 0;
    let mut previous2 = 0.0;
    let rational_upsampler = fg.add_block(ApplyIntoIter::new(move |current: &f32| -> Vec<f32> {
        let mut vec = Vec::<f32>::with_capacity(interpolation2);
        for i in 0..interpolation2 {
            if counter2 == 0 {
                vec.push(previous2 + (i as f32) * (current - previous2) / (interpolation2 as f32));
            }
            counter2 = (counter2 + 1) % decimation2;
        }
        previous2 = *current;
        vec
    }));

    let mut counter3: usize = 0;
    let mut acc = 0.0;
    let average_differentiator = fg.add_block(Combine::<f32, f32, f32>::new(move |a, b| {
            acc +=  (a - b).abs() / ((a+b)/2.0).abs();
            counter3 += 1;
            if (counter3 > 10*48_000) {
                counter3 = 0;
                acc = 0.0;
            }
            acc / (counter3 as f32)
    }));

    fg.connect_stream(src, "out", throttle, "in")?;
    fg.connect_stream(throttle, "out", snk1, "in")?;
    fg.connect_stream(throttle, "out", rational_downsampler, "in")?;
    fg.connect_stream(throttle, "out", average_differentiator, "in0")?;
    fg.connect_stream(rational_downsampler, "out", rational_upsampler, "in")?;
    fg.connect_stream(rational_upsampler, "out", snk2, "in")?;
    fg.connect_stream(rational_upsampler, "out", average_differentiator, "in1")?;
    fg.connect_stream(average_differentiator, "out", snk3, "in")?;


    Runtime::new().run(fg)?;

    Ok(())
}
