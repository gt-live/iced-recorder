//https://book.iced.rs/additional-resources.html
use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use iced::widget::{ Button, button, Column, column, container, PickList, pick_list, row };
use iced::{ Length, Task, Size, window, Renderer };
use iced::Theme;

//mod startup;
//use crate::startup::{get_host};

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
}

impl Default for Recorder {
    fn default() -> Self {
        let host = cpal::default_host();
        let input_device = host.default_input_device()
            .expect("failed to find input device");
        let output_device = host.default_output_device()
            .expect("failed to find output device");
        Self {
            host,
            input_device,
            output_device,
            state: State::Stop,
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

    fn update(&mut self, message: Message) {
        match message {
            Message::Start => {
                self.state = State::Start
            }
            Message::Stop => {
                self.state = State::Stop
            }
            Message::SettingsSelectInputDevice(device) => {
                if let Some(device) = self.host.input_devices()
                    .unwrap() // should be fine right? else the conditional wont run
                    .find(|x| x.name().map(|y| y == device).unwrap_or(false)) 
                {
                    self.input_device = device
                    // TODO: code that pauses, creates new stream and continues running
                }
            },
            Message::SettingsSelectOutputDevice(device) => {
                if let Some(device) = self.host.output_devices()
                    .unwrap() // should be fine right? else the conditional wont run
                    .find(|x| x.name().map(|y| y == device).unwrap_or(false)) 
                {
                    self.output_device = device
                    // TODO: code that pauses, creates new stream and continues running
                }
            },
        }
    }

    fn view(&self) -> Column<Message> {
        // TODO: add droplist for devices

        let record_button = gen_record_button(&self.state);
        let record_button = iced::Element::new(record_button).explain(iced::Color::BLACK);
        //let record_button = container(record_button)
        //    .padding(10)
        //    .center(800)
        //    .style(container::rounded_box);
        //let input_devices: Vec<String> = self.host.input_devices()
        //        .unwrap()
        //        //.unwrap_or(|| [])
        //        .map(|x| x.name()
        //            .unwrap_or("EMPTY".to_string())
        //        ).collect();

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
        let control_bar = row![input_list, output_list];
        let interface = column![control_bar, record_button];
        interface
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

