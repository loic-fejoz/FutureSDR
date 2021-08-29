use std::ptr;

use crate::runtime::AsyncKernel;
use crate::runtime::Block;
use crate::runtime::BlockMeta;
use crate::runtime::BlockMetaBuilder;
use crate::runtime::MessageIo;
use crate::runtime::MessageIoBuilder;
use crate::runtime::StreamIo;
use crate::runtime::StreamIoBuilder;
use crate::runtime::WorkIo;
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};

pub struct AudioSource {
    device: cpal::Device,
    config: cpal::StreamConfig,
}

impl AudioSource {
    pub fn new(device: cpal::Device, config: cpal::StreamConfig) -> Block {
        Block::new_async(
            BlockMetaBuilder::new("Audio_Source").build(),
            StreamIoBuilder::new().add_stream_output("out", 1).build(),
            MessageIoBuilder::<AudioSource>::new().build(),
            AudioSource { device, config },
        )
    }
}

#[async_trait]
impl AsyncKernel for AudioSource {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        print!("AudioSource work\n");
        let stream = self.device.build_output_stream(
            &self.config,
             |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // react to stream events and read or write stream data here.
                let o = sio.output(0).slice::<f32>();
                print!("{:?}", data);
                
                unsafe {
                    let src_ptr = data.as_ptr();
                    let dst_ptr = o.as_mut_ptr();
                    ptr::copy_nonoverlapping(src_ptr, dst_ptr, data.len());
                }
                
                debug_assert_eq!(o.len() % 4/*self.item_size*/, 0);


                sio.output(0).produce(o.len() / 4/*self.item_size*/);

            },
            move |err| {
                // react to errors here.
                print!("{:?}", err);
                io.finished = true
            },
        );
        //        stream.unwrap().play().unwrap();


        Ok(())
    }

    async fn init(
        &mut self,
        _sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        print!("AudioSource Init\n");
        Ok(())
    }

    async fn deinit(
        &mut self,
        _s: &mut StreamIo,
        _m: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        print!("AudioSource Deinit");
        Ok(())
    }
}

pub struct AudioSourceBuilder {
    device: Option<cpal::Device>,
    config: Option<cpal::StreamConfig>,
}

impl AudioSourceBuilder {
    pub fn new() -> AudioSourceBuilder {
        AudioSourceBuilder {
            device: None,
            config: None,
        }
    }

    pub fn default() -> AudioSourceBuilder {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no default output device available");
        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_max_sample_rate();
        let config = supported_config.into();
        AudioSourceBuilder {
            device: Some(device),
            config: Some(config),
        }
    }

    pub fn build(self) -> Block {
        AudioSource::new(
            self.device.expect("Device must be set."),
            self.config.expect("Config must be set."),
        )
    }
}
