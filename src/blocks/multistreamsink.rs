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
        let mut actual_streams = self.streams.lock().unwrap().clone();
        println!("#channels: {}", actual_streams.len());
        let mut count = 0;
        if i.len() > 0 {
            for v in i.iter() {
                // if actual_streams.iter().all(|sender| sender.poll_ready() == Ok(Async::Ready(_))) {
                    for sender in actual_streams.iter_mut() {
                        if sender.is_closed() {
                            //self.streams.lock().unwrap().remove(sender);
                            continue;
                        }
                        //sender.try_send(*v);
                        if let std::result::Result::Err(err) = sender.send(*v).await {
                            println!("stream closed: {:?}", err);
                        }
                    }
                    count = count + 1;
                // }
            }

            sio.input(0).consume(count);
        }

        if sio.input(0).finished() && count == i.len() {
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