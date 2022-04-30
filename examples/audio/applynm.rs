use std::mem;
use std::time::{Duration,Instant};

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

pub struct ApplyNM<A, B, const N: usize, const M: usize>
where
    A: 'static,
    B: 'static,
{
    f: Box<dyn FnMut(&[A], &mut [B])  + Send + 'static>,
    acc_duration: Duration,
    work_count: u32,
}

impl<A, B, const N: usize, const M: usize> ApplyNM<A, B, N, M>
where
    A: 'static,
    B: 'static,
{
    pub fn new(f: impl FnMut(&[A], &mut [B]) + Send + 'static) -> Block {
        Block::new_sync(
            BlockMetaBuilder::new("ApplyNM").build(),
            StreamIoBuilder::new()
                .add_input("in", mem::size_of::<A>())
                .add_output("out", mem::size_of::<B>())
                .build(),
            MessageIoBuilder::<ApplyNM<A, B, N, M>>::new().build(),
            ApplyNM { f: Box::new(f), acc_duration: Duration::from_secs(0), work_count: 0 },
        )
    }
}

impl<A, B, const N: usize, const M: usize> SyncKernel for ApplyNM<A, B, N, M>
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
        let now = Instant::now();
        let i = sio.input(0).slice::<A>();
        let o = sio.output(0).slice::<B>();

        let m = std::cmp::min(i.len() / N, o.len() / M);
        if m > 0 {
            for (v, r) in i.chunks_exact(N).zip(o.chunks_exact_mut(M)) {
                (self.f)(v, r);
            }

            sio.input(0).consume(N * m);
            sio.output(0).produce(M * m);
        }

        if sio.input(0).finished() && m == i.len() {
            io.finished = true;
        }
        let work_duration = now.elapsed();
        self.acc_duration += work_duration;
        self.work_count += 1;

        if self.work_count % 1000 == 0 || (sio.input(0).finished() && m == i.len()) {
            eprint!("count: {:?}, average: {:?}\n", self.work_count, self.acc_duration/self.work_count);
        }

        Ok(())
    }

    // fn init(
    //     &mut self,
    //     _sio: &mut StreamIo,
    //     _mio: &mut MessageIo<Self>,
    //     _meta: &mut BlockMeta,
    // ) -> Result<()> {
    //     self.work_count = 0;
    //     self.acc_duration = Duration::from_secs(0);
    //     Ok(())
    // }
}
