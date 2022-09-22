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
use std::sync::atomic::{AtomicUsize, Ordering};
use bytes::{Bytes, BytesMut, BufMut};
use futures::{stream,Stream};
use crate::futures::StreamExt;

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

    pub fn build_riff_wav_header(bits_per_sample: u32) -> futures::stream::Iter<std::vec::IntoIter<u8>> {
        //see https://stackoverflow.com/questions/59065564/http-realtime-audio-streaming-server
        //and https://stackoverflow.com/questions/51079338/audio-livestreaming-with-python-flask

        let datasize: u32 = 10240000; // Some veeery big number here instead of: #samples * channels * bitsPerSample // 8
        let audio_rate: u32 = 48_000;
        let channels: u32 = 1;
        // let bitsPerSample: u32 = 16;
        let bytes_per_second: u32 = audio_rate * channels * bits_per_sample / 8;
        let mut header = BytesMut::with_capacity(bytes_per_second as usize);
        header.put(&b"RIFF"[..]);                                                   // (4byte) Marks file as RIFF
        header.put_u32_le(datasize+36);                                             // (4byte) File size in bytes excluding this and RIFF marker
        header.put(&b"WAVE"[..]);                                              // (4byte) File type
        header.put(&b"fmt "[..]);                                      // (4byte) Format Chunk Marker
        header.put_u32_le(16);                                         // (4byte) Length of above format data
        header.put_u16_le(1);                                   // (2byte) Format type (1 - PCM)
        header.put_u16_le(channels as u16);// o += (channels).to_bytes(2,'little')                                    // (2byte)
        header.put_u32_le(audio_rate);// o += (sampleRate).to_bytes(4,'little')                                  // (4byte)
        header.put_u32_le(bytes_per_second);// o += (sampleRate * channels * bitsPerSample // 8).to_bytes(4,'little')  // (4byte)
        header.put_u16_le((channels * bits_per_sample / 8) as u16);// o += (channels * bitsPerSample // 8).to_bytes(2,'little')               // (2byte)
        header.put_u16_le(bits_per_sample as u16);// o += (bitsPerSample).to_bytes(2,'little')                               // (2byte)
        header.put(&b"data "[..]);// o += bytes("data",'ascii')                                              // (4byte) Data Chunk Marker
        header.put_u32_le(datasize);// o += (datasize).to_bytes(4,'little')                                    // (4byte) Data size in bytes

        stream::iter(header.to_vec())
    }
}

impl MultistreamSink<u8> {
    pub fn as_riff_wav_stream<E>(streams: Arc<Mutex<Vec<futures::channel::mpsc::Sender<u8>>>>, buffer: usize) -> impl Stream<Item=Result<Bytes,E>> + 'static {
        let header = MultistreamSink::<u8>::build_riff_wav_header(8);
        let stream = header.map(
            |a|{
                let bytes = a.to_le_bytes().to_vec();
                let bytes = axum::body::Bytes::from(bytes);
                Ok::<bytes::Bytes, E>(bytes)
            }
        );
    
        let receiver = MultistreamSink::<u8>::build_new_stream(streams, buffer);
        let stream = stream.chain(
            receiver.map(
                |a|{
                    let bytes = a.to_le_bytes().to_vec();
                    let bytes = axum::body::Bytes::from(bytes);
                    Ok(bytes)
                }
            )
        );
        stream
    }
}

impl MultistreamSink<i16> {
    pub fn as_riff_wav_stream<E>(streams: Arc<Mutex<Vec<futures::channel::mpsc::Sender<i16>>>>, buffer: usize) -> impl Stream<Item=Result<Bytes,E>> + 'static {
        let header = MultistreamSink::<u16>::build_riff_wav_header(16);
        let stream = header.map(
            |a|{
                let bytes = a.to_le_bytes().to_vec();
                let bytes = axum::body::Bytes::from(bytes);
                Ok::<bytes::Bytes, E>(bytes)
            }
        );
    
        let receiver = MultistreamSink::<i16>::build_new_stream(streams, buffer);
        // TODO when this http connection is dropped, need to empty receiver and close it nicely
        let stream = stream.chain(
            receiver.map(
                |a|{
                    let bytes = a.to_le_bytes().to_vec();
                    let bytes = axum::body::Bytes::from(bytes);
                    Ok(bytes)
                }
            )
        );
        stream
    }
}

impl MultistreamSink<f32> {
    pub fn as_riff_wav_stream<E>(streams: Arc<Mutex<Vec<futures::channel::mpsc::Sender<f32>>>>, buffer: usize) -> impl Stream<Item=Result<Bytes,E>> + 'static {
        let header = MultistreamSink::<f32>::build_riff_wav_header(16);
        let stream = header.map(
            |a|{
                let bytes = a.to_le_bytes().to_vec();
                let bytes = axum::body::Bytes::from(bytes);
                Ok::<bytes::Bytes, E>(bytes)
            }
        );
    
        let receiver = MultistreamSink::<f32>::build_new_stream(streams, buffer);
        // TODO when this http connection is dropped, need to empty receiver and close it nicely
        let stream = stream.chain(
            receiver.map(
                |a|{
                    let bytes = (((std::i16::MAX as f32)/512.0*a) as i16).to_le_bytes().to_vec();
                    let bytes = axum::body::Bytes::from(bytes);
                    Ok(bytes)
                }
            )
        );
        stream
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

        static PREVIOUS_SENDERS_COUNT: AtomicUsize = AtomicUsize::new(0);

        let i = sio.input(0).slice::<T>();
        let mut actual_streams = self.streams.lock().unwrap().clone();
        let current_senders_count = actual_streams.len();
        if current_senders_count != PREVIOUS_SENDERS_COUNT.load(Ordering::Relaxed) {
            println!("#channels: {}", current_senders_count );
        }
        PREVIOUS_SENDERS_COUNT.store(current_senders_count, Ordering::Relaxed);
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
                            panic!("argh");
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