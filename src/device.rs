use crate::{instruction::Instruction, registers::Registers};
use failure::{bail, format_err, Error};
use hashbrown::HashSet;
use std::path;

#[derive(Debug, Default)]
pub struct Device {
    /// If the device is halted.
    pub halted: bool,
    /// Loaded instructions.
    pub instructions: Vec<Instruction>,
    pub registers: Registers,
    /// Count of number of instructions that has been executed.
    pub count: usize,
    /// Unique instructions that has been run.
    pub unique: HashSet<usize>,
}

impl Device {
    /// Load a program from the specified path.
    pub fn load_path(&mut self, path: impl AsRef<path::Path>) -> Result<(), Error> {
        use std::{fs::File, io::Read};

        let mut f = File::open(path.as_ref())?;
        let mut input = String::new();
        f.read_to_string(&mut input)?;
        self.load(input.lines())
    }

    /// Load a program.
    pub fn load<'a>(&mut self, input: impl Iterator<Item = &'a str>) -> Result<(), Error> {
        self.reset();
        self.instructions.clear();

        for line in input {
            if line.starts_with("#ip") {
                let ip = line
                    .split(" ")
                    .nth(1)
                    .ok_or_else(|| format_err!("expected argument to `#ip`"))
                    .and_then(|arg| {
                        str::parse(arg).map_err(|e| format_err!("bad argument to `#ip`: {}", e))
                    })?;

                self.registers.ip = ip;
                continue;
            }

            let inst = match Instruction::decode(line) {
                Some(inst) => inst,
                None => {
                    bail!("bad instruction: {}", line);
                }
            };

            self.instructions.push(inst);
        }

        Ok(())
    }

    pub fn step(&mut self) -> Result<(), Error> {
        let Device {
            ref instructions,
            ref mut registers,
            ..
        } = *self;

        let ip = registers.ip()?;

        let inst = match instructions.get(ip) {
            Some(inst) => inst,
            None => {
                self.halted = true;
                return Ok(());
            }
        };

        registers.last_ip = Some(registers.ip()?);
        inst.op_code.apply(registers, &inst.inputs, inst.output)?;
        *registers.ip_mut()? += 1;

        self.unique.insert(ip);
        self.count += 1;
        Ok(())
    }

    /// Clear all temporary state for the device.
    ///
    /// Temporary state keeps track of things that has been modified.
    pub fn clear(&mut self) {
        self.registers.clear();
    }

    pub fn reset(&mut self) {
        self.halted = false;
        self.count = 0;
        self.unique.clear();
        self.registers.reset();
    }
}
