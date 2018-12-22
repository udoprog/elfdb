use crate::{
    device::Device,
    events::{Event, Events},
    hook::{Action, Hook},
    parser,
    visuals::Visuals,
};
use failure::{bail, Error};
use std::{borrow::Cow, io};
use termion::{
    event::Key,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};
use tui::{
    self,
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
};

type TerminalType = tui::Terminal<TermionBackend<AlternateScreen<RawTerminal<io::Stdout>>>>;

pub enum Message {
    Error(Cow<'static, str>),
    Info(Cow<'static, str>),
    Bold(Cow<'static, str>),
}

impl Message {
    pub fn error(error: impl Into<Cow<'static, str>>) -> Message {
        Message::Error(error.into())
    }

    pub fn info(error: impl Into<Cow<'static, str>>) -> Message {
        Message::Info(error.into())
    }

    pub fn bold(error: impl Into<Cow<'static, str>>) -> Message {
        Message::Bold(error.into())
    }
}

pub struct Terminal {
    interactive: bool,
    hooks: Vec<Hook>,
    events: Events,
    terminal: Option<TerminalType>,
    input: String,
    /// Last command run.
    last: Option<String>,
    messages: Vec<Message>,
    scroll: usize,
    /// If we should use human-readable decoding for instructions.
    human_decoding: bool,
    /// Visualization step when running in non-interactive mode.
    noninteractive_step: usize,
}

impl Terminal {
    pub fn new() -> Self {
        let mut messages = Vec::new();

        Self::help_command(&mut messages);

        Self {
            interactive: false,
            hooks: Vec::new(),
            events: Events::new(),
            terminal: None,
            input: String::new(),
            last: None,
            messages,
            scroll: 0,
            human_decoding: true,
            noninteractive_step: 1_000_000,
        }
    }

    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }

    pub fn hook(mut self, hook: Hook) -> Self {
        self.hooks.push(hook);
        self
    }

    fn help_command(messages: &mut Vec<Message>) {
        messages.push(Message::bold("Commands:"));
        messages.push(Message::info("  help - show this help."));
        messages.push(Message::info("  exit, q - close this session."));
        messages.push(Message::info(
            "  load <path> - load an elfcode program from the given path.",
        ));
        messages.push(Message::info(
            "  reset - reset the device back to its original state.",
        ));
        messages.push(Message::info(
            "  break, b <expr> - break when the given expression holds true.",
        ));
        messages.push(Message::info(
            "    <expr> can be one of: line(<line>), read(<reg>), write(<reg>), not(<expr>),",
        ));
        messages.push(Message::info(
            "    all(<expr1>[, <expr2>]), unique(<reg>), or <op>(<reg>, <value>).",
        ));
        messages.push(Message::info("    <reg> is a registry, like `a` or `ip`."));
        messages.push(Message::info(
            "    <value> is a registry value, like `42` or `100000`.",
        ));
        messages.push(Message::info(
            "    <op> can be one of `eq`, `lt`, `lte`, `gt`, or `gte`.",
        ));
        messages.push(Message::info(
            "  clear, cl [index] - clear breakpoint, if [index] is blank removed the last one.",
        ));
        messages.push(Message::info(
            "  inspect [index] - inspect the state of a breakpoint.",
        ));
        messages.push(Message::info("  step, s - run a single instruction."));
        messages.push(Message::info(
            "  continue, c - continue running in non-interactive mode.",
        ));
        messages.push(Message::info(
            "  set <reg> <value> - set the register <reg> to the given value <value>.",
        ));
        messages.push(Message::bold("Keys:"));
        messages.push(Message::info(
            "  <up>|<down> - scroll the instructions window up and down.",
        ));
        messages.push(Message::info(
            "  <F1> - toggle between original and human decoding of instructions.",
        ));
        messages.push(Message::info("  <q> - quit when in non-interactive mode."));
        messages.push(Message::info("  <p> - pause when in non-interactive mode."));
    }

    /// Internal draw function.
    fn draw_internal(
        terminal: &mut TerminalType,
        interactive: bool,
        human_decoding: bool,
        scroll: &mut usize,
        messages: &mut Vec<Message>,
        hooks: &mut Vec<Hook>,
        input: &mut String,
        device: &Device,
    ) -> Result<(), Error> {
        use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};

        let mut instruction_list = Vec::new();
        let mut register_list = Vec::new();
        let mut breakpoints_list = Vec::new();
        let mut device_list = Vec::new();

        for (line, inst) in device.instructions[*scroll..]
            .iter()
            .enumerate()
            .map(|(i, v)| (i + *scroll, v))
        {
            let standout = device
                .registers
                .last_ip
                .as_ref()
                .map(|ip| line == *ip)
                .unwrap_or(false);

            let l = if human_decoding {
                format!("{:<3}: {}", line, inst.human_display(&device.registers))
            } else {
                format!("{:<3}: {}", line, inst.display())
            };

            if standout {
                let style = Style::default().fg(Color::Black).bg(Color::White);
                instruction_list.push(Text::Styled(l.into(), style));
            } else {
                instruction_list.push(Text::Raw(l.into()));
            }
        }

        for (reg, value) in device.registers.iter().enumerate() {
            let mark = if device.registers.is_read(reg) {
                "*"
            } else {
                " "
            };

            let l = if human_decoding {
                format!("{:<2}{}= {}", device.registers.name(reg), mark, value)
            } else {
                format!("{:<2}{}= {}", reg, mark, value)
            };

            if device.registers.is_written(reg) {
                let style = Style::default().fg(Color::Black).bg(Color::White);
                register_list.push(Text::Styled(l.into(), style));
            } else {
                register_list.push(Text::Raw(l.into()));
            }
        }

        for (index, hook) in hooks.iter().enumerate() {
            breakpoints_list.push(Text::raw(format!(
                "{:<2}: {}",
                index,
                hook.display(&device)
            )));
        }

        device_list.push(Text::raw(format!("Count: {}", device.count)));
        device_list.push(Text::raw(format!("Unique: {}", device.unique.len())));

        terminal.draw(|mut f| {
            let mut constraints = Vec::new();
            constraints.push(Constraint::Min(0));

            if device.halted {
                messages.push(Message::bold("device is halted"));
                messages.push(Message::info("use `reset` to unhalt"));
            }

            if !interactive {
                messages.push(Message::bold(
                    "running in non-interactive mode, press `p` to pause or `q` to quit",
                ));
            }

            for _ in 0..messages.len() {
                constraints.push(Constraint::Length(1));
            }

            if interactive {
                constraints.push(Constraint::Length(1));
            }

            let horizontal = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(f.size());

            let (top, mut horizontal) = match horizontal.split_first() {
                Some(d) => d,
                None => panic!("bad horizontal layout"),
            };

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Percentage(40)].as_ref())
                .split(*top);

            let (left, right) = match layout.as_slice() {
                &[left, right] => (left, right),
                _ => panic!("bad horizontal layout"),
            };

            let title = if human_decoding {
                "Instructions (`F1` for Original)"
            } else {
                "Instructions (`F1` for Human)"
            };

            List::new(instruction_list.into_iter())
                .block(Block::default().borders(Borders::ALL).title(title))
                .render(&mut f, left);

            {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(8),
                            Constraint::Min(0),
                            Constraint::Length(4),
                        ]
                        .as_ref(),
                    )
                    .split(right);

                let (top, middle, bottom) = match layout.as_slice() {
                    &[top, middle, bottom] => (top, middle, bottom),
                    _ => panic!("bad horizontal layout"),
                };

                List::new(register_list.into_iter())
                    .block(Block::default().borders(Borders::ALL).title("Registers"))
                    .render(&mut f, top);

                List::new(breakpoints_list.into_iter())
                    .block(Block::default().borders(Borders::ALL).title("Breakpoints"))
                    .render(&mut f, middle);

                List::new(device_list.into_iter())
                    .block(Block::default().borders(Borders::ALL).title("Device"))
                    .render(&mut f, bottom);
            }

