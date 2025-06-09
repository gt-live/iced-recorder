//https://book.iced.rs/additional-resources.html
use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use iced::widget::{ Button, button, Column, column, container, PickList, pick_list, row, text, Text };
use iced::{ Length, Task, Size, window, Renderer };
use iced::Theme;

use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path;
use rfd::FileDialog;
use mp3lame_encoder;

use iced_recorder::streams;
//use iced_recorder::controller::{self, StreamCommand, WavWriterHandle};
use iced_recorder::storage;
use iced_recorder::error::RecorderError;
//use crate::storage;
//use crate::controller;
//use crate::stream;
//use crate::error::RecorderError;

//use std::thread::{self, JoinHandle};

//use thiserror::Error;

//mod startup;
//use crate::startup::{get_host};


// TODO:
// 1. specify file save to location
//    https://docs.rs/rfd/latest/rfd/
//    https://docs.rs/rfd/latest/rfd/struct.FileDialog.html
//    https://github.com/iced-rs/iced/blob/master/examples/editor/src/main.rs
//    show default, let people change location if they want to
// 1b. show only parent dir + file name
// 2. compress to mp3
//    https://github.com/DoumanAsh/mp3lame-encoder
//    https://docs.rs/libopusenc/latest/libopusenc/https://docs.rs/libopusenc/latest/libopusenc/
// 2b. write to multiple buffer checkpoints then finalize together?
// 3. counter for duration recorded and file buffer size
// 4. popup textbox for errors

pub const WINDOW_INITIAL_WIDTH: f32 = 170.0;
pub const WINDOW_INITIAL_HEIGHT: f32 = 310.0;

fn main() -> Result<(), iced::Error> {


    println!("Hello, world!");
    iced::application(Recorder::title, Recorder::update, Recorder::view)
        .theme(|_| Theme::Dark)
        //.centered()
        .window(window::Settings {
            size: Size::new(WINDOW_INITIAL_WIDTH, WINDOW_INITIAL_HEIGHT),
            ..window::Settings::default()
        })
        .run()
}

//#[derive(Clone)]
#[derive(Debug, Clone)]
enum Message {
    Start,
    Stop,
    SettingsSelectInputDevice(String),
    SettingsSelectOutputDevice(String),
    SettingsSelectFilePath,
    Error(RecorderError),
}


// https://github.com/iced-rs/iced/pull/2331
// Task<Message> replaces Command<Message> (the async think)
enum State {
    Start,
    Stop,
}

struct Recorder {
    host: cpal::Host,
    input_device: cpal::Device,
    output_device: cpal::Device,
    state: State,

    // configs
    recording_path: path::PathBuf,

    //input_sender: Option<mpsc::Sender<StreamCommand<f32>>>,
    //output_sender: Option<mpsc::Sender<StreamCommand<f32>>>,
    input_stream: Option<cpal::Stream>,
    output_stream: Option<cpal::Stream>,
    control_thread: Option<streams::Controller>,
    //control_thread: Option<thread::JoinHandle<Result<bool, RecorderError>>>,
    //wav_writer: Option<Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>>,
    //wav_writer: Option<WavWriterHandle>,
    wav_writer: Option<streams::WriterStream>,

}

impl Default for Recorder {
    fn default() -> Self {
        let host = cpal::default_host();
        let input_device = host.default_input_device()
            .expect("failed to find input device");
        let output_device = host.default_output_device()
            .expect("failed to find output device");

        //const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");

        Self {
            host,
            input_device,
            output_device,
            state: State::Stop,
            recording_path: path::PathBuf::from(PATH),

            //input_sender: None,
            //output_sender: None,
            input_stream: None,
            output_stream: None,
            control_thread: None,
            wav_writer: None,
        }
    }
}

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");

impl Recorder {
    //fn new(_flags: ()) -> (Self, Task<Message>) {
    //    (Self {
    //        state: State::Stop,
    //    }, Task::none())
    //}

