use futuresdr::anyhow::Result;
use futuresdr::blocks::Apply;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::audio::FileSource;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
mod applynm;
use applynm::ApplyNM;
use futuresdr::runtime::buffer::slab::Slab;

/// Just like in https://www.nickwilcox.com/blog/autovec/
/// the goal is to check wether current implementation
/// enable autovectorization

#[repr(C)]
pub struct StereoSample {
    l: f32,
    r: f32,
}

fn main() -> Result<()> {
    const SLAB_SIZE: usize = 2048;
    let gain_l: f32 = 0.8;
    let gain_r: f32 = 0.9;

    let mut fg = Flowgraph::new();

    let src = FileSource::new("rick.mp3");
    let inner = src.as_async::<FileSource>().unwrap();
    assert_eq!(inner.channels(), 1, "We expect mp3 to be single channel.");
    let mono_to_stereo = Apply::<f32, StereoSample>::new(move |v: &f32| -> StereoSample {
        StereoSample{l: *v * gain_l, r: *v * gain_r}
    });
    let adapt_item_size = ApplyNM::<StereoSample, f32, 1, 2>::new(move |v: &[StereoSample], d: &mut [f32]| {
        d[0] = v[0].l;
        d[1] = v[0].r;
    });
    let snk = AudioSink::new(inner.sample_rate(), 2);

    let src = fg.add_block(src);
    let snk = fg.add_block(snk);
    let mono_to_stereo = fg.add_block(mono_to_stereo);
    let adapt_item_size = fg.add_block(adapt_item_size);

    fg.connect_stream_with_type(src, "out", mono_to_stereo, "in", Slab::with_size(SLAB_SIZE))?;
    fg.connect_stream_with_type(mono_to_stereo, "out", adapt_item_size, "in", Slab::with_size(SLAB_SIZE))?;
    fg.connect_stream_with_type(adapt_item_size, "out", snk, "in", Slab::with_size(SLAB_SIZE))?;

    Runtime::new().run(fg)?;

    Ok(())
}
