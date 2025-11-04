use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};



use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::error::RecorderError;
use crate::storage;
use crate::streams;
use std::sync::mpsc::{channel, Sender, SendError, Receiver, RecvError};
use std::sync::{Arc, Mutex};
use std::io::{self, BufWriter};
use std::fs::{self, File};
//use std::error::Error;

use std::thread::{self, JoinHandle};
//use crate::streams::enums::{StreamError, Command, UiUpdate};
use crate::streams::enums::{StreamError, UiMessage};

struct RunContext {
    commands: Receiver<Command>,
    ui_sender: Sender<UiMessage>,
}

impl RunContext {
    fn update(&mut self) {
    }
}

#[derive(Debug, Clone)]
enum Command {
    LoadDevices(String, String),
    Stop,
}

pub struct ControllerStream {
    thread: Option<JoinHandle<()>>,
    commands: Sender<Command>,
}

impl ControllerStream {
    pub fn new(ui_sender: Sender<UiMessage>) -> ControllerStream {
        let (tx, rx) = channel();
        let run_context = RunContext {
            commands: rx,
            ui_sender,
        };
        let thread = thread::Builder::new()
            .name("controller_stream".to_owned())
            //.spawn(move || run_input(run_context, &mut error_callback))
            .spawn(move || run_input(rx, run_context))
            .unwrap();
        ControllerStream {
            thread: Some(thread),
            commands: tx,
        }
    }
    fn push_command(&self, command: Command) -> Result<(), SendError<Command>> {
        self.commands.send(command)?;
        Ok(())
    }
}

impl Drop for ControllerStream {
    #[inline]
    fn drop(&mut self) {
        if self.push_command(Command::Stop).is_ok() {
            self.thread.take().unwrap().join().unwrap();
        }
    }
}

fn init_context() -> RunContext {
    let host = cpal::default_host();
}

fn run_input(commands: Receiver<Command>, events: Receiver<Event>, mut run_context: RunContext) {
    //let mut commands = run_context.commands.take().expect("run_context] command receiver not found");
    loop {
        match commands.try_recv() {
            Ok(command) => /* TODO */ println!("TODO"),
            // Ok(Command::Break) => break,
            Err(error) => break,
        };

        match events.recv() {
        }







        match commands.recv() {
            Ok(Command::LoadDevices(mic_name, speaker_name)) => record_start(run_context),
            Ok(Command::Stop) => break,
            Err(e) => break,
        };
    }
    //run_context.commands = Some(commands);

    // finalizer: call all stop commands
}


fn get_selected_devices(mic_name: &str, speaker_name: &str) -> (Device, Device) {
    let host = cpal::default_host();
    let mic_device = host.output_devices()
        .expect("no output devices found")
        .find(|x| x.name().map(|y| y == speaker_name).unwrap_or(false)) 
        .expect("output device not found");
    let speaker_device = host.input_devices()
        .expect("no input devices found")
        .find(|x| x.name().map(|y| y == mic_name).unwrap_or(false)) 
        .expect("input device not found");
    (mic_device, speaker_device)
}

fn record_start(context: RunContext) {
}











