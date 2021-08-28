use anyhow::Result;
use futuresdr::blocks::AudioSourceBuilder;
use std::time::Duration;

use futuresdr::blocks::AudioSource;
use futuresdr::blocks::NullSink;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Pmt;
use futuresdr::runtime::Runtime;

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let audio_src = fg.add_block(AudioSourceBuilder::default().build());
    let snk = fg.add_block(NullSink::new(1));

    fg.connect_stream(audio_src, "out", snk, "in")?;

    Runtime::new().run(fg)?;

    Ok(())
}