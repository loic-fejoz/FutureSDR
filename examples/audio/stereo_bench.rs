#![feature(test)]

extern crate test;

use test::Bencher;

use futuresdr::anyhow::Result;
use futuresdr::blocks::NullSink;
use futuresdr::blocks::VectorSourceBuilder;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
mod applyinterleaved;
use applyinterleaved::ApplyInterleaved;
use futuresdr::runtime::buffer::slab::Slab;

fn run_mono_to_stereo(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;

    let mut fg = Flowgraph::new();

    let src = VectorSourceBuilder::<u32>::new(vec![1; vec_size]).build();
    let mono_to_stereo = ApplyInterleaved::<f32, f32>::new(move |v: &f32, d: &mut [f32]| {
        d[0] = v * gain_l;
        d[1] = v * gain_r;
    });
    let snk = NullSink::<f32>::new();

    let src = fg.add_block(src);
    let snk = fg.add_block(snk);
    let mono_to_stereo = fg.add_block(mono_to_stereo);

    fg.connect_stream_with_type(src, "out", mono_to_stereo, "in", Slab::with_size(slab_size))?;
    fg.connect_stream_with_type(mono_to_stereo, "out", snk, "in", Slab::with_size(slab_size))?;

    Runtime::new().run(fg)?;

    Ok(())
}

#[bench]
fn mono_to_stereo_1024(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo(4096, 1024);
    });
}

#[bench]
fn mono_to_stereo_2048(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo(4096, 2048);
    });
}

#[bench]
fn mono_to_stereo_4096(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo(4096, 4096);
    });
}
