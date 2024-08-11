use crate::*;

use embedded_dma as dma;
use libm::*;
use nrf52833_hal::pwm;

type BuzzerPin = gpio::p0::P0_02<gpio::Output<gpio::PushPull>>;

struct Timer {
    scale: pwm::Prescaler,
    period: u32,
}

const TIMER: Timer = Timer {
    scale: pwm::Prescaler::Div8,
    period: 2_000_000,
};

/// Error during buzzer driver operation.
pub enum Error<PWM> {
    /// PWM error.
    PwmError(pwm::Error, PWM, pwm::Pins),
}

impl<PWM> core::fmt::Debug for Error<PWM> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::PwmError(err, _, _) => write!(f, "pwm error: {:?}", err),
        }
    }
}

pub struct WuKongBuzzer<PWM>
where
    PWM: pwm::Instance,
{
    buzzer: Option<pwm::Pwm<PWM>>,
}


fn period(timer_frequency: u32, key: u8) -> u32 {
    let f = 440.0 * powf(2.0, (key as f32 - 69.0) / 12.0);
    let p = timer_frequency as f32 / f;
    truncf(p + 0.5) as u32
}

type Seq = [u16; 1];

struct DmaBuffer(Seq);

impl core::ops::Deref for DmaBuffer {
    type Target = Seq;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl dma::ReadBuffer for DmaBuffer {
    type Word = u16;
    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        (self.0.as_ptr(), self.0.len())
    }
}

impl<PWM> WuKongBuzzer<PWM>
where
    PWM: pwm::Instance,
{
    pub fn new(pwm: PWM, pin: BuzzerPin) -> Self {
        let buzzer = pwm::Pwm::new(pwm);
        buzzer
            // output the waveform on the speaker pin
            .set_output_pin(pwm::Channel::C0, pin.degrade())
            // Prescaler set for 2MHz.
            .set_prescaler(TIMER.scale)
            // Configure for up counter mode.
            .set_counter_mode(pwm::CounterMode::UpAndDown)
            // Read duty cycle values from sequence.
            .set_load_mode(pwm::LoadMode::Common)
            // Enable sample channel.
            .enable_channel(pwm::Channel::C0)
            // Enable sample group.
            .enable_group(pwm::Group::G0)
            // Enable but don't start.
            .enable();
        Self { buzzer: Some(buzzer) }
    }

    pub fn play_note(&mut self, key: u8, duration: u32) {
        let p = period(TIMER.period, key);
        let nloops = duration * TIMER.period / (2 * 1000 * p);
        let pwm = self.buzzer.take().unwrap();
        pwm
            .set_max_duty(p as u16)
            .repeat(nloops as u16)
            // Be sure to be advancing the thing.
            .set_step_mode(pwm::StepMode::Auto)
            // Set no delay between samples.
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            // Set reset delay at end of sequence.
            .set_seq_end_delay(pwm::Seq::Seq0, 0);
        let seq = pwm
            .load(
                Some(DmaBuffer([p as u16 / 2])),
                Some(DmaBuffer([p as u16 / 2])),
                false,
            )
            .unwrap_or_else(|_| panic!());
        seq.reset_event(pwm::PwmEvent::LoopsDone);
        seq.start_seq(pwm::Seq::Seq0);
        loop {
            if seq.is_event_triggered(pwm::PwmEvent::LoopsDone) {
                break;
            }
        }
        let (_, _, pwm) = seq.split();
        self.buzzer = Some(pwm);
    }
}
