//https://book.iced.rs/additional-resources.html
use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use iced::widget::{ Button, button, Column, column, container, PickList, pick_list, row, text, Text };
use iced::{ Length, Task, Size, window, Renderer };
use iced::Theme;

use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::fs::File;
use std::io::BufWriter;
use std::path;
use rfd::FileDialog;

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

// TODO: see how other applications do the error handling
#[derive(thiserror::Error, Debug, Clone)]
enum RecorderError {
    #[error("unable to get config from stream device: {0}")]
    StreamConfigError(String),
    //StreamConfigError(#[from] cpal::DefaultStreamConfigError),
    #[error("Merging of streams with differing sample rate currently not supported `{0}`, `{1}`")]
    UnsupportedSampleRate(u32, u32),
    #[error("Merging of streams with differing sample size currently not supported `{0}`, `{1}`")]
    UnsupportedSampleSize(usize, usize),

    #[error("Unable to create file at path: {0}")]
    FileCreationError(String),
    //FileCreationError(#[from] hound::Error),
    //FileCreationError(#[from] hound::Error, String)
    #[error("Unsupported {1} sample format: {0}")]
    UnsupportedSampleFormat(cpal::SampleFormat, String),

    #[error("Expected wavwriter in iced-app state but not found")]
    WavWriterNotFoundError,

    #[error("Error building stream: {0}")]
    BuildStreamError(String),
    //BuildStreamError(#[from] cpal::BuildStreamError),
    #[error("Error playing stream: {0}")]
    PlayStreamError(String),
    //PlayStreamError(#[from] cpal::PlayStreamError),

    #[error("Attempted to send on close channel")]
    StreamFrameSendError,
    #[error("Sending channel {1} disconnected: {0}")]
    StreamFrameRecvError(mpsc::RecvError, String),

    #[error("unable to end recording (send to stream channel): {0}")]
    StreamStopError(String),

    #[error("unable to find join_handle for control thread in state")]
    StateJoinHandleNotFound,

    #[error("unable to spawn control thread: {0}")]
    ThreadControlThreadFailedToSpawn(String),
    #[error("unable to execute join for control thread")]
    ThreadControlThreadFailedToJoin,
    //ThreadControlThreadFailedToJoin(String),
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

    input_sender: Option<mpsc::Sender<StreamCommand<f32>>>,
    output_sender: Option<mpsc::Sender<StreamCommand<f32>>>,
    input_stream: Option<cpal::Stream>,
    output_stream: Option<cpal::Stream>,
    control_thread: Option<thread::JoinHandle<Result<bool, RecorderError>>>,
    wav_writer: Option<Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>>,
}

