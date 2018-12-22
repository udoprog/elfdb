use crate::{op_code::OpCode, registers::Registers};
use std::fmt;

/// An instruction.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub op_code: OpCode,
    pub inputs: [i64; 2],
    pub output: i64,
}

impl Instruction {
    pub fn decode(state: &str) -> Option<Instruction> {
        let mut it = state.split(" ");
        let op_code = OpCode::decode(it.next()?)?;
        let mut it = it.flat_map(|d| str::parse(d).ok());

        Some(Instruction {
            op_code,
            inputs: [it.next()?, it.next()?],
            output: it.next()?,
        })
    }

    /// Display this instruction.
    pub fn display<'a>(&'a self) -> Display<'a> {
        Display { inst: self }
    }

    /// Provide a human-readable display implementation for this instruction.
    pub fn human_display<'a>(&'a self, registers: &'a Registers) -> HumanDisplay<'a> {
        HumanDisplay {
            inst: self,
            registers,
        }
    }
}

pub struct Display<'a> {
    inst: &'a Instruction,
}

impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = self.inst.op_code;
        let [a, b] = self.inst.inputs;
        let o = self.inst.output;
        write!(fmt, "{} {} {} {}", op, a, b, o)
    }
}

pub struct HumanDisplay<'a> {
    inst: &'a Instruction,
    registers: &'a Registers,
}

impl<'a> fmt::Display for HumanDisplay<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::OpCode::*;

        let op = self.inst.op_code;
        let [a, b] = self.inst.inputs;
        let o = self.inst.output;
        let r = self.registers;

        match op {
            Addr | Mulr | Banr | Borr | Gtrr | Eqrr => write!(
                fmt,
                "{:<2} = {} {} {}",
                r.name(o),
                r.name(a),
                op.infix(),
                r.name(b)
            ),
            Addi | Muli | Bani | Bori | Gtri | Eqri => {
                write!(fmt, "{:<2} = {} {} {}", r.name(o), r.name(a), op.infix(), b)
            }
            Gtir | Eqir => write!(fmt, "{:<2} = {} {} {}", r.name(o), a, op.infix(), r.name(b)),
            Setr => write!(fmt, "{:<2} = {}", r.name(o), r.name(a)),
            Seti => write!(fmt, "{:<2} = {}", r.name(o), a),
        }
    }
}
