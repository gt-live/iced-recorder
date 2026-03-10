use std::mem;
use std::thread;
use std::path;
use std::sync::mpsc::{channel, Sender, SendError, Receiver, RecvError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::settings::{self, DeviceSelection, InputDeviceSelection, OutputDeviceSelection};
use crate::streams;
use crate::storage;
use crate::runtime::{self, ContextHead, ContextHeadBuilder, ContextTail, ContextTailBuilder, ContextTailBuilderArgs, RuntimeError};

// move all to use RuntimeEvent
#[derive(Debug, Clone)]
enum Command {
    LoadDevices(String, String),
    //LoadDevices(String, String),
    LoadDefaultDevices,
    Stop,
}

//#[derive(Debug, Clone)]
//pub enum runtime::Event {
//    LoadDevices(String, String),
//    Stop,
//}

pub struct Runtime {
    // this comes from iced
    //commands: Receiver<Command>,
    // this comes from children
    //events: Receiver<runtime::Event>,
    events: Option<Receiver<runtime::Event>>,
    controller: Option<RuntimeController>,
}

// merge run and new?
impl Runtime {
    pub fn new(
        ui_sender: Sender<streams::UiUpdate>,
        writer_path: path::PathBuf,
        //writer_spec: storage::Mp3Spec,
        //writer_sender: Sender<streams::Command>,
    ) -> Result<Runtime, RuntimeError> {
        let (events_sender, events_receiver) = channel();
        //let (commands_sender, commands_receiver) = channel();
        let (write_sender, write_recv) = channel();


        //let context_tail = ContextTailBuilder::default()
        //    .set_write_path(writer_path)
        //    .set_write_spec(writer_spec)
        //    .try_build()
        //    .map_err(|e| RuntimeError::ContextBuildError(e.to_string()))?;
        //let writer_sender = context_tail.get_writer_sender();

        // in the future i should probably not use Builder pattern if my default isnt good enough
        // i want to let the type system guide, and not catch this at runtime
        let context_head = ContextHeadBuilder::default()
            .set_app_sender(ui_sender.clone())
            .set_events_sender(events_sender.clone())
            .set_writer_sender(write_sender.clone())
            .try_build()
            .map_err(|e| RuntimeError::ContextHeadBuildError(e.to_string()))?;
        let mic_config = context_head.get_mic_config();
        let speaker_config = context_head.get_speaker_config();
        let combined_config = streams::wav_spec_from_configs(&mic_config, &speaker_config)
            //.map_err()?;
            .unwrap();

        let context_tail = ContextTail::new(ContextTailBuilderArgs{
            write_path: writer_path,
            write_spec: combined_config,
            write_recv,
        });

        Ok(Runtime {
            controller: Some(RuntimeController {
                context_head,
                context_tail,
                events_sender,
                ui_sender,
                writer_sender: write_sender, // i should standardize the naming
            }),
            events: Some(events_receiver),
        })
    }

    pub fn run(&mut self) {
        // spawn a thread?
        let controller = self.controller.take().unwrap(); // TODO
        let events = self.events.take().unwrap(); // TODO
        let runner = thread::spawn(move || {
            //self.controller.init();
            controller.init();
            loop {
                //match self.commands.try_recv() {
                //    Ok(_) => println!("TODO"),
                //    Err(_) => break,
                //}

                //match self.events.recv() {
                match events.recv() {
                    Ok(runtime::Event::Stop) => break,
                    Ok(event) => { 
                        //if let Err(e) = self.controller.update(event) {
                        if let Err(e) = controller.update(event) {
                            eprintln!("Runtime run: {}", e);
                        };
                    },
                    Err(_) => break, // TODO: handle this better or print msg
                }
            }
            // cleanup
            return
        });
    }
}

pub(crate) struct RuntimeController {
    // consider packaging these into a Head, that can be swapped out as needed
    context_head: ContextHead,
    context_tail: ContextTail,

    ////mic_stream: Option<cpal::Stream>,
    ////speaker_stream: Option<cpal::Stream>,
    //pub(crate) mic_stream: cpal::Stream,
    //pub(crate) speaker_stream: cpal::Stream,
    //pub(crate) bridge_thread: Option<streams::Controller>,
    ////pub(crate) ui_thread: Option<task::Handle>,
    events_sender: Sender<runtime::Event>,
    ui_sender: Sender<streams::UiUpdate>,
    writer_sender: Sender<streams::Command>,

    //ui_receiver: Option<Receiver<streams::UiUpdate>>,
}

// to create, use the builder
impl RuntimeController {
    //fn new(
    //    context_head: ContextHead,
    //    events_sender: Sender<runtime::Event>,
    //    ui_sender: Sender<streams::UiUpdate>,
    //    writer_sender: Sender<streams::Command>,
    //) -> RuntimeController {
    //    RuntimeController {
    //        context_head,
    //        events_sender,
    //        ui_sender,
    //        writer_sender,
    //    }
    //}
    fn init(&mut self) -> Result<(), RuntimeError> {
        self.context_head.play()?;
        Ok(())
    }
    //fn process_commands(&mut self, command: Command) {
    //    match command {
    //        Command::LoadDefaultDevices => ,
    //        Command::LoadDevices(mic_name, speaker_name) => ,
    //    }
    //}
    fn update(&mut self, event: runtime::Event) -> Result<(), RuntimeError> {
        match event {
            runtime::Event::AudioDeviceNotAvailable => self.load_default_devices(),
            runtime::Event::LoadDevices(input, output) => self.load_selected_devices(input, output),
            runtime::Event::LoadPath(path) => self.load_new_path(path),
            runtime::Event::AudioDeviceUnspecifiedError(error) => {
                eprintln!("AudioDeviceUnspecifiedError: {}", error);
                self.load_default_devices()
            },
            //runtime::Event::Stop => self.stop_recording(),
            _ => return Ok(()),
        }
    }

    fn load_new_path(&mut self, path: path::PathBuf) -> Result<(), RuntimeError> {
        let mut context_tail = ContextTailBuilder::default()
            .set_write_path(path)
            //.set_write_path(writer_path)
            .set_write_spec(self.context_tail.get_writer_spec())
            .try_build()
            .map_err(|e| RuntimeError::ContextBuildError(e.to_string()))?;
        // get builder from
        let writer_sender = context_tail.get_writer_sender();
        let mut context_head = match self.context_head.get_builder()
            .set_writer_sender(writer_sender)
            .try_build() {
            Ok(builder) => builder,
            Err(e) => return Err(RuntimeError::ContextBuildError(e.to_string())),
        };
        mem::swap(&mut self.context_head, &mut context_head);
        mem::swap(&mut self.context_tail, &mut context_tail);
        self.context_head.play()?;
        Ok(())
    }

    fn load_selected_devices(&mut self, input: InputDeviceSelection, output: OutputDeviceSelection) -> Result<(), RuntimeError> {
        let input = match input {
            settings::DeviceSelection::Default => return self.load_default_devices(),
            settings::DeviceSelection::Selected(input) => input,
        };
        let output = match output {
            settings::DeviceSelection::Default => return self.load_default_devices(),
            settings::DeviceSelection::Selected(output) => output,
        };
        self.load_named_devices(&input, &output)
    }

    // make sure everything ContextHead is cleaned up by drop
    fn _load_devices(&mut self, builder: ContextHeadBuilder) -> Result<(), RuntimeError> {
        let mut context_head = builder
            .set_app_sender(self.ui_sender.clone())
            .set_events_sender(self.events_sender.clone())
            .set_writer_sender(self.writer_sender.clone())
            .try_build()
            .map_err(|e| RuntimeError::ContextHeadBuildError(e.to_string()))?;
        mem::swap(&mut self.context_head, &mut context_head);
        Ok(())
    }
    fn load_named_devices(&mut self, mic_name: &str, speaker_name: &str) -> Result<(), RuntimeError> {
        let builder = ContextHeadBuilder::default()
            .set_devices(mic_name, speaker_name);
        self._load_devices(builder)
    }
    fn load_default_devices(&mut self) -> Result<(), RuntimeError> {
        let builder = ContextHeadBuilder::default();
        self._load_devices(builder)
    }
    // should not even run
    //fn stop_recording(&mut self) {
    //    self.events_sender.send(runtime::Event::Stop);
    //    // in record_stop i would drop io_streams, control, write
    //    // but i cant take them here, so will need to drop the entire 
    //    // runtime
    //}
}



// io --> bridge        => 1. Frame(Vec<f32>)
// io --> controller    => 1. DeviceUnsyncError # destroy io and recreate (err_fn)
// bridge --> ui -> app => 1. Decibels
// app --> controller   => 1. SwapDevice        # destroy io and recreate (user change io device)
// 
