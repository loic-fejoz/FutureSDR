#![feature(core_intrinsics)]

#[cfg(not(RUSTC_IS_STABLE))]
use core::intrinsics::{fmul_fast};

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

pub struct LeftRightBalanceInterleavedFast
{
    gain_l: f32,
    gain_r: f32,
}

impl LeftRightBalanceInterleavedFast
{
    pub fn new(gain_l: f32, gain_r: f32) -> Block {
        Block::new_sync(
            BlockMetaBuilder::new("LeftRightBalanceInterleavedFast").build(),
            StreamIoBuilder::new()
                .add_input("in", mem::size_of::<f32>())
                .add_output("out", mem::size_of::<f32>())
                .build(),
            MessageIoBuilder::<LeftRightBalanceInterleavedFast>::new().build(),
            LeftRightBalanceInterleavedFast {gain_l, gain_r},
        )
    }
}

impl SyncKernel for LeftRightBalanceInterleavedFast
{
    fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        let i = sio.input(0).slice::<f32>();
        let o = sio.output(0).slice::<f32>();

        let m = std::cmp::min(i.len(), o.len() / 2);
        if m > 0 {
            for (v, r) in i.iter().zip(o.chunks_exact_mut(2)) {
                #[cfg(not(RUSTC_IS_STABLE))]
                unsafe {
                    r[0] = fmul_fast(*v, self.gain_l);
                    r[1] = fmul_fast(*v, self.gain_r);
                }
                #[cfg(RUSTC_IS_STABLE)]
                {
                    assert!(false);
                    r[0] = v * self.gain_l;
                    r[1] = v * self.gain_r;
                }
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
