use crate::device::Device;
use failure::Error;

pub trait Visuals {
    fn setup(&mut self) -> Result<(), Error>;

    fn done(&mut self, device: &mut Device) -> Result<(), Error>;

    /// Returns `false` if the debugger should continue running.
    /// `true` will cause the debugger to exit.
    fn draw(&mut self, _: &mut Device) -> Result<bool, Error>;
}

pub struct NoopVisuals;

impl Visuals for NoopVisuals {
    fn setup(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn done(&mut self, _: &mut Device) -> Result<(), Error> {
        Ok(())
    }

    fn draw(&mut self, _: &mut Device) -> Result<bool, Error> {
        Ok(false)
    }
}
