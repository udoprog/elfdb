use std::{io, sync::mpsc, thread};

use termion::{event::Key, input::TermRead};

#[derive(Debug)]
pub enum Event {
    Input(Key),
}

/// An small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event>,
    #[allow(unused)]
    input_handle: thread::JoinHandle<()>,
}

impl Events {
    pub fn new() -> Events {
        let (tx, rx) = mpsc::channel();

        let input_handle = {
            let tx = tx.clone();

            thread::spawn(move || {
                let stdin = io::stdin();

                for evt in stdin.keys() {
                    match evt {
                        Ok(key) => {
                            if let Err(_) = tx.send(Event::Input(key)) {
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        };

        Events { rx, input_handle }
    }

    pub fn try_next(&self) -> Result<Option<Event>, mpsc::RecvError> {
        match self.rx.try_recv() {
            Ok(e) => Ok(Some(e)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(mpsc::RecvError),
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