            for message in messages.drain(..) {
                let (current, rest) = match horizontal.split_first() {
                    Some(d) => d,
                    None => panic!("bad horizontal layout"),
                };

                horizontal = rest;

                let (style, m) = match message {
                    Message::Error(m) => (Style::default().fg(Color::Red), m),
                    Message::Info(m) => (Style::default().fg(Color::White), m),
                    Message::Bold(m) => (
                        Style::default()
                            .modifier(Modifier::Underline)
                            .fg(Color::White),
                        m,
                    ),
                };

                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Min(0),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(*current);

                Paragraph::new([Text::Raw(m)].iter())
                    .style(style)
                    .render(&mut f, layout[1]);
            }

            if interactive {
                let (current, _) = match horizontal.split_first() {
                    Some(d) => d,
                    None => panic!("bad horizontal layout"),
                };

                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Min(0),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(*current);

                Paragraph::new([Text::raw(format!("> {}", input))].iter())
                    .style(Style::default().fg(Color::White))
                    .render(&mut f, layout[1]);
            }
        })?;

        Ok(())
    }
}

impl Visuals for Terminal {
    fn setup(&mut self) -> Result<(), Error> {
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = tui::Terminal::new(backend)?;
        terminal.hide_cursor()?;

        self.terminal = Some(terminal);
        Ok(())
    }

    fn done(&mut self, _: &mut Device) -> Result<(), Error> {
        Ok(())
    }

