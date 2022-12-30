use crate::anyhow::Result;
use crate::blocks::signal_source::NCO;
use crate::num_complex::Complex32;
use crate::runtime::Block;
use crate::runtime::BlockMeta;
use crate::runtime::BlockMetaBuilder;
use crate::runtime::Kernel;
use crate::runtime::MessageIo;
use crate::runtime::MessageIoBuilder;
use crate::runtime::StreamIo;
use crate::runtime::StreamIoBuilder;
use crate::runtime::WorkIo;

use std::marker::PhantomData;
use std::ops::Mul;

pub struct ControlledOscillator<F, A, B>
where
    A: Copy + Send + 'static,
    B: Send + 'static,
    F: FnMut(&A, &mut NCO) -> B + Send + 'static,
{
    nco: NCO,
    controller: F,
    _a: std::marker::PhantomData<A>,
    _b: std::marker::PhantomData<B>,
}

impl<F, A, B> ControlledOscillator<F, A, B>
where
    F: FnMut(&A, &mut NCO) -> B + Send + 'static,
    A: Copy + Send + 'static + std::marker::Sync,
    B: Send + 'static,
{
    pub fn new(controller: F, nco: NCO) -> Block {
        Block::new(
            BlockMetaBuilder::new("ControlledOscillator").build(),
            StreamIoBuilder::new()
                .add_input::<A>("in")
                .add_output::<B>("out")
                .build(),
            MessageIoBuilder::<Self>::new().build(),
            ControlledOscillator {
                _a: PhantomData,
                _b: PhantomData,
                nco,
                controller,
            },
        )
    }
}

#[doc(hidden)]
#[async_trait]
impl<F, A, B> Kernel for ControlledOscillator<F, A, B>
where
    F: FnMut(&A, &mut NCO) -> B + Send + 'static,
    A: Copy + Send + 'static,
    B: Send + 'static,
{
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        let i = sio.input(0).slice::<A>();
        let o = sio.output(0).slice::<B>();

        let m = std::cmp::min(i.len(), o.len());
        if m > 0 {
            for (v, r) in i.iter().zip(o.iter_mut()) {
                *r = (self.controller)(v, &mut self.nco);
                self.nco.step();
            }

            sio.input(0).consume(m);
            sio.output(0).produce(m);
        }

        if sio.input(0).finished() && m == i.len() {
            io.finished = true;
        }

        Ok(())
    }
}

#[repr(u8)]
#[derive(PartialEq, Eq)]
pub enum ControlKind {
    VCO,
    VcoFreq,
    BPSK,
    ASK,
}

pub struct ControlledOscillatorBuilder<A, B> {
    _a: std::marker::PhantomData<A>,
    _b: std::marker::PhantomData<B>,
}

pub struct ControlledOscillatorInternalBuilder<A, B, const S: ControlKind> {
    sample_rate: f32,
    initial_frequency: f32,
    initial_phase: f32,
    amplitude: B,
    sensitivity: Option<A>,
}

impl<A, B, const S: ControlKind> ControlledOscillatorInternalBuilder<A, B, S>
where
    A: Copy + Send + 'static,
    B: Send + 'static,
{
    pub fn initial_phase(mut self, initial_phase: f32) -> ControlledOscillatorInternalBuilder<A, B, S> {
        self.initial_phase = initial_phase;
        self
    }

    pub fn sample_rate(mut self, sample_rate: f32) -> ControlledOscillatorInternalBuilder<A, B, S> {
        self.sample_rate = sample_rate;
        self
    }
}

impl<A, B> ControlledOscillatorBuilder<A, B>
where
    A: Copy + Send + 'static,
    B: Send + 'static,
{
    /// Oscillator controlled by angle rate.
    /// input: float stream of voltage to control angle rate.
    /// `sample_rate` in Hz
    /// `sensitivity` units are radians/sec/(input unit)
    pub fn vco(
        sample_rate: f32,
        sensitivity: A,
        amplitude: B,
    ) -> ControlledOscillatorInternalBuilder<A, B, {ControlKind::VCO}> {
        ControlledOscillatorInternalBuilder::<A, B, {ControlKind::VCO}> {
            sample_rate,
            initial_frequency: 0.0,
            initial_phase: 0.0,
            amplitude,
            sensitivity: Some(sensitivity),
        }
    }

    /// Oscillator controlled by frequency
    /// input: float stream of voltage to control frequency.
    /// `sample_rate` in Hz
    /// `sensitivity` units are Hz/(input unit)
    pub fn freq_co(
        sample_rate: f32,
        sensitivity: A,
        amplitude: B,
    ) -> ControlledOscillatorInternalBuilder<A, B, {ControlKind::VcoFreq}> {
        ControlledOscillatorInternalBuilder::<A, B, {ControlKind::VcoFreq}> {
            sample_rate,
            initial_frequency: 0.0,
            initial_phase: 0.0,
            amplitude,
            sensitivity: Some(sensitivity),
        }
    }
}

impl<A, B> ControlledOscillatorBuilder<A, B>
where
    A: Copy + Send + 'static + Default,
    B: Send + 'static + From<f32>,
{
    /// Generate a binary phase shift encoded stream
    /// input: stream of symbol
    /// `sample_rate` in Hz
    /// `frequency`in Hz
    pub fn bpsk(
        sample_rate: f32,
        frequency: f32,
    ) ->  ControlledOscillatorInternalBuilder<A, B, {ControlKind::BPSK}> {
        ControlledOscillatorInternalBuilder::<A, B, {ControlKind::BPSK}> {
            sample_rate,
            initial_frequency: frequency,
            initial_phase: 0.0,
            amplitude: B::from(1.0),
            sensitivity: None,
        }
    }
}

