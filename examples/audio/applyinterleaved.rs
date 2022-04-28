use std::mem;

use futuresdr::anyhow::Result;
use futuresdr::runtime::Block;
use futuresdr::runtime::BlockMeta;
use futuresdr::runtime::BlockMetaBuilder;
use futuresdr::runtime::MessageIo;
use futuresdr::runtime::MessageIoBuilder;
use futuresdr::runtime::StreamIo;
use futuresdr::runtime::StreamIoBuilder;
use futuresdr::runtime::SyncKernel;
use futuresdr::runtime::WorkIo;

pub struct ApplyInterleaved<A, B>
where
    A: 'static,
    B: 'static,
{
    f: Box<dyn FnMut(&A, &mut [B])  + Send + 'static>,
}

impl<A, B> ApplyInterleaved<A, B>
where
    A: 'static,
    B: 'static,
{
    pub fn new(f: impl FnMut(&A, &mut [B]) + Send + 'static) -> Block {
        Block::new_sync(
            BlockMetaBuilder::new("ApplyInterleaved").build(),
            StreamIoBuilder::new()
                .add_input("in", mem::size_of::<A>())
                .add_output("out", mem::size_of::<B>())
                .build(),
            MessageIoBuilder::<ApplyInterleaved<A, B>>::new().build(),
            ApplyInterleaved { f: Box::new(f) },
        )
    }
}

impl<A, B> SyncKernel for ApplyInterleaved<A, B>
where
    A: 'static,
    B: 'static,
{
    fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        let i = sio.input(0).slice::<A>();
        let o = sio.output(0).slice::<B>();

        let m = std::cmp::min(i.len(), o.len() / 2);
        if m > 0 {
            for (v, r) in i.iter().zip(o.chunks_exact_mut(2)) {
                (self.f)(v, r);
            }

            sio.input(0).consume(m);
            sio.output(0).produce(2*m);
        }

        if sio.input(0).finished() && m == i.len() {
            io.finished = true;
        }

        Ok(())
    }
}
