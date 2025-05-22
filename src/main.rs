//https://book.iced.rs/additional-resources.html
use iced::widget::{ Button, button, Column, column, container };
use iced::{ Task, Size, window };
use iced::Theme;

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

#[derive(Debug, Clone)]
enum Message {
    Start,
    Stop,
}

// https://github.com/iced-rs/iced/pull/2331
// Task<Message> replaces Command<Message> (the async think)
enum State {
    Start,
    Stop,
}

struct Recorder {
    state: State,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            state: State::Stop
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
        }
    }

    fn view(&self) -> Column<Message> {

        let record_button = gen_record_button(&self.state);
        let record_button = iced::Element::new(record_button).explain(iced::Color::BLACK);
        //let record_button = container(record_button)
        //    .padding(10)
        //    .center(800)
        //    .style(container::rounded_box);

        let interface = column![record_button];
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