impl Default for Recorder {
    fn default() -> Self {
        let host = cpal::default_host();
        let input_device = host.default_input_device()
            .expect("failed to find input device");
        let output_device = host.default_output_device()
            .expect("failed to find output device");

        const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");

        Self {
            host,
            input_device,
            output_device,
            state: State::Stop,
            recording_path: path::PathBuf::from(PATH),

            input_sender: None,
            output_sender: None,
            input_stream: None,
            output_stream: None,
            control_thread: None,
            wav_writer: None,
        }
    }
}

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

        let spec = wav_spec_from_configs(&input_config, &output_config)?;
        //let writer = hound::WavWriter::create(PATH, spec).map_err(|error| RecorderError::FileCreationError(error.to_string()))?;
        let writer = hound::WavWriter::create(self.recording_path.as_path(), spec).map_err(|error| RecorderError::FileCreationError(error.to_string()))?;
        let writer = Arc::new(Mutex::new(Some(writer)));

        // 1. Create channel to for stream_a
        let (ti, ri) = mpsc::channel();
        self.input_sender = Some(ti.clone());
        // 2. Create channel to for stream_b
        let (to, ro) = mpsc::channel();
        self.output_sender = Some(to.clone());

        // Run the input stream on a separate thread.
        let writer_2 = writer.clone();

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        let input_stream = match input_config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.input_device.build_input_stream(
                    &input_config.into(),
                    //move |data, _| send_frame::<f32>(data, ti_2.clone()),
                    move |data, _| send_frame::<f32>(data, ti.clone()),
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
                    move |data, _| send_frame::<f32>(data, to.clone()),
                    //move |data, _| send_frame::<f32>(data, ti.clone()),
                    err_fn,
                    None,
                ).map_err(|error| RecorderError::BuildStreamError(error.to_string()))?
            },
            sample_format =>  return Err(RecorderError::UnsupportedSampleFormat(sample_format, "output".to_string())),
        };
        // TODO: set thread to state
        let thread = thread::Builder::new()
            .name("cpal_wasapi_in".to_owned())
            .spawn(move || write_frame(ro, ri, &writer_2))
            .map_err(|error| RecorderError::ThreadControlThreadFailedToSpawn(error.to_string()))?;

        input_stream.play().map_err(|error| RecorderError::PlayStreamError(error.to_string()))?;
        output_stream.play().map_err(|error| RecorderError::PlayStreamError(error.to_string()))?;
        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        self.control_thread = Some(thread);
        self.wav_writer = Some(writer);
        Ok(())
    }

    fn record_stop(&mut self) -> Result<(), RecorderError> {
        if let Some(tx) = &self.input_sender {
            tx.send(StreamCommand::Stop).map_err(|error| RecorderError::StreamStopError(error.to_string()))?;
        };
        if let Some(tx) = &self.output_sender {
            tx.send(StreamCommand::Stop).map_err(|error| RecorderError::StreamStopError(error.to_string()))?;
        };
        drop(self.input_stream.take());
        drop(self.output_stream.take());
        //drop(thread);
        //let join_handle = self.control_thread.take();
        let join_handle = match self.control_thread.take() {
            Some(join_handle) => join_handle,
            None => return Err(RecorderError::StateJoinHandleNotFound),
        };
        let _ = join_handle.join().map_err(|_| RecorderError::ThreadControlThreadFailedToJoin)?;

        let writer = match &self.wav_writer {
            Some(writer) => writer,
            None => return Err(RecorderError::WavWriterNotFoundError),
        };

        //writer.lock().unwrap().take().unwrap().finalize()?;
        if let Err(e) = writer.lock().unwrap().take().unwrap().finalize() {
            println!("{}", e)
        }
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


/////////////////// CPAL ///////////////////////
#[derive(Clone)]
enum StreamCommand<T> {
    Frame(Vec<T>),
    //Error(usize),
    Stop,
}

// FUTURE: support mismatching sample_rate, sample_size
fn wav_spec_from_configs(config_a: &cpal::SupportedStreamConfig, config_b: &cpal::SupportedStreamConfig) -> Result<hound::WavSpec, RecorderError> {
    let sample_rate_a = config_a.sample_rate().0;
    let sample_rate_b = config_b.sample_rate().0;
    if sample_rate_a != sample_rate_b {
        return Err(RecorderError::UnsupportedSampleRate(sample_rate_a, sample_rate_b))
    }

    // sample size is also determined by frame size i think
    let sample_size_a = config_a.sample_format().sample_size();
    let sample_size_b = config_b.sample_format().sample_size();
    if sample_size_a != sample_size_b {
        return Err(RecorderError::UnsupportedSampleSize(sample_size_a, sample_size_b))
    }
    //let sample_size = std::cmp::min(sample_size_a, sample_size_b);

    Ok(hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate_a as _,
        bits_per_sample: (sample_size_a * 8) as _,
        sample_format: sample_format(config_a.sample_format()),
    })
}

fn send_frame<T: cpal::Sample>(input: &[T], chan: mpsc::Sender<StreamCommand<T>>) {
  if let Err(_) = chan.send(StreamCommand::Frame(input.to_vec())) {
      Task::done(Message::Error(RecorderError::StreamFrameSendError));
  }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

// FUTURE: use a builder to generate ?
fn write_frame<T>(ra: mpsc::Receiver<StreamCommand<T>>, rb: mpsc::Receiver<StreamCommand<T>>, writer: &WavWriterHandle) -> Result<bool, RecorderError>
where
    T: cpal::Sample<Signed = T> + hound::Sample, // + std::fmt::Display,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            loop {
                let sample_a = match ra.recv() {
                    Ok(StreamCommand::Frame(a)) => a,
                    Err(e) => return Err(RecorderError::StreamFrameRecvError(e, "a2".to_string())),
                    Ok(StreamCommand::Stop) => return Ok(false), 
                };
                let sample_b = match rb.recv() {
                    Ok(StreamCommand::Frame(b)) => b,
                    Err(e) => return Err(RecorderError::StreamFrameRecvError(e, "b4".to_string())),
                    Ok(StreamCommand::Stop) => return Ok(false), 
                };

                // TODO some way to customize it
                let mut a_iter = sample_a.into_iter();
                let mut b_iter = sample_b.into_iter();
                loop {
                    let a0 = match a_iter.next() {
                        Some(a) => a,
                        None => break,
                    };
                    let a1 = match a_iter.next() {
                        Some(a) => a,
                        None => break,
                    };
                    let b0 = match b_iter.next() {
                        Some(b) => b,
                        None => break,
                    };
                    let b1 = match b_iter.next() {
                        Some(b) => b,
                        None => break,
                    };

                    let s0 = a0.add_amp(b0);
                    let s1 = a1.add_amp(b1);
                    writer.write_sample(s0).ok();
                    writer.write_sample(s1).ok();

                    if let None = b_iter.next() { break }
                    if let None = b_iter.next() { break }
                }
            };
        }
    }
    return Ok(false);
}


////////////////// UTIL //////////////
fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}
