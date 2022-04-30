#![feature(test,core_intrinsics)]

extern crate test;

use test::Bencher;

use futuresdr::anyhow::Result;
use futuresdr::blocks::Apply;
use futuresdr::runtime::Block;
use futuresdr::blocks::NullSink;
use futuresdr::blocks::FiniteSource;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use futuresdr::runtime::buffer::slab::Slab;

mod applyinterleaved;
use applyinterleaved::ApplyInterleaved;

mod applynm;
use applynm::ApplyNM;

mod leftrightbalance;
use leftrightbalance::LeftRightBalanceInterleaved;

mod leftrightbalancefast;
use leftrightbalancefast::LeftRightBalanceInterleavedFast;

#[cfg(not(RUSTC_IS_STABLE))]
use core::intrinsics::{fmul_fast};

fn run_mono_to_stereo(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = ApplyInterleaved::<f32, f32>::new(move |v: &f32, d: &mut [f32]| {
        d[0] = *v * gain_l;
        d[1] = *v * gain_r;
    });
    run_flow_graph(vec_size, slab_size, mono_to_stereo, None)
}

fn run_mono_to_stereo_nm(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = ApplyNM::<f32, f32, 1, 2>::new(move |v: &[f32], d: &mut [f32]| {
        d[0] =  v[0] * gain_l;
        d[1] =  v[0] * gain_r;
    });
    run_flow_graph(vec_size, slab_size, mono_to_stereo, None)
}

fn run_mono_to_stereo_leftrightbalance(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = LeftRightBalanceInterleaved::new(gain_l, gain_r);
    run_flow_graph(vec_size, slab_size, mono_to_stereo, None)
}

fn run_mono_to_stereo_leftrightbalance_fast(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = LeftRightBalanceInterleavedFast::new(gain_l, gain_r);
    run_flow_graph(vec_size, slab_size, mono_to_stereo, None)
}

#[repr(C)]
pub struct StereoSample {
    l: f32,
    r: f32,
}

pub fn run_mono_to_stereo_on_struct(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = Apply::<f32, StereoSample>::new(move |v: &f32| -> StereoSample {
        StereoSample{l: *v * gain_l, r: *v * gain_r}
    });
    let adapt_item_size = ApplyNM::<StereoSample, f32, 1, 2>::new(|v: &[StereoSample], d: &mut [f32]| {
        d[0] = v[0].l;
        d[1] = v[0].r;
    });
    run_flow_graph(vec_size, slab_size, mono_to_stereo, Some(adapt_item_size))
}

pub fn run_mono_to_stereo_on_struct_nm(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = ApplyNM::<f32, f32, 1, 2>::new(move |v: &[f32], d: &mut [f32]| {
        let o = StereoSample{l: v[0] * gain_l, r: v[0] * gain_r};
        d[0] = o.l;
        d[1] = o.r;
    });
    run_flow_graph(vec_size, slab_size, mono_to_stereo, None)
}

fn run_mono_to_stereo_nm_fast(vec_size: usize, slab_size: usize) -> Result<()> {
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;
    let mono_to_stereo = ApplyNM::<f32, f32, 1, 2>::new(move |v: &[f32], d: &mut [f32]| {
        #[cfg(not(RUSTC_IS_STABLE))]
        unsafe {
            d[0] = fmul_fast(v[0], gain_l);
            d[1] = fmul_fast(v[0], gain_r);
        }
        #[cfg(RUSTC_IS_STABLE)]
        {
            assert!(false);
        }
    });
    run_flow_graph(vec_size, slab_size, mono_to_stereo, None)
}

fn run_flow_graph(vec_size: usize, slab_size: usize, mono_to_stereo: Block, adapter: Option<Block>) -> Result<()> {

    let mut fg = Flowgraph::new();

    let mut counter: u16 = 0;
    let vec_size = vec_size as u16;
    let src = FiniteSource::<f32>::new(move || {
        counter += 1;
        if counter == /*vec_size*/ std::u16::MAX {
            return None;
        }
        Some(counter as f32)
    });

    let snk = NullSink::<f32>::new();

    let src = fg.add_block(src);
    let snk = fg.add_block(snk);
    let mono_to_stereo = fg.add_block(mono_to_stereo);

    fg.connect_stream_with_type(src, "out", mono_to_stereo, "in", Slab::with_size(slab_size))?;
    if let Some(adapter) = adapter {
        let adapter = fg.add_block(adapter);
        fg.connect_stream_with_type(mono_to_stereo, "out", adapter, "in", Slab::with_size(slab_size))?;
        fg.connect_stream_with_type(adapter, "out", snk, "in", Slab::with_size(slab_size))?;
    } else {
        fg.connect_stream_with_type(mono_to_stereo, "out", snk, "in", Slab::with_size(slab_size))?;
    }
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

#[bench]
fn mono_to_stereo_4096_applynm(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo_nm(4096, 4096);
    });
}

// #[bench]
// fn mono_to_stereo_1024_on_struct(bencher: &mut Bencher) {
//     bencher.iter(|| {
//         _ = run_mono_to_stereo_on_struct(4096, 1024);
//     });
// }

// #[bench]
// fn mono_to_stereo_2048_on_struct(bencher: &mut Bencher) {
//     bencher.iter(|| {
//         _ = run_mono_to_stereo_on_struct(4096, 2048);
//     });
// }

#[bench]
fn mono_to_stereo_4096_apply_on_struct(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo_on_struct(4096, 4096);
    });
}

#[bench]
fn mono_to_stereo_4096_applynm_with_struct(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo_on_struct_nm(4096, 4096);
    });
}

#[bench]
fn mono_to_stereo_4096_applynm_fast(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo_nm_fast(4096, 4096);
    });
}

#[bench]
fn mono_to_stereo_4096_dedicatedblock(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo_leftrightbalance(4096, 4096);
    });
}

#[bench]
fn mono_to_stereo_4096_dedicatedblock_fast(bencher: &mut Bencher) {
    bencher.iter(|| {
        _ = run_mono_to_stereo_leftrightbalance_fast(4096, 4096);
    });
}