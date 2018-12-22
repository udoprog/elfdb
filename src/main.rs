use elfdb::{device::Device, tui, visuals::Visuals};
use failure::{format_err, Error, ResultExt};
use std::path::Path;

fn run<'a, V>(mut visuals: V, initial: Option<&Path>) -> Result<(), Error>
where
    V: Visuals,
{
    let mut device = Device::default();

    if let Some(initial) = initial {
        device.load_path(initial).with_context(|_| {
            format_err!("failed to load program from path `{}`", initial.display())
        })?;
    }

    visuals.setup()?;

    loop {
        if visuals.draw(&mut device)? {
            break;
        }

        device.clear();
        device.step()?;
    }

    visuals.done(&mut device)?;
    Ok(())
}

fn main() -> Result<(), Error> {
    use std::{env, panic, path::PathBuf, process};

    panic::set_hook(Box::new(|p| {
        eprintln!("{}", p);
        process::exit(101);
    }));

    let mut args = env::args();
    args.next();

    let program = match args.next() {
        None => None,
        Some(program) => Some(PathBuf::from(program)),
    };

    run(
        tui::Terminal::new().interactive(),
        program.as_ref().map(|p| p.as_path()),
    )?;
    Ok(())
}