    fn draw(&mut self, device: &mut Device) -> Result<bool, Error> {
        let Terminal {
            ref mut interactive,
            ref mut terminal,
            ref mut input,
            ref mut last,
            ref mut messages,
            ref mut hooks,
            ref mut scroll,
            ref mut human_decoding,
            ref mut noninteractive_step,
            ..
        } = *self;

        let terminal = match terminal.as_mut() {
            Some(terminal) => terminal,
            None => bail!("terminal not configured"),
        };

        let draw = *interactive || device.count % *noninteractive_step == 0;

        // adjust scroll if needed.
        if draw {
            // NB: make sure instruction is visible.
            if let Some(last_ip) = device.registers.last_ip.as_ref().cloned() {
                if last_ip < *scroll {
                    *scroll = last_ip;
                } else {
                    let size = terminal.size()?;
                    let height = (size.height - 4) as usize;

                    if last_ip > *scroll + height {
                        *scroll = last_ip;
                    }
                }
            }
        }

        loop {
            let draw = *interactive || device.count % *noninteractive_step == 0;

            if draw {
                Self::draw_internal(
                    terminal,
                    *interactive,
                    *human_decoding,
                    scroll,
                    messages,
                    hooks,
                    input,
                    device,
                )?;
            }

            if !*interactive {
                // if device is halted, make interactive.
                if device.halted {
                    *interactive = true;
                    continue;
                }

                match self.events.try_next()? {
                    Some(e) => match e {
                        Event::Input(Key::Char('q')) => {
                            return Ok(true);
                        }
                        Event::Input(Key::Char('p')) => {
                            *interactive = true;
                            continue;
                        }
                        e => generic_handle(
                            e,
                            human_decoding,
                            device.instructions.len(),
                            scroll,
                            messages,
                        ),
                    },
                    None => {}
                }

                for b in hooks.iter_mut() {
                    if let Action::Pause = b.test(device)? {
                        *interactive = true;
                    }
                }

                if !*interactive {
                    return Ok(false);
                }

                continue;
            }

            loop {
                match self.events.next()? {
                    Event::Input(Key::Backspace) => {
                        input.pop();
                        break;
                    }
                    Event::Input(Key::Char('\n')) => {
                        if !input.is_empty() {
                            *last = Some(input.clone());
                        }

                        input.clear();

                        let last = match *last {
                            Some(ref last) => last,
                            None => {
                                messages.push(Message::error("no command to re-run"));
                                break;
                            }
                        };

                        let mut it = last.splitn(2, " ");

                        match it.next() {
                            Some("help") => {
                                Self::help_command(messages);
                                break;
                            }
                            Some("reset") => {
                                for h in hooks.iter_mut() {
                                    h.reset();
                                }

                                device.reset();
                                break;
                            }
                            Some("c") | Some("continue") => {
                                if device.halted {
                                    messages
                                        .push(Message::error("can't continue, device is halted!"));
                                    break;
                                }

                                *interactive = false;
                                return Ok(false);
                            }
                            Some("q") | Some("exit") | Some("quit") => {
                                return Ok(true);
                            }
                            Some("s") | Some("step") => {
                                if device.halted {
                                    messages.push(Message::error("can't step, device is halted!"));
                                    break;
                                }

                                return Ok(false);
                            }
                            Some("b") | Some("break") => {
                                let condition = match it.next() {
                                    Some(condition) => condition,
                                    None => {
                                        messages.push(Message::error("missing break condition!"));
                                        break;
                                    }
                                };

                                if let Ok(hook) = parse_hook(device, condition, messages) {
                                    hooks.push(hook);
                                }

                                break;
                            }
                            Some("load") => {
                                let it = it.flat_map(|s| s.split(" "));
                                load_command(device, it, messages);
                                break;
                            }
                            Some("clear") | Some("cl") => {
                                let it = it.flat_map(|s| s.split(" "));
                                clear_command(it, messages, hooks);
                                break;
                            }
                            Some("inspect") => {
                                let it = it.flat_map(|s| s.split(" "));
                                inspect_command(it, messages, hooks);
                                break;
                            }
                            Some("set") => {
                                let it = it.flat_map(|s| s.split(" "));
                                set_command(device, it, messages)?;
                                break;
                            }
                            Some(command) => {
                                messages
                                    .push(Message::error(format!("no such command: {}", command)));
                            }
                            None => {
                                messages.push(Message::error("expected command"));
                                break;
                            }
                        }

                        break;
                    }
                    Event::Input(Key::Char(c)) => {
                        if c == ' ' && input.is_empty() {
                            continue;
                        }

                        input.push(c);
                        break;
                    }
                    Event::Input(Key::Ctrl('d')) => {
                        if input.is_empty() {
                            return Ok(true);
                        }
                    }
                    e => {
                        generic_handle(
                            e,
                            human_decoding,
                            device.instructions.len(),
                            scroll,
                            messages,
                        );
                        break;
                    }
                }
            }
        }

        fn generic_handle(
            e: Event,
            human_decoding: &mut bool,
            len: usize,
            scroll: &mut usize,
            messages: &mut Vec<Message>,
        ) {
            match e {
                Event::Input(Key::Up) => {
                    *scroll = scroll.saturating_sub(1);
                }
                Event::Input(Key::Down) => {
                    *scroll = usize::min(scroll.saturating_add(1), len.saturating_sub(1));
                }
                Event::Input(Key::F(1)) => {
                    *human_decoding = !*human_decoding;
                }
                e => {
                    messages.push(Message::error(format!("unhandled event: {:?}", e)));
                }
            }
        }

        fn parse_hook<'a>(
            device: &Device,
            condition: &str,
            messages: &mut Vec<Message>,
        ) -> Result<Hook, ()> {
            match parser::parse(condition, device) {
                Ok(hook) => Ok(hook),
                Err(e) => {
                    messages.push(Message::error(format!("bad condition: {}", e)));
                    Err(())
                }
            }
        }

