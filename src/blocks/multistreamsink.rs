use std::mem;

use crate::anyhow::Result;
use crate::runtime::Block;
use crate::runtime::BlockMeta;
use crate::runtime::BlockMetaBuilder;
use crate::runtime::Kernel;
use crate::runtime::MessageIo;
use crate::runtime::MessageIoBuilder;
use crate::runtime::StreamIo;
use crate::runtime::StreamIoBuilder;
use crate::runtime::WorkIo;
use crate::futures::SinkExt;
use std::sync::{Arc, Mutex};

pub struct MultistreamSink<T: 'static> {
    streams: Arc<Mutex<Vec<futures::channel::mpsc::Sender<T>>>>
}

impl <T> MultistreamSink<T>
where
    T: Send + 'static + std::marker::Sync + std::marker::Copy
{
    pub fn new(streams: Arc<Mutex<Vec<futures::channel::mpsc::Sender<T>>>>) -> Block {
        Block::new(
            BlockMetaBuilder::new("MultistreamSink").build(),
            StreamIoBuilder::new().add_input("in", mem::size_of::<T>()).build(),
            MessageIoBuilder::new().build(),
            MultistreamSink::<T> {
                streams
            }
        )
    }

    pub fn build_new_stream(streams: Arc<Mutex<Vec<futures::channel::mpsc::Sender<T>>>>, buffer: usize) -> futures::channel::mpsc::Receiver<T> {
        let (sender, receiver) = futures::channel::mpsc::channel(buffer);
        streams.lock().unwrap().push(sender);
        receiver
    }
}

#[async_trait]
impl <T> Kernel for MultistreamSink<T>
where
    T: Send + Copy + 'static + std::marker::Sync + std::marker::Copy
{
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {

        let i = sio.input(0).slice::<T>();

        let m = i.len();
        let mut count = 0;
        if m > 0 {
            for v in i.iter() {
                count = count + 1;
                let mut actual_streams = self.streams.lock().unwrap().clone();
                for sender in actual_streams.iter_mut() {
                    sender.send(*v).await.unwrap();
                }
            }

            sio.input(0).consume(m);
        }

        if sio.input(0).finished() && m == i.len() {
            io.finished = true;
        }

        Ok(())
    }

    async fn init(
        &mut self,
        _sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        Ok(())
    }
}