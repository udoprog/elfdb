pub mod device;
mod events;
pub mod hook;
pub mod instruction;
pub mod op_code;
mod parser;
mod registers;
pub mod tui;
pub mod visuals;

pub type Reg = i64;

/// Convert into a registers.
pub trait AsReg {
    fn as_reg(self) -> usize;
}

impl AsReg for usize {
    fn as_reg(self) -> usize {
        self
    }
}

impl AsReg for i32 {
    fn as_reg(self) -> usize {
        self as usize
    }
}

impl AsReg for i64 {
    fn as_reg(self) -> usize {
        self as usize
    }
}
