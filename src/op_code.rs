use crate::registers::Registers;
use failure::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OpCode {
    Addr,
    Addi,
    Mulr,
    Muli,
    Banr,
    Bani,
    Borr,
    Bori,
    Setr,
    Seti,
    Gtir,
    Gtri,
    Gtrr,
    Eqir,
    Eqri,
    Eqrr,
}

impl fmt::Display for OpCode {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::OpCode::*;

        let name = match *self {
            Addr => "addr",
            Addi => "addi",
            Mulr => "mulr",
            Muli => "muli",
            Banr => "banr",
            Bani => "bani",
            Borr => "borr",
            Bori => "bori",
            Setr => "setr",
            Seti => "seti",
            Gtir => "gtir",
            Gtri => "gtri",
            Gtrr => "gtrr",
            Eqir => "eqir",
            Eqri => "eqri",
            Eqrr => "eqrr",
        };

        name.fmt(fmt)
    }
}

impl OpCode {
    pub fn decode(input: &str) -> Option<OpCode> {
        use self::OpCode::*;

        let out = match input {
            "addr" => Addr,
            "addi" => Addi,
            "mulr" => Mulr,
            "muli" => Muli,
            "banr" => Banr,
            "bani" => Bani,
            "borr" => Borr,
            "bori" => Bori,
            "setr" => Setr,
            "seti" => Seti,
            "gtir" => Gtir,
            "gtri" => Gtri,
            "gtrr" => Gtrr,
            "eqir" => Eqir,
            "eqri" => Eqri,
            "eqrr" => Eqrr,
            _ => return None,
        };

        Some(out)
    }

    /// Get the infix version of this op code.
    pub fn infix(self) -> &'static str {
        use self::OpCode::*;

        match self {
            Addr => "+",
            Addi => "+",
            Mulr => "*",
            Muli => "*",
            Banr => "&",
            Bani => "&",
            Borr => "|",
            Bori => "|",
            Setr => "?",
            Seti => "?",
            Gtir => ">",
            Gtri => ">",
            Gtrr => ">",
            Eqir => "==",
            Eqri => "==",
            Eqrr => "==",
        }
    }

    /// Apply the given operation to the registers.
    pub fn apply(&self, r: &mut Registers, inputs: &[i64; 2], o: i64) -> Result<(), Error> {
        use self::OpCode::*;

        let [a, b] = *inputs;

        *r.reg_mut(o)? = match *self {
            Addr => r.reg(a)? + r.reg(b)?,
            Addi => r.reg(a)? + b,
            Mulr => r.reg(a)? * r.reg(b)?,
            Muli => r.reg(a)? * b,
            Banr => r.reg(a)? & r.reg(b)?,
            Bani => r.reg(a)? & b,
            Borr => r.reg(a)? | r.reg(b)?,
            Bori => r.reg(a)? | b,
            Setr => r.reg(a)?,
            Seti => a,
            Gtir => {
                if a > r.reg(b)? {
                    1
                } else {
                    0
                }
            }
            Gtri => {
                if r.reg(a)? > b {
                    1
                } else {
                    0
                }
            }
            Gtrr => {
                if r.reg(a)? > r.reg(b)? {
                    1
                } else {
                    0
                }
            }
            Eqir => {
                if a == r.reg(b)? {
                    1
                } else {
                    0
                }
            }
            Eqri => {
                if r.reg(a)? == b {
                    1
                } else {
                    0
                }
            }
            Eqrr => {
                if r.reg(a)? == r.reg(b)? {
                    1
                } else {
                    0
                }
            }
        };

        Ok(())
    }
}
