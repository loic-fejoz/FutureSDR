use clap::Parser;

use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::Apply;
use futuresdr::blocks::FileSource;
use futuresdr::blocks::FirBuilder;
use futuresdr::blocks::MultistreamSink;
use futuresdr::num_integer::gcd;
use futuresdr::runtime::buffer::slab::Slab;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use num_complex::Complex;

use axum::response::Html;
use axum::Extension;
use futures::stream;
use std::io;
use axum::routing::get;
use axum::Router;
use axum::body::{Bytes, StreamBody};
// use futures::channel::mpsc;
// use futures::channel::oneshot;
use futures::prelude::*;
use tower_http::add_extension::AddExtensionLayer;
use tower_http::cors::CorsLayer;
use std::sync::{Arc, Mutex};

// Inspired by https://wiki.gnuradio.org/index.php/Simulation_example:_Single_Sideband_transceiver

async fn my_route() -> Html<&'static str> {
    Html(
        r#"
    <html>
        <head>
            <meta charset='utf-8' />
            <title>FutureSDR</title>
        </head>
        <body>
            <h1>My Custom Route</h1>
            Visit <a href="/stream.txt">stream</a>
            <audio controls>
                <source src=/stream.wav" type="audio/vnd.wav;codec=1" preload="none">
                Your browser does not support the audio element.
            </audio>
        </body>
    </html>
    "#,
    )
}

async fn handler_my_sound(
    Extension(streams): Extension<Arc<Mutex<Vec<futures::channel::mpsc::Sender<f32>>>>>,
) -> StreamBody<impl Stream<Item = io::Result<Bytes>>> {

    //TODO https://stackoverflow.com/questions/59065564/http-realtime-audio-streaming-server
    //TODO https://stackoverflow.com/questions/51079338/audio-livestreaming-with-python-flask

    let mem = Bytes::from("Hello world");
    let a = mem.slice(0..5);
    let chunks: Vec<io::Result<_>> = vec![
        Ok(a),
        Ok(mem.slice(5..6)),
        Ok(mem.slice(6..)),
    ];

    let stream = stream::iter(chunks);

    let stream = stream.chain(
        MultistreamSink::<f32>::build_new_stream(streams, 1000).map(
            |a|{
                let bytes = a.to_le_bytes().to_vec();
                let bytes = axum::body::Bytes::from(bytes);
                Ok(bytes)
            }
        )
    );
    StreamBody::new(stream)
}

#[derive(Parser, Debug)]
struct Args {
    /// file sample rate
    #[clap(long, default_value_t = 256_000)]
    file_rate: u32,

    /// file to use as a source
    #[clap(short, long, default_value = "ssb_lsb_256k_complex2.dat")]
    filename: String,

    /// Audio Rate
    #[clap(short, long)]
    audio_rate: Option<u32>,

    /// center frequency
    /// explanation in http://www.csun.edu/~skatz/katzpage/sdr_project/sdr/grc_tutorial4.pdf
    #[clap(short, long, default_value_t = 51_500)]
    center_freq: i32,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Configuration {:?}", args);

    let file_rate = args.file_rate as u32;

    let audio_rate = if let Some(r) = args.audio_rate {
        r
    } else {
        let mut audio_rates = AudioSink::supported_sample_rates();
        assert!(!audio_rates.is_empty());
        audio_rates.sort_by_key(|a| std::cmp::Reverse(gcd(*a, file_rate)));
        println!("Supported Audio Rates {:?}", audio_rates);
        audio_rates[0]
    };
    println!("Selected Audio Rate {:?}", audio_rate);
    let mut fg = Flowgraph::new();

    let center_freq = args.center_freq;

    // To be downloaded from https://www.csun.edu/~skatz/katzpage/sdr_project/sdr/ssb_lsb_256k_complex2.dat.zip
    let file_name = args.filename;
    let src_name = format!("File {}", file_name);
    let mut src = FileSource::<Complex<f32>>::repeat(file_name);
    src.set_instance_name(&src_name);

