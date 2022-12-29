use futuresdr::anyhow::Result;
use futuresdr::blocks::ControlledOscillatorBuilder;
use futuresdr::blocks::Fft;
use futuresdr::blocks::FiniteSource;
use futuresdr::blocks::VectorSink;
use futuresdr::blocks::VectorSinkBuilder;
use futuresdr::num_complex::Complex32;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;

use futuresdr::macros::connect;

fn check_vco_amplitude(max: f32) -> Result<()> {
    let mut fg = Flowgraph::new();

    const SAMPLE_RATE: f32 = 1_000.0;

    let mut iter = std::iter::repeat(440.0).take(1024);
    let src = FiniteSource::<_, f32>::new(move || iter.next());
    let vco = ControlledOscillatorBuilder::<f32, f32>::freq_co(SAMPLE_RATE, 1.0, 1.0).build();
    let vect_sink = VectorSinkBuilder::<f32>::new().build();

    connect!(fg,
        src > vco > vect_sink
    );

    fg = Runtime::new().run(fg)?;

    let snk = fg.kernel::<VectorSink<f32>>(vect_sink).unwrap();
    let v = snk.items();

    assert!(v.iter().all(|v| -max <= *v && *v <= max));
    assert!(v.iter().any(|v| *v >= max / 2.0));
    Ok(())
}

#[test]
fn vco_amplitude() -> Result<()> {
    check_vco_amplitude(1.0)?;
    check_vco_amplitude(2.0)?;
    Ok(())
}

fn check_freq_co_frequency(freq: f32, sensitivity: f32) -> Result<()> {
    let mut fg = Flowgraph::new();

    const SAMPLE_RATE: f32 = 1_000.0;
    const FFT_SIZE: usize = 512;

    let mut iter = std::iter::repeat(freq / sensitivity).take(1024);
    let src = FiniteSource::<_, f32>::new(move || iter.next());
    let vco = ControlledOscillatorBuilder::<f32, Complex32>::freq_co(
        SAMPLE_RATE,
        sensitivity,
        1.0.into(),
    )
    .build();
    let fft = Fft::new(FFT_SIZE);
    let vect_sink = VectorSinkBuilder::<Complex32>::new().build();

    connect!(fg,
        src > vco > fft > vect_sink
    );

    fg = Runtime::new().run(fg)?;

    let snk = fg.kernel::<VectorSink<Complex32>>(vect_sink).unwrap();
    let v = snk.items();
    let index_of_max: usize = v
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.norm_sqr().total_cmp(&b.norm_sqr()))
        .map(|(index, _)| index)
        .expect("Max FFT shall exists");
    let index_of_max = index_of_max % FFT_SIZE;
    assert_eq!(
        index_of_max,
        (freq * (FFT_SIZE as f32) / SAMPLE_RATE) as usize
    );

    Ok(())
}

fn check_vco_frequency(freq: f32, sensitivity: f32) -> Result<()> {
    let mut fg = Flowgraph::new();

    const SAMPLE_RATE: f32 = 1_000.0;
    const FFT_SIZE: usize = 512;

    let mut iter = std::iter::repeat(2.0 * core::f32::consts::PI * freq / sensitivity).take(1024);
    let src = FiniteSource::<_, f32>::new(move || iter.next());
    let vco =
        ControlledOscillatorBuilder::<f32, Complex32>::vco(SAMPLE_RATE, sensitivity, 1.0.into())
            .build();
    let fft = Fft::new(FFT_SIZE);
    let vect_sink = VectorSinkBuilder::<Complex32>::new().build();

    connect!(fg,
        src > vco > fft > vect_sink
    );

    fg = Runtime::new().run(fg)?;

    let snk = fg.kernel::<VectorSink<Complex32>>(vect_sink).unwrap();
    let v = snk.items();
    let index_of_max: usize = v
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.norm_sqr().total_cmp(&b.norm_sqr()))
        .map(|(index, _)| index)
        .expect("Max FFT shall exists");
    let index_of_max = index_of_max % FFT_SIZE;
    assert_eq!(
        index_of_max,
        (freq * (FFT_SIZE as f32) / SAMPLE_RATE) as usize
    );

    Ok(())
}

#[test]
fn vco_frequency() -> Result<()> {
    check_freq_co_frequency(440.0, 1.0)?;
    check_freq_co_frequency(325.0, 1.0)?;
    check_freq_co_frequency(440.0, 2.0)?;
    check_vco_frequency(440.0, 1.0)?;
    Ok(())
}
