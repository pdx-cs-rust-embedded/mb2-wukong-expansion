use crate::*;
use super::*;

pub enum Motor {
    M1,
    M2,
}

impl<TWIM, I2cDelay> WuKongBus<TWIM, I2cDelay>
where
    TWIM: twim::Instance,
    I2cDelay: delay::DelayNs,
{
    pub fn set_motor_speed(&mut self, motor: Motor, speed: i8) -> Result<(), twim::Error> {
        let motor = match motor {
            Motor::M1 => 1,
            Motor::M2 => 2,
        };
        let sign = if speed >= 0 { 1 } else { 2 };
        // XXX Fixme: Return error on overspeed.
        let speed = speed.abs().min(100);
        let buf = [motor, sign, speed as u8, 0];
        self.i2c.write(Self::I2C_ADDR, &buf)
    }
}