    const FILE_LEVEL_ADJUSTEMENT: f32 = 0.0001;
    let mut xlating_local_oscillator_index: u32 = 0;
    let fwt0: f32 = -2.0 * std::f32::consts::PI * (center_freq as f32) / (file_rate as f32);
    let mut freq_xlating = Apply::new(move |v: &Complex<f32>| {
        let lo_v = Complex::<f32>::new(0.0, (xlating_local_oscillator_index as f32) * fwt0).exp();
        xlating_local_oscillator_index = (xlating_local_oscillator_index + 1) % file_rate;
        FILE_LEVEL_ADJUSTEMENT * v * lo_v
    });
    freq_xlating.set_instance_name(&format!("freq_xlating {}", center_freq));

    // low_pass_filter.set_instance_name(&format!("low pass filter {} {}", cutoff, transition_bw));
    let low_pass_filter = FirBuilder::new_resampling::<Complex<f32>, Complex<f32>>(
        audio_rate as usize,
        file_rate as usize,
    );

    const VOLUME_ADJUSTEMENT: f64 = 0.5;
    const MID_AUDIO_SPECTRUM_FREQ: u32 = 1500;
    let mut ssb_lo_index: u32 = 0;
    let mut weaver_ssb_decode = Apply::new(move |v: &Complex<f32>| {
        let local_oscillator_phase = 2.0f64
            * std::f64::consts::PI
            * (ssb_lo_index as f64)
            * (MID_AUDIO_SPECTRUM_FREQ as f64)
            / (audio_rate as f64);
        let term1 = v.re as f64 * local_oscillator_phase.cos();
        let term2 = v.im as f64 * local_oscillator_phase.sin();
        let result = VOLUME_ADJUSTEMENT * (term1 + term2); // substraction for LSB, addition for USB
        ssb_lo_index = (ssb_lo_index + 1) % audio_rate;
        result as f32
    });
    weaver_ssb_decode.set_instance_name("Weaver SSB decoder");

    // let zmq_snk = PubSinkBuilder::new(8)
    //         .address("tcp://127.0.0.1:50001")
    //         .build();
   
    let snk = AudioSink::new(audio_rate, 1);

    let src = fg.add_block(src);
    let freq_xlating = fg.add_block(freq_xlating);
    let low_pass_filter = fg.add_block(low_pass_filter);
    let weaver_ssb_decode = fg.add_block(weaver_ssb_decode);
    let snk = fg.add_block(snk);
    // let zmq_snk = fg.add_block(zmq_snk);

    // Send bytes into audio stream
    let streams = Vec::<futures::channel::mpsc::Sender<f32>>::new();
    let streams = Arc::new(Mutex::new(streams));
    let streaming_sink = MultistreamSink::<f32>::new(Arc::clone(&streams));
    let streaming_sink = fg.add_block(streaming_sink);
    const SLAB_SIZE: usize = 2 * 2 * 8192;
    fg.connect_stream_with_type(src, "out", freq_xlating, "in", Slab::with_size(SLAB_SIZE))?;
    fg.connect_stream(freq_xlating, "out", low_pass_filter, "in")?;
    fg.connect_stream(low_pass_filter, "out", weaver_ssb_decode, "in")?;

    // fg.connect_stream(low_pass_filter, "out", zmq_snk, "in")?;
    fg.connect_stream(weaver_ssb_decode, "out", snk, "in")?;
    fg.connect_stream(weaver_ssb_decode, "out", streaming_sink, "in")?;

    let router = Router::new()
        .route("/my_route/", get(my_route))
        .route("/stream.txt", get(handler_my_sound))
        .layer(AddExtensionLayer::new(Arc::clone(&streams)))
        .layer(CorsLayer::permissive());
    fg.set_custom_routes(router);

    println!("Visit http://localhost:1337/my_route/");

    Runtime::new().run(fg)?;

    Ok(())
}
