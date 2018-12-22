use crate::{device::Device, Reg};
use failure::Error;
use hashbrown::HashSet;
use std::fmt;

#[derive(Debug)]
pub enum Action {
    Pause,
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Eq,
    Lt,
    Lte,
    Gt,
    Gte,
}

impl Op {
    pub fn test(self, a: Reg, b: Reg) -> bool {
        use self::Op::*;

        match self {
            Eq => a == b,
            Gt => a > b,
            Gte => a >= b,
            Lt => a < b,
            Lte => a <= b,
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Op::*;

        let op = match self {
            Eq => "eq",
            Gt => "gt",
            Gte => "gte",
            Lt => "lt",
            Lte => "lte",
        };

        op.fmt(fmt)
    }
}

#[derive(Debug, Clone)]
pub enum Hook {
    /// Break when the given register is read.
    Read(usize),
    /// Break when the given register is written to.
    Write(usize),
    /// Break when the given line is run.
    Line(usize),
    /// Break when the given register equals the specified value.
    Op(Op, usize, Reg),
    /// Break when a unique value has been observed in the specified registry.
    Unique(HashSet<Reg>, Option<Reg>, usize),
    /// Break when the inverted condition of a hook is true.
    Not(Box<Hook>),
    /// All the criterias listed must match.
    All(Vec<Hook>),
}

impl Hook {
    /// Create a Unique hook for the given register.
    pub fn unique(register: usize) -> Hook {
        Hook::Unique(HashSet::new(), None, register)
    }

    /// Reset the state of a hook.
    pub fn reset(&mut self) {
        use self::Hook::*;

        match *self {
            Unique(ref mut seen, ..) => {
                seen.clear();
            }
            Not(ref mut inner) => {
                inner.reset();
            }
            _ => {}
        }
    }

    pub fn inspect<'a>(&'a self) -> Inspect<'a> {
        Inspect { hook: self }
    }

    /// Test if the breakpoint is valid.
    pub fn test(&mut self, device: &mut Device) -> Result<Action, Error> {
        use self::Hook::*;

        match *self {
            Read(reg) => {
                if device.registers.is_read(reg) {
                    return Ok(Action::Pause);
                }
            }
            Write(reg) => {
                if device.registers.is_written(reg) {
                    return Ok(Action::Pause);
                }
            }
            Line(line) => {
                if device
                    .registers
                    .last_ip
                    .as_ref()
                    .map(|ip| *ip == line)
                    .unwrap_or(false)
                {
                    return Ok(Action::Pause);
                }
            }
            Op(op, reg, value) => {
                if op.test(device.registers.reg(reg)?, value) {
                    return Ok(Action::Pause);
                }
            }
            Unique(ref mut seen, ref mut last, register) => {
                let value = device.registers.reg(register)?;

                if seen.insert(value) {
                    *last = Some(value);
                    return Ok(Action::Pause);
                }
            }
            Not(ref mut inner) => match inner.test(device)? {
                Action::None => return Ok(Action::Pause),
                Action::Pause => return Ok(Action::None),
            },
            All(ref mut hooks) => {
                for h in hooks {
                    if let Action::None = h.test(device)? {
                        return Ok(Action::None);
                    }
                }

                return Ok(Action::Pause);
            }
        }

        Ok(Action::None)
    }

    /// Convert hook into a string.
    pub fn display<'a>(&'a self, device: &'a Device) -> Display<'a> {
        Display {
            hook: self,
            device: device,
        }
    }
}

pub struct Inspect<'a> {
    hook: &'a Hook,
}

impl fmt::Display for Inspect<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Hook::*;

        let Inspect { ref hook } = *self;

        match *hook {
            Read(..) => write!(fmt, "read()"),
            Write(..) => write!(fmt, "write()"),
            Line(..) => write!(fmt, "line()"),
            Op(op, ..) => write!(fmt, "{}()", op),
            Unique(ref seen, ref last, ..) => {
                write!(fmt, "unique(seen: {}, last: {:?})", seen.len(), last)
            }
            Not(ref inner) => write!(fmt, "not({})", inner.inspect()),
            All(hooks) => {
                let mut it = hooks.iter().peekable();

                write!(fmt, "all(")?;

                while let Some(h) = it.next() {
                    h.inspect().fmt(fmt)?;

                    if it.peek().is_some() {
                        write!(fmt, ", ")?;
                    }
                }

                write!(fmt, ")")?;
                Ok(())
            }
        }
    }
}

pub struct Display<'a> {
    hook: &'a Hook,
    device: &'a Device,
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Hook::*;

        let Display {
            ref hook,
            ref device,
        } = *self;

        match *hook {
            Read(reg) => write!(fmt, "read({})", device.registers.name(*reg)),
            Write(reg) => write!(fmt, "write({})", device.registers.name(*reg)),
            Line(line) => write!(fmt, "line({})", line),
            Op(op, reg, value) => {
                write!(fmt, "{}({}, {})", op, device.registers.name(*reg), *value)
            }
            Unique(_, _, reg) => write!(fmt, "unique({})", device.registers.name(*reg)),
            Not(ref inner) => write!(fmt, "not({})", inner.display(device)),
            All(hooks) => {
                let mut it = hooks.iter().peekable();

                write!(fmt, "all(")?;

                while let Some(h) = it.next() {
                    h.display(device).fmt(fmt)?;

                    if it.peek().is_some() {
                        write!(fmt, ", ")?;
                    }
                }

                write!(fmt, ")")?;
                Ok(())
            }
        }
    }
}
