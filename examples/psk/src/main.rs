//! A simple BPSK encoder test
//!

use clap::Parser;

use futuresdr::anyhow::Result;
use futuresdr::blocks::zeromq::PubSinkBuilder;
use futuresdr::blocks::Apply;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::blocks::ControlledOscillatorBuilder;
use futuresdr::blocks::FiniteSource;
use futuresdr::blocks::Throttle;
use futuresdr::macros::connect;
use futuresdr::num_complex::Complex32;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use rand::Rng;

#[derive(Parser, Debug)]
struct Args {
    /// Center frequency
    #[clap(short, long, default_value_t = 2000.0)]
    frequency: f64,

    /// Sample rate
    #[clap(short, long, default_value_t = 32000.0)]
    rate: f64,

    /// Samples per baud
    #[clap(short, long, default_value_t = 4)]
    samples_per_baud: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Configuration {args:?}");

    let sample_rate = args.rate as u32;
    println!("Sample rate {sample_rate:?}");

    let samples_per_symbol: usize = (sample_rate) as usize / args.samples_per_baud;

    // Create the `Flowgraph` where the `Block`s will be added later on
    let mut fg = Flowgraph::new();

    // A random source of bits
    let src = FiniteSource::<_, bool>::new(move || {
        let mut rng = rand::thread_rng();
        let a_bit: bool = rng.gen();
        Some(a_bit)
    });

    // Differential encoding
    let mut diff_accumulator = false;
    let differentiator = Apply::new(move |i: &bool| -> bool {
        diff_accumulator = (*i) ^ diff_accumulator;
        diff_accumulator
    });

    // Duplicate symbol to get as many samples as needed per symbols
    let repeat_per_symbol = ApplyIntoIter::new(
        move |i: &bool| -> std::iter::Take<std::iter::Repeat<bool>> {
            std::iter::repeat(*i).take(samples_per_symbol)
        },
    );
    // Generate BPSK
    let bpsk_encoder = ControlledOscillatorBuilder::<bool, Complex32>::bpsk(
        sample_rate as f32,
        args.frequency as f32,
    )
    .build();
    // Limit and regurlarly send via ZeroMQ
    let throttle = Throttle::<Complex32>::new(sample_rate as f64);
    let snk = PubSinkBuilder::<Complex32>::new()
        .address("tcp://127.0.0.1:50001")
        .build();

    connect!(fg,
        src > differentiator > repeat_per_symbol > bpsk_encoder > throttle > snk
    );

    Runtime::new().run(fg)?;

    Ok(())
}
