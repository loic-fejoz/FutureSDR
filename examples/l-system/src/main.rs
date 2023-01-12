//! A simple BPSK encoder test
use clap::Parser;

use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::blocks::ConsoleSink;
use futuresdr::blocks::ControlledOscillatorBuilder;
// use futuresdr::blocks::FileSink;
use futuresdr::blocks::FiniteSource;
use futuresdr::blocks::Head;
use futuresdr::blocks::NullSink;
use futuresdr::blocks::Selector;
use futuresdr::blocks::SelectorDropPolicy;
use futuresdr::blocks::SelectorFinishPolicy;
use futuresdr::blocks::Throttle;
use futuresdr::macros::connect;
// use futuresdr::num_complex::Complex32;
use futuresdr::runtime::scheduler::FlowScheduler;
use futuresdr::runtime::scheduler::TpbScheduler;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;

#[derive(Parser, Debug)]
struct Args {
    /// Center frequency
    #[clap(short, long, default_value_t = 2000.0)]
    frequency: f64,

    /// Sample rate
    #[clap(short, long, default_value_t = 48000.0)]
    rate: f64,

    /// Samples per baud
    #[clap(short, long, default_value_t = 4)]
    samples_per_baud: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Alphabet {
    A,
    B,
    C,
    D,
}

fn l_system(i: Alphabet) -> impl Iterator<Item = Alphabet> + Send {
    let next = match i {
        Alphabet::A => vec![Alphabet::A, Alphabet::B],
        Alphabet::B => vec![Alphabet::B, Alphabet::D],
        Alphabet::C => vec![Alphabet::D, Alphabet::C],
        Alphabet::D => vec![Alphabet::D, Alphabet::A, Alphabet::B],
    };
    next.into_iter()
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Configuration {args:?}");

    let sample_rate = args.rate as u32;
    println!("Sample rate {sample_rate:?}");

    // one music note last 1s
    let samples_per_symbol: usize = sample_rate as usize;

    // Create the `Flowgraph` where the `Block`s will be added later on
    let mut fg = Flowgraph::new();

    // axiom, aka initial value
    let mut v = vec![Alphabet::C, Alphabet::A].into_iter();
    let src = FiniteSource::new(move || v.next());

    // Merge axiom and generated stream
    let selector = Selector::<Alphabet, 2, 1>::new(
        SelectorDropPolicy::NoDrop,
        SelectorFinishPolicy::MoveToNext,
    );

    // L-System, see for instance
    // * http://docsdrive.com/pdfs/medwelljournals/jeasci/2020/3080-3082.pdf
    // * https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=91efd85dec118e819943457d4b53d278ebe28f65
    let grammar = ApplyIntoIter::new(move |i: &Alphabet| l_system(*i));

    // Stop after some generations
    let head = Head::<Alphabet>::new(100);

    // Display
    let snk = ConsoleSink::<Alphabet>::new("\n");

    // Duplicate symbol to get as many samples as needed per symbols
    // And convert alphabet into frequency
    let repeat_per_symbol = ApplyIntoIter::new(
        move |i: &Alphabet| -> std::iter::Take<std::iter::Repeat<f32>> {
            print!("{i:?}\n");
            std::iter::repeat(match *i {
                Alphabet::C => 261.63,
                Alphabet::D => 293.66,
                Alphabet::A => 440.0,
                Alphabet::B => 493.88,
                _ => 0.0,
            })
            .take(samples_per_symbol)
        },
    );
    // Generate sound
    let encoder =
        ControlledOscillatorBuilder::<f32, f32>::freq_co(sample_rate as f32, 1.0, 1.0).build();

    let throttle = Throttle::<f32>::new(sample_rate as f64);

    let audio = AudioSink::new(sample_rate, 1);
    //let audio = NullSink::<f32>::new();

    connect!(fg,
        src > selector.in0;
        selector.out0 > grammar > snk;
        grammar > selector.in1;
        grammar > repeat_per_symbol > encoder > throttle > audio;
    );

    Runtime::with_scheduler(TpbScheduler::new()).run(fg)?;

    Ok(())
}
