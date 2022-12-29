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

enum ControlKind {
    VCO,
    VCO_FREQ,
}

pub struct ControlledOscillatorBuilder<A, B> {
    sample_rate: f32,
    initial_frequency: f32,
    initial_phase: f32,
    kind: ControlKind,
    amplitude: B,
    sensitivity: A,
}

impl<A, B> ControlledOscillatorBuilder<A, B>
where
    A: Copy + Send + 'static,
    B: Send + 'static,
{
    pub fn initial_phase(mut self, initial_phase: f32) -> ControlledOscillatorBuilder<A, B> {
        self.initial_phase = initial_phase;
        self
    }

    pub fn sample_rate(mut self, sample_rate: f32) -> ControlledOscillatorBuilder<A, B> {
        self.sample_rate = sample_rate;
        self
    }

    /// Oscillator controlled by angle rate.
    /// input: float stream of voltage to control angle rate.
    /// `sample_rate` in Hz
    /// `sensitivity` units are radians/sec/(input unit)
    pub fn vco(
        sample_rate: f32,
        sensitivity: A,
        amplitude: B,
    ) -> ControlledOscillatorBuilder<A, B> {
        ControlledOscillatorBuilder::<A, B> {
            sample_rate,
            initial_frequency: 0.0,
            initial_phase: 0.0,
            kind: ControlKind::VCO,
            amplitude,
            sensitivity,
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
    ) -> ControlledOscillatorBuilder<A, B> {
        ControlledOscillatorBuilder::<A, B> {
            sample_rate,
            initial_frequency: 0.0,
            initial_phase: 0.0,
            kind: ControlKind::VCO_FREQ,
            amplitude,
            sensitivity,
        }
    }
}

impl<A> ControlledOscillatorBuilder<A, f32>
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
        match self.kind {
            ControlKind::VCO_FREQ => ControlledOscillator::new(
                move |v: &A, nco: &mut NCO| {
                    let freq: f32 = (*v * self.sensitivity).into();
                    nco.set_frequency(freq, self.sample_rate);
                    nco.phase.cos() * self.amplitude
                },
                nco,
            ),
            ControlKind::VCO => ControlledOscillator::new(
                move |v: &A, nco: &mut NCO| {
                    let angle_rate: f32 = f32::from(*v * self.sensitivity) / self.sample_rate;
                    nco.set_angle_rate(angle_rate);
                    nco.phase.cos() * self.amplitude
                },
                nco,
            ),
        }
    }
}

impl<A> ControlledOscillatorBuilder<A, Complex32>
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
        match self.kind {
            ControlKind::VCO_FREQ => ControlledOscillator::new(
                move |v: &A, nco: &mut NCO| {
                    let freq: f32 = (*v * self.sensitivity).into();
                    nco.set_frequency(freq, self.sample_rate);
                    Complex32::new(nco.phase.cos(), nco.phase.sin()) * self.amplitude
                },
                nco,
            ),
            ControlKind::VCO => ControlledOscillator::new(
                move |v: &A, nco: &mut NCO| {
                    let angle_rate: f32 = f32::from(*v * self.sensitivity) / self.sample_rate;
                    nco.set_angle_rate(angle_rate);
                    Complex32::new(nco.phase.cos(), nco.phase.sin()) * self.amplitude
                },
                nco,
            ),
        }
    }
}
