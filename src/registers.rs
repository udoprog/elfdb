use crate::{AsReg, Reg};
use failure::{bail, Error};
use hashbrown::HashSet;
use std::fmt;

#[derive(Debug, Default)]
pub struct Registers {
    registers: [Reg; 6],
    /// Written to registers.
    written: HashSet<usize>,
    /// Read from registers.
    read: HashSet<usize>,
    /// Last instruction that was executed.
    pub last_ip: Option<usize>,
    /// Which register contains the current instruction.
    pub ip: usize,
}

impl Registers {
    /// Test if the given registry has been written to.
    pub fn is_written(&self, reg: impl AsReg) -> bool {
        self.written.contains(&reg.as_reg())
    }

    /// Test if the given registry has been read from.
    pub fn is_read(&self, reg: impl AsReg) -> bool {
        self.read.contains(&reg.as_reg())
    }

    /// Access the given register immutably.
    pub fn reg(&mut self, reg: impl AsReg) -> Result<Reg, Error> {
        let index = reg.as_reg();

        self.read.insert(index);

        match self.registers.get(index).cloned() {
            Some(reg) => Ok(reg),
            None => bail!("no such register: {}", index),
        }
    }

    /// Access the given register mutably.
    pub fn reg_mut(&mut self, reg: impl AsReg) -> Result<&mut i64, Error> {
        let index = reg.as_reg();

        self.written.insert(index);

        match self.registers.get_mut(index) {
            Some(reg) => Ok(reg),
            None => bail!("no such register: {}", index),
        }
    }

    pub fn ip(&self) -> Result<usize, Error> {
        match self.registers.get(self.ip) {
            Some(reg) => Ok(*reg as usize),
            None => bail!("no ip register: {}", self.ip),
        }
    }

    pub fn ip_mut(&mut self) -> Result<&mut Reg, Error> {
        match self.registers.get_mut(self.ip) {
            Some(reg) => Ok(reg),
            None => bail!("no ip register: {}", self.ip),
        }
    }

    /// Iterate over all registers.
    pub fn iter(&self) -> impl Iterator<Item = Reg> + '_ {
        self.registers.iter().cloned()
    }

    /// Clear all state for the device.
    pub fn clear(&mut self) {
        self.written.clear();
        self.read.clear();
    }

    pub fn reset(&mut self) {
        self.written.clear();
        self.read.clear();
        self.last_ip = None;

        for r in self.registers.iter_mut() {
            *r = Default::default();
        }
    }

    pub fn name(&self, reg: impl AsReg) -> RegName {
        let reg = reg.as_reg();

        let special = if reg == self.ip { Some("ip") } else { None };

        RegName {
            special,
            name: ['a', 'b', 'c', 'd', 'e', 'f'].get(reg).cloned(),
        }
    }
}

pub struct RegName {
    special: Option<&'static str>,
    name: Option<char>,
}

impl fmt::Display for RegName {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(special) = self.special.as_ref() {
            return special.fmt(fmt);
        }

        if let Some(name) = self.name {
            return name.fmt(fmt);
        }

        "?".fmt(fmt)
    }
}
