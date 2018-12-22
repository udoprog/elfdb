use crate::{
    device::Device,
    hook::{Hook, Op},
};
use failure::{bail, Error};

#[derive(Debug)]
pub enum Token {
    String(String),
    Immediate(i64),
    Open,
    Close,
    Comma,
}

impl Token {
    fn as_string(self) -> Result<String, Error> {
        match self {
            Token::String(string) => Ok(string),
            other => bail!("expected string, but got: {:?}", other),
        }
    }

    fn as_immediate(self) -> Result<i64, Error> {
        match self {
            Token::Immediate(immediate) => Ok(immediate),
            other => bail!("expected immediate, but got: {:?}", other),
        }
    }

    fn as_register(self, device: &Device) -> Result<usize, Error> {
        let out = match self.as_string()?.as_str() {
            "a" => 0,
            "b" => 1,
            "c" => 2,
            "d" => 3,
            "e" => 4,
            "f" => 5,
            "ip" => device.registers.ip,
            other => bail!("not a register: {}", other),
        };

        Ok(out)
    }

    fn as_open(self) -> Result<(), Error> {
        match self {
            Token::Open => Ok(()),
            _ => bail!("expected string"),
        }
    }

    fn as_close(self) -> Result<(), Error> {
        match self {
            Token::Close => Ok(()),
            _ => bail!("expected string"),
        }
    }

    fn as_comma(self) -> Result<(), Error> {
        match self {
            Token::Comma => Ok(()),
            _ => bail!("expected string"),
        }
    }
}

fn tokenize<'a>(input: &'a str) -> Tokenizer<'a> {
    Tokenizer {
        it: input.chars().peekable(),
    }
}

pub struct Tokenizer<'a> {
    it: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn try_next(&mut self) -> Result<Option<Token>, Error> {
        loop {
            let t = match self.it.peek().cloned() {
                Some(c) => match c {
                    'a'..='z' => return Ok(Some(Token::String(self.string()))),
                    '0'..='9' => return Ok(Some(Token::Immediate(self.immediate()?))),
                    '(' => Token::Open,
                    ')' => Token::Close,
                    ',' => Token::Comma,
                    ' ' => {
                        self.it.next();
                        continue;
                    }
                    c => bail!("unexpected character: {}", c),
                },
                None => return Ok(None),
            };

            self.it.next();
            return Ok(Some(t));
        }
    }

    pub fn next(&mut self) -> Result<Token, Error> {
        match self.try_next()? {
            Some(next) => Ok(next),
            None => bail!("expected token"),
        }
    }

    pub fn as_hook(&mut self, device: &Device) -> Result<Hook, Error> {
        let name = self.next()?.as_string()?;

        match name.as_str() {
            "line" => {
                self.next()?.as_open()?;
                let line = self.next()?.as_immediate()? as usize;
                self.next()?.as_close()?;
                return Ok(Hook::Line(line));
            }
            "read" => {
                self.next()?.as_open()?;
                let reg = self.next()?.as_register(device)?;
                self.next()?.as_close()?;
                return Ok(Hook::Read(reg));
            }
            "write" => {
                self.next()?.as_open()?;
                let reg = self.next()?.as_register(device)?;
                self.next()?.as_close()?;
                return Ok(Hook::Write(reg));
            }
            "unique" => {
                self.next()?.as_open()?;
                let reg = self.next()?.as_register(device)?;
                self.next()?.as_close()?;
                return Ok(Hook::unique(reg));
            }
            "all" => {
                self.next()?.as_open()?;
                let mut hooks = Vec::new();
                hooks.push(self.as_hook(device)?);

                loop {
                    match self.next()? {
                        Token::Comma => {
                            hooks.push(self.as_hook(device)?);
                        }
                        Token::Close => break,
                        _ => bail!("expected comma or close"),
                    }
                }

                return Ok(Hook::All(hooks));
            }
            "not" => {
                self.next()?.as_open()?;
                let inner = self.as_hook(device)?;
                self.next()?.as_close()?;
                return Ok(Hook::Not(Box::new(inner)));
            }
            "gt" | "lt" | "eq" | "gte" | "lte" => {
                self.next()?.as_open()?;
                let reg = self.next()?.as_register(device)?;
                self.next()?.as_comma()?;
                let immediate = self.next()?.as_immediate()?;
                self.next()?.as_close()?;

                let op = match name.as_str() {
                    "gt" => Op::Gt,
                    "lt" => Op::Lt,
                    "eq" => Op::Eq,
                    "gte" => Op::Gte,
                    "lte" => Op::Lte,
                    other => bail!("bad operation: {}", other),
                };

                return Ok(Hook::Op(op, reg, immediate));
            }
            other => bail!("no such function: {}", other),
        }
    }

    fn string(&mut self) -> String {
        let mut buffer = String::new();

        while let Some(c) = self.it.peek().cloned() {
            match c {
                'a'..='z' => {
                    buffer.push(c);
                }
                _ => break,
            }

            self.it.next();
        }

        return buffer;
    }

    fn immediate(&mut self) -> Result<i64, Error> {
        let mut buffer = String::new();

        while let Some(c) = self.it.peek().cloned() {
            match c {
                '0'..='9' => buffer.push(c),
                _ => break,
            }

            self.it.next();
        }

        Ok(str::parse(&buffer)?)
    }
}

pub fn parse(input: &str, device: &Device) -> Result<Hook, Error> {
    return Ok(tokenize(input).as_hook(device)?);
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    pub fn test_parse() {
        use crate::device::Device;
        let device = Device::default();
        parse("all(line(28), line(40))", &device).expect("not parse");
    }
}
