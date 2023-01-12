use futuresdr::anyhow::Result;
use futuresdr::blocks::FiniteSource;
use futuresdr::blocks::Selector;
use futuresdr::blocks::SelectorDropPolicy;
use futuresdr::blocks::SelectorFinishPolicy;
use futuresdr::blocks::VectorSink;
use futuresdr::blocks::VectorSinkBuilder;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;

use futuresdr::macros::connect;

#[test]
fn selector_sequence_3() -> Result<()> {
    let mut fg = Flowgraph::new();

    let mut v0 = vec![0u32, 1, 2, 3].into_iter();
    let src0 = FiniteSource::new(move || v0.next());

    let mut v1 = vec![4u32, 5].into_iter();
    let src1 = FiniteSource::new(move || v1.next());

    let mut v2 = vec![6u32, 7].into_iter();
    let src2 = FiniteSource::new(move || v2.next());

    let selector =
        Selector::<u32, 3, 1>::new(SelectorDropPolicy::NoDrop, SelectorFinishPolicy::MoveToNext);

    let vect_sink = VectorSinkBuilder::<u32>::new().build();

    connect!(fg,
        src0 > selector.in0;
        src1 > selector.in1;
        src2 > selector.in2;
        selector.out0 > vect_sink;
    );

    fg = Runtime::new().run(fg)?;

    let snk = fg.kernel::<VectorSink<u32>>(vect_sink).unwrap();
    let v = snk.items();

    assert_eq!(8, v.len());
    // assert_eq!(vec![0u32, 1, 2, 3, 4, 5, 6, 7], v);
    Ok(())
}