    fn title(&self) -> String {
        "A naive two stream rusty recorder".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Start => {
                if let Err(e) = self.record_start() {
                    return Task::done(Message::Error(e))
                }
                self.state = State::Start;
                Task::none()
            }
            Message::Stop => {
                self.state = State::Stop;
                if let Err(e) = self.record_stop() {
                    return Task::done(Message::Error(e))
                }
                Task::none()
            }
            Message::SettingsSelectInputDevice(device) => {
                if let Some(device) = self.host.input_devices()
                    .unwrap() // should be fine right? else the conditional wont run
                    .find(|x| x.name().map(|y| y == device).unwrap_or(false)) 
                {
                    self.input_device = device
                    // TODO: code that pauses, creates new stream and continues running
                };
                Task::none()
            },
            Message::SettingsSelectOutputDevice(device) => {
                if let Some(device) = self.host.output_devices()
                    .unwrap() // should be fine right? else the conditional wont run
                    .find(|x| x.name().map(|y| y == device).unwrap_or(false)) 
                {
                    self.output_device = device
                    // TODO: code that pauses, creates new stream and continues running
                };
                Task::none()
            },
            Message::SettingsSelectFilePath => {
                let fd = FileDialog::new()
                    .set_title("where to save recording to")
                    .set_file_name("recording.wav")
                    .save_file();
                if let Some(path) = fd {
                    self.recording_path = path;
                }
                Task::none()
            },
            Message::Error(e) => {
                println!("{}", e);
                Task::none()
            }
        }
    }

    fn view(&self) -> Column<Message> {
        // TODO: add droplist for devices

        let record_button = gen_record_button(&self.state);
        let record_button = iced::Element::new(record_button).explain(iced::Color::BLACK);

        let input_list: PickList<'_, String, Vec<String>, String, Message, Theme, Renderer> = pick_list(
            self.host.input_devices()
                .unwrap()
                //.unwrap_or([])
                .map(|x| x.name().unwrap_or("EMPTY".to_string()))
                .collect::<Vec<_>>(),
            Some(self.input_device.name()
                .unwrap_or("EMPTY".to_string())
            ), 
            Message::SettingsSelectInputDevice); 
        let output_list: PickList<'_, String, Vec<String>, String, Message, Theme, Renderer> = pick_list(
            self.host.output_devices()
                .unwrap()
                .map(|x| x.name().unwrap_or("EMPTY".to_string()))
                .collect::<Vec<_>>(),
            Some(self.output_device.name()
                .unwrap_or("EMPTY".to_string())
            ), 
            Message::SettingsSelectOutputDevice); 
        let input_list = container(input_list)
            .width(Length::FillPortion(1));
        let output_list = container(output_list)
            .width(Length::FillPortion(1));
        let browse_button: Button<'_, Message> = button("browse").on_press(Message::SettingsSelectFilePath);
        let browse_button = container(browse_button)
            .width(Length::FillPortion(1));
        let browse_path: Text<'_, Theme, Renderer> = text(self.recording_path.as_path().to_str().unwrap_or(""));
        let browse_path = container(browse_path)
            .width(Length::FillPortion(3));
        let path_row = row![browse_path, browse_button];
        let control_bar = row![input_list, output_list];
        let interface = column![control_bar, path_row, record_button];
        interface
    }


    fn record_start(&mut self) -> Result<(), RecorderError> {
        // The WAV file we're recording to.
        //const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");

        let input_config = self.input_device
            .default_input_config()
            .map_err(|error| RecorderError::StreamConfigError(error.to_string()))?;
        let output_config = self.output_device
            .default_output_config()
            .map_err(|error| RecorderError::StreamConfigError(error.to_string()))?;
            //.expect("Failed to get default output config");
        //println!("Default output config: {:?}", output_config);

        let spec = streams::wav_spec_from_configs(&input_config, &output_config)?;
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
        let writer = streams::WriterStream::new(spec, self.recording_path.clone(), err_fn);
        //let writer = Arc::new(Mutex::new(Some(writer)));
        let writer_sender = writer.get_sender();
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
        let controller = streams::Controller::new(writer_sender, err_fn);
        let (mut sender_speaker, mut sender_mic) = controller.get_senders();


        //// 1. Create channel to for stream_a
        //let (ti, ri) = mpsc::channel();
        //self.input_sender = Some(ti.clone());
        //// 2. Create channel to for stream_b
        //let (to, ro) = mpsc::channel();
        //self.output_sender = Some(to.clone());

        //// Run the input stream on a separate thread.
        //let writer_2 = writer.clone();

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        let input_stream = match input_config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.input_device.build_input_stream(
                    &input_config.into(),
                    //move |data, _| send_frame::<f32>(data, ti_2.clone()),
                    //move |data, _| controller::send_frame::<f32>(data, ti.clone()),
                    move |data, _| sender_mic(data.to_vec()),
                    err_fn,
                    None,
                ).map_err(|error| RecorderError::BuildStreamError(error.to_string()))?
            },
            sample_format =>  return Err(RecorderError::UnsupportedSampleFormat(sample_format, "input".to_string())),
        };

        let output_stream = match output_config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.output_device.build_input_stream(
                    &output_config.into(),
                    //move |data, _| send_frame::<f32>(data, ti.clone()),
                    //move |data, _| controller::send_frame::<f32>(data, to.clone()),
                    move |data, _| sender_speaker(data.to_vec()),
                    err_fn,
                    None,
                ).map_err(|error| RecorderError::BuildStreamError(error.to_string()))?
            },
            sample_format =>  return Err(RecorderError::UnsupportedSampleFormat(sample_format, "output".to_string())),
        };
        //// TODO: set thread to state
        //let thread = thread::Builder::new()
        //    .name("cpal_wasapi_in".to_owned())
        //    .spawn(move || controller::write_frame(ro, ri, &writer_2))
        //    .map_err(|error| RecorderError::ThreadControlThreadFailedToSpawn(error.to_string()))?;

        input_stream.play().map_err(|error| RecorderError::PlayStreamError(error.to_string()))?;
        output_stream.play().map_err(|error| RecorderError::PlayStreamError(error.to_string()))?;
        //writer.play();
        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        //self.control_thread = Some(thread);
        self.control_thread = Some(controller);
        self.wav_writer = Some(writer);

        Ok(())
    }

    fn record_stop(&mut self) -> Result<(), RecorderError> {
        // this would have sent to controller_thread right?
        //if let Some(tx) = &self.input_sender {
        //    tx.send(StreamCommand::Stop).map_err(|error| RecorderError::StreamStopError(error.to_string()))?;
        //};
        //if let Some(tx) = &self.output_sender {
        //    tx.send(StreamCommand::Stop).map_err(|error| RecorderError::StreamStopError(error.to_string()))?;
        //};
        drop(self.input_stream.take());
        drop(self.output_stream.take());
        drop(self.control_thread.take()); //should cascade?
        drop(self.wav_writer.take());
        //let join_handle = match self.control_thread.take() {
        //    Some(join_handle) => join_handle,
        //    None => return Err(RecorderError::StateJoinHandleNotFound),
        //};
        //let _ = join_handle.join().map_err(|_| RecorderError::ThreadControlThreadFailedToJoin)?;

        //let writer = match &self.wav_writer {
        //    Some(writer) => writer,
        //    None => return Err(RecorderError::WavWriterNotFoundError),
        //};
        //drop(writer);
        self.recording_path = path::PathBuf::from(PATH);
        //println!("Recording {} complete!", PATH);
        Ok(())
    }
} 

fn gen_record_button<'a>(state: &State) -> Button<'a, Message> {
    let button = match state {
        State::Start => {
            button("Stop [Rec]").on_press(Message::Stop)
        },
        State::Stop => {
            button("Start [Rec]").on_press(Message::Start)
        },
    };
    button
}




////////////////// util //////////////
fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