impl<A> ControlledOscillatorInternalBuilder<A, f32, {ControlKind::VcoFreq}>
where
    A: Copy + Send + 'static + Into<f32> + Mul + std::marker::Sync,
    // F: FnMut(A, &mut NCO) -> f32 + Send + 'static,
    f32: From<<A as Mul>::Output>,
{
    pub fn build(self) -> Block {
        let nco = NCO::new(
            self.initial_phase,
            2.0 * core::f32::consts::PI * self.initial_frequency / self.sample_rate,
        );
        ControlledOscillator::new(
            move |v: &A, nco: &mut NCO| {
                let freq: f32 = (*v * self.sensitivity.expect("sensitivity is mandatory for VCO")).into();
                nco.set_frequency(freq, self.sample_rate);
                nco.phase.cos() * self.amplitude
            },
            nco,
        )
    }
}

impl<A> ControlledOscillatorInternalBuilder<A, f32, {ControlKind::VCO}>
where
    A: Copy + Send + 'static + Into<f32> + Mul + std::marker::Sync,
    // F: FnMut(A, &mut NCO) -> f32 + Send + 'static,
    f32: From<<A as Mul>::Output>,
{
    pub fn build(self) -> Block {
        let nco = NCO::new(
            self.initial_phase,
            2.0 * core::f32::consts::PI * self.initial_frequency / self.sample_rate,
        );
        ControlledOscillator::new(
            move |v: &A, nco: &mut NCO| {
                let angle_rate: f32 = f32::from(*v * self.sensitivity.expect("sensitivity is mandatory for VCO")) / self.sample_rate;
                nco.set_angle_rate(angle_rate);
                nco.phase.cos() * self.amplitude
            },
            nco,
        )
    }
}

impl<A> ControlledOscillatorInternalBuilder<A, f32, {ControlKind::BPSK}>
where
    A: Copy + Send + 'static + std::marker::Sync + std::cmp::PartialEq,
{
    pub fn build(self) -> Block {
        let nco = NCO::new(
            self.initial_phase,
            2.0 * core::f32::consts::PI * self.initial_frequency / self.sample_rate,
        );
        let mut previous_v: Option<A> = None;
        ControlledOscillator::new(
            move |v: &A, nco: &mut NCO| {
                if Some(*v) != previous_v {
                    nco.adjust_phase(core::f32::consts::PI);
                    previous_v = Some(*v);
                }
                nco.phase.cos() * self.amplitude
            },
            nco,
        )
    }
}

impl<A> ControlledOscillatorInternalBuilder<A, Complex32, {ControlKind::VcoFreq}>
where
    A: Copy + Send + 'static + Into<f32> + Mul + std::marker::Sync,
    // F: FnMut(A, &mut NCO) -> f32 + Send + 'static,
    f32: From<<A as Mul>::Output>,
{
    pub fn build(self) -> Block {
        let nco = NCO::new(
            self.initial_phase,
            2.0 * core::f32::consts::PI * self.initial_frequency / self.sample_rate,
        );
        ControlledOscillator::new(
            move |v: &A, nco: &mut NCO| {
                let freq: f32 = (*v * self.sensitivity.expect("sensitivity is mandatory for VCO")).into();
                nco.set_frequency(freq, self.sample_rate);
                Complex32::new(nco.phase.cos(), nco.phase.sin()) * self.amplitude
            },
            nco,
        )
    }
}

impl<A> ControlledOscillatorInternalBuilder<A, Complex32, {ControlKind::VCO}>
where
    A: Copy + Send + 'static + Into<f32> + Mul + std::marker::Sync,
    // F: FnMut(A, &mut NCO) -> f32 + Send + 'static,
    f32: From<<A as Mul>::Output>,
{
    pub fn build(self) -> Block {
        let nco = NCO::new(
            self.initial_phase,
            2.0 * core::f32::consts::PI * self.initial_frequency / self.sample_rate,
        );
        ControlledOscillator::new(
            move |v: &A, nco: &mut NCO| {
                let angle_rate: f32 = f32::from(*v * self.sensitivity.expect("sensitivity is mandatory for VCO")) / self.sample_rate;
                nco.set_angle_rate(angle_rate);
                Complex32::new(nco.phase.cos(), nco.phase.sin()) * self.amplitude
            },
            nco,
        )
    }
}

impl<A> ControlledOscillatorInternalBuilder<A, Complex32, {ControlKind::BPSK}>
where
    A: Copy + Send + 'static + std::marker::Sync + std::cmp::PartialEq,
{
    pub fn build(self) -> Block {
        let nco = NCO::new(
            self.initial_phase,
            2.0 * core::f32::consts::PI * self.initial_frequency / self.sample_rate,
        );
        let mut previous_v: Option<A> = None;
        ControlledOscillator::new(
            move |v: &A, nco: &mut NCO| {
                if Some(*v) != previous_v {
                    nco.adjust_phase(core::f32::consts::PI);
                    previous_v = Some(*v);
                }
                Complex32::new(nco.phase.cos(), nco.phase.sin()) * self.amplitude
            },
            nco,
        )
    }
}
