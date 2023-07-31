use clap::Parser;
use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::*;
use futuresdr::blocks::Apply;
use futuresdr::blocks::ApplyNM;
use futuresdr::blocks::Combine;
use futuresdr::blocks::Delay;
use futuresdr::blocks::FirBuilder;
use futuresdr::blocks::FileSink;
use futuresdr::blocks::Split;
use futuresdr::futuredsp::firdes;
use futuresdr::futuredsp::windows::hamming;
use futuresdr::macros::connect;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use hound::{SampleFormat, WavSpec};
use num::Complex;
use std::f32::consts::TAU;
use std::path::Path;

#[derive(Clone, Debug)]
enum Mode {
    LSB,
    USB,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Mode::LSB => write!(f, "LSB"),
            Mode::USB => write!(f, "USB"),
        }
    }
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "LSB" => Ok(Mode::LSB),
            "USB" => Ok(Mode::USB),
            _ => Err("Not a valid mode".to_owned()),
        }
    }
}

#[derive(Parser)]
struct Cli {
    input: String,
    output: String,

    #[arg(short, long, default_value_t = Mode::LSB)]
    mode: Mode,

    #[clap(short, long, default_value_t = 50.3e3)]
    frequency: f32,

    #[clap(long, default_value_t = 256_000)]
    sample_rate: u32,

    #[clap(long, default_value_t = 3000.0)]
    audio_bandwidth: f32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut fg = Flowgraph::new();

    let source = FileSource::new(&cli.input);
    let src_kernel = source.kernel::<FileSource>().unwrap();
    assert!(
        src_kernel.channels() == 1,
        "Input audio must be mono but found {} channels",
        src_kernel.channels()
    );

    let audio_rate = src_kernel.sample_rate() as f64;
    let file_rate = cli.sample_rate;

    // Using a bandpass instead, can help to tame low frequencies bleeding
    // ouside of the chosen bandwidth.
    let taps = firdes::kaiser::lowpass(cli.audio_bandwidth / audio_rate, 350.0 / audio_rate, 0.05);
    let lowpass = FirBuilder::new::<f32, f32, _, _>(taps);

    let split = Split::new(move |v: &f32| (*v, *v));

    // Phase transformation by 90°.
    let window = hamming(167, false);
    let taps = firdes::hilbert(window.as_slice());
    let hilbert = FirBuilder::new::<f32, f32, _, _>(taps);

    // Match the delay caused by the phase transformation.
    let delay = Delay::<f32>::new(window.len() as isize / -2);

    let mode = cli.mode.clone();
    let to_complex = Combine::new(move |i: &f32, q: &f32| match mode {
        Mode::LSB => Complex::new(*i, *q * -1.0),
        Mode::USB => Complex::new(*i, *q),
    });

    let resampler = FirBuilder::new_resampling::<Complex<f32>, Complex<f32>>(file_rate as usize, audio_rate as usize);

    let mut osc = Complex::new(1.0, 0.0);
    let shift = Complex::from_polar(1.0, TAU * cli.frequency / file_rate as f32);
    let mixer = Apply::new(move |v: &Complex<f32>| {
        osc *= shift;
        v * osc
    });

    let to_i16_iq = ApplyNM::<_, _, _, 1, 2>::new(move |i: &[Complex<f32>], o: &mut [i16]| {
        o[0] = (i[0].re * 0.9 * i16::MAX as f32) as i16;
        o[1] = (i[0].im * 0.9 * i16::MAX as f32) as i16;
    });

    let sink = WavSink::<i16>::new(
        Path::new(cli.output.as_str()),
        WavSpec {
            channels: 2,
            sample_rate: file_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    );

    // TODO Make this work with `ssb-receiver`.
    let dat = FileSink::<Complex<f32>>::new(format!("{}.dat", cli.output));
    
    connect!(fg,
        source > lowpass > split;
        split.out0 > delay > to_complex.in0;
        split.out1 > hilbert > to_complex.in1;
        to_complex > resampler > mixer > to_i16_iq > sink;
        mixer > dat;
    );

    Runtime::new().run(fg)?;

    Ok(())
}
