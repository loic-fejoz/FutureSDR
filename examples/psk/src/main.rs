//! A simple BPSK encoder test
//!
use std::collections::VecDeque;

use clap::Parser;

use futuresdr::anyhow::Result;
use futuresdr::blocks::zeromq::PubSinkBuilder;
use futuresdr::blocks::Apply;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::ControlledOscillatorBuilder;
use futuresdr::blocks::FileSink;
use futuresdr::blocks::FiniteSource;
use futuresdr::blocks::Head;
use futuresdr::blocks::Throttle;
use futuresdr::macros::connect;
use futuresdr::num_complex::Complex32;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use futuredsp::windows;
use futuresdr::blocks::FirBuilder;
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

    // let samples_per_symbol: usize = (sample_rate) as usize / args.samples_per_baud;
    let samples_per_symbol: usize = sample_rate as usize;

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
        move |i: &bool| -> std::iter::Take<std::iter::Repeat<f32>> {
            std::iter::repeat(if *i {440.0} else {392.0} ).take(samples_per_symbol)
        },
    );
    // Generate BPSK
    let encoder = ControlledOscillatorBuilder::<f32, f32>::freq_co(
        sample_rate as f32,
        1.0,
        1.0,
    )
    .build();
    // Limit and regurlarly send via ZeroMQ
    //let throttle = Throttle::<Complex32>::new(sample_rate as f64);
    // let snk = PubSinkBuilder::<Complex32>::new()
    //     .address("tcp://127.0.0.1:50001")
    //     .build();
    let snk = AudioSink::new(sample_rate, 1);
    // let snk = FileSink::<f32>::new("/tmp/test.f32");

    // let n_taps = 38;
    // let alpha = 2.2;
    // let taps: Vec<f32> = windows::gaussian(n_taps, alpha).iter().map(|x| (*x) as f32).collect();
    // let filter = FirBuilder::new::<f32, f32, _, _>(taps);

    // Inefficient moving average:
    let mut acc = VecDeque::<f32>::new();
    let filter = Apply::new(move |i: &f32| -> f32 {
        acc.push_back(*i);
        if acc.len() > 10000 {
            acc.pop_front();
        }
        let sum: f32 = acc.iter().sum();
        sum / (acc.len() as f32)
    });


    let _head = Head::<f32>::new(10*sample_rate as u64);
    

    connect!(fg,
        src > differentiator > repeat_per_symbol > filter > encoder > snk
    );

    Runtime::new().run(fg)?;

    Ok(())
}