        fn load_command<'a>(
            device: &mut Device,
            mut it: impl Iterator<Item = &'a str>,
            messages: &mut Vec<Message>,
        ) {
            match it.next() {
                Some(path) => match device.load_path(path) {
                    Ok(()) => {}
                    Err(e) => {
                        messages.push(Message::error(format!(
                            "problem when loading `{}`: {}",
                            path, e
                        )));
                    }
                },
                _ => {
                    messages.push(Message::error("expected: load <path>"));
                }
            }
        }

        fn clear_command<'a>(
            mut it: impl Iterator<Item = &'a str>,
            messages: &mut Vec<Message>,
            hooks: &mut Vec<Hook>,
        ) {
            let index = match it.next() {
                Some(index) => match str::parse(index) {
                    Ok(index) => index,
                    Err(e) => {
                        messages.push(Message::error(format!("bad index `{}`: {}", index, e)));
                        return;
                    }
                },
                None => {
                    if hooks.is_empty() {
                        messages.push(Message::error("no breakpoints to clear"));
                        return;
                    }

                    hooks.len() - 1
                }
            };

            if index >= hooks.len() {
                messages.push(Message::error(format!("bad hook index `{}`", index)));
                return;
            }

            hooks.remove(index);
        }

        fn inspect_command<'a>(
            mut it: impl Iterator<Item = &'a str>,
            messages: &mut Vec<Message>,
            hooks: &mut Vec<Hook>,
        ) {
            let index = match it.next() {
                Some(index) => match str::parse(index) {
                    Ok(index) => index,
                    Err(e) => {
                        messages.push(Message::error(format!("bad index `{}`: {}", index, e)));
                        return;
                    }
                },
                None => {
                    if hooks.is_empty() {
                        messages.push(Message::error("no breakpoints to clear"));
                        return;
                    }

                    hooks.len() - 1
                }
            };

            match hooks.get(index) {
                Some(hook) => {
                    messages.push(Message::info(hook.inspect().to_string()));
                }
                None => {
                    messages.push(Message::error(format!("no hook with index `{}`", index)));
                }
            }
        }

        fn set_command<'a>(
            device: &mut Device,
            mut it: impl Iterator<Item = &'a str>,
            messages: &mut Vec<Message>,
        ) -> Result<(), Error> {
            let reg = match register(device, it.next(), messages) {
                Some(reg) => reg,
                None => {
                    messages.push(Message::error("expected: set <register> <value>"));
                    return Ok(());
                }
            };

            let value = match it.next() {
                Some(value) => match str::parse(value) {
                    Ok(value) => value,
                    Err(e) => {
                        messages.push(Message::error(format!("bad value `{}`: {}", value, e)));
                        return Ok(());
                    }
                },
                None => {
                    messages.push(Message::error("expected: set <register> <value>"));
                    return Ok(());
                }
            };

            *device.registers.reg_mut(reg)? = value;
            Ok(())
        }

        fn register(
            device: &Device,
            reg: Option<&str>,
            messages: &mut Vec<Message>,
        ) -> Option<usize> {
            let reg = match reg {
                Some("a") => 0,
                Some("b") => 1,
                Some("c") => 2,
                Some("d") => 3,
                Some("e") => 4,
                Some("f") => 5,
                Some("ip") => device.registers.ip,
                Some(o) => {
                    messages.push(Message::error(format!("bad register: {}", o)));
                    return None;
                }
                None => return None,
            };

            Some(reg)
        }
    }
}
