use iced::widget::{ Button, button, Column, column, container, PickList, pick_list, row, text, Text };
use iced::{ Length, Task, Size, Subscription, window, Renderer };
use iced::Theme;
use iced::task;

//use iced_recorder::controller::{self, StreamCommand, WavWriterHandle};
//use events_recorder::runtime::{self};
use events_recorder::runtime;
use events_recorder::streams::{self, UiMessage, UiUpdate};
use events_recorder::storage;
use events_recorder::error::RecorderError;

use std::path;
use std::sync::mpsc::{channel, Sender, SendError, Receiver, RecvError};

//fn main() {
//    println!("Hello, world!");
//}

pub const WINDOW_INITIAL_WIDTH: f32 = 170.0;
pub const WINDOW_INITIAL_HEIGHT: f32 = 310.0;

fn main() -> Result<(), iced::Error> {

    println!("Hello, world!");
    iced::application(Recorder::title, Recorder::update, Recorder::view)
        .theme(|_| Theme::Dark)
        //.centered()
        //.subscription(Recorder::subscription)
        .window(window::Settings {
            size: Size::new(WINDOW_INITIAL_WIDTH, WINDOW_INITIAL_HEIGHT),
            ..window::Settings::default()
        })
        .run()
}

// https://github.com/iced-rs/iced/pull/2331
// Task<Message> replaces Command<Message> (the async think)
enum State {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
enum Message {
    Start,
    Stop,
    ////CheckDevices,
    //SettingsSelectInputDevice(String),
    //SettingsSelectOutputDevice(String),
    //SettingsSyncIoDevices,
    //SettingsSelectFilePath,
    PulseUpdate(f32),
    Error(RecorderError),
}

#[derive(Default)]
struct Recorder {
    runtime: Option<runtime::Runtime>,
    state: State,

    recording_path: path::PathBuf,

    ui_sender: Option<Sender<streams::UiUpdate>>,
    ui_thread: Option<task::Handle>,
}

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
impl Recorder {
    fn default() -> Self {
        Self {
            runtime: None,
            recording_path:  path::PathBuf::from(PATH),
            state: State::Stop,
            ui_sender: None,
            ui_thread: None,
        }
    }
    fn title(&self) -> String {
        "A naive two stream rusty recorder".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Start => {
                // task is a Ui update Sipper
                let task = match self.record_start() {
                    Err(e) => return Task::done(Message::Error(e)),
                    Ok(task) => task,
                };
                // NOTE: this must be after the above block, for Subscription initializer
                self.state = State::Start;
                //Task::none()
                task
            }
            Message::Stop => {
                self.state = State::Stop;
                if let Err(e) = self.record_stop() {
                    return Task::done(Message::Error(e))
                }
                Task::none()
            }
            //Message::SettingsSelectInputDevice(device) => {
            //    if let Some(device) = self.host.input_devices()
            //        .unwrap() // should be fine right? else the conditional wont run
            //        .find(|x| x.name().map(|y| y == device).unwrap_or(false)) 
            //    {
            //        self.input_device = device
            //        // TODO: code that pauses, creates new stream and continues running
            //    };
            //    if let Err(e) = self.reload_input_device() {
            //        return Task::done(Message::Error(e))
            //    };
            //    Task::none()
            //},
            //Message::SettingsSelectOutputDevice(device) => {
            //    if let Some(device) = self.host.output_devices()
            //        .unwrap() // should be fine right? else the conditional wont run
            //        .find(|x| x.name().map(|y| y == device).unwrap_or(false)) 
            //    {
            //        self.output_device = device
            //        // TODO: code that pauses, creates new stream and continues running
            //    };
            //    if let Err(e) = self.reload_output_device() {
            //        return Task::done(Message::Error(e))
            //    };
            //    Task::none()
            //},
            //Message::SettingsSyncIoDevices => {
            //    //eprintln!("[Update][SettingsSyncIoDevices]");
            //    if let Err(e) = self.reload_default_devices() {
            //        eprintln!("[Update][SettingsSyncIoDevices] reload default error: {e}");
            //        return Task::done(Message::Error(e))
            //    }
            //    //eprintln!("[Update][SettingsSyncIoDevices] pre-reload");
            //    if let Err(e) = self.reload_devices() {
            //        eprintln!("[Update][SettingsSyncIoDevices] reload devices error: {e}");
            //        return Task::done(Message::Error(e))
            //    };
            //    println!("[Update][SettingsSyncIoDevices] done");
            //    Task::none()
            //},
            //Message::SettingsSelectFilePath => {
            //    let fd = FileDialog::new()
            //        .set_title("where to save recording to")
            //        .set_file_name("recording.wav")
            //        .save_file();
            //    if let Some(path) = fd {
            //        self.recording_path = path;
            //    }
            //    Task::none()
            //},
            Message::PulseUpdate(x) => {
                self.volume = x;
                Task::none()
            },
            Message::Error(e) => {
                println!("[ERROR] {:?}", e);
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
        //let audio_button: Button<'_, Message> = button("sync audio").on_press(Message::SettingsSyncIoDevices);
        //let rand_text: Text<'_, Theme, Renderer> = text(fastrand::u32(0..std::u32::MAX));
        //let browse_button: Button<'_, Message> = button("browse").on_press(Message::SettingsSelectFilePath);
        //let browse_button = container(browse_button)
        //    .width(Length::FillPortion(1));
        let browse_path: Text<'_, Theme, Renderer> = text(self.recording_path.as_path().to_str().unwrap_or(""));
        let browse_path = container(browse_path)
            .width(Length::FillPortion(3));
        let volume_bar = text(self.volume);
        let path_row = row![browse_path, browse_button];
        let control_bar = row![input_list, output_list];
        let interface = column![control_bar, path_row, volume_bar, record_button];
        //let interface = column![control_bar, audio_button, path_row, rand_text, volume_bar, record_button];
        interface
    }

    fn record_stop(&mut self) -> Result<(), RecorderError> {
    }

    fn record_start(&mut self) -> Result<(), RecorderError> {
        let (ui_sender, ui_receiver) = channel();
        self.ui_sender = Some(ui_sender.clone());

        let (ui_task, handle) = Task::run(
            streams::progress(ui_receiver),
            |x| {
                let message = match x.expect("[ui_thread_callback] SUBSCRIPTION ERROR") {
                    streams::UiMessage::Decibel(db) => Message::PulseUpdate(db),
                    streams::UiMessage::AudioIoError => { 
                        eprintln!("[ui_thread_callback] AudioIoError");
                        Message::Error("[ui_thread_callback] AudioIoError".to_string())
                    },
                };
                message
            },
        ).abortable();
        //let (task, handle) = Task::sip(
        //    streams::progress(ui_receiver),
        //    Message::PulseUpdate,
        //    |sample| Message::PulseUpdate(0.0),
        //).abortable();
        self.ui_thread = Some(handle.abort_on_drop());

        let runtime = runtime::Runtime::new(ui_sender, self.recording_path)
            .map_err(|e| RecorderError::RuntimeBuildError(e.to_string()))?;
        Ok(())
    }
}




