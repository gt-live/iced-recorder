use iced::{Theme, Element};
use iced::{ Length, Task, Size, window, Renderer };
use iced::widget::{ Column, column, container };
//use iced::widget::{ Button, button, Column, column, container, PickList, pick_list, row, text, Text };
use iced::{ border, Border, color };

pub const WINDOW_INITIAL_WIDTH: f32 = 170.0;
pub const WINDOW_INITIAL_HEIGHT: f32 = 310.0;

fn main() -> Result<(), iced::Error> {
    println!("Hello, world!");
    iced::application(Levels::title, Levels::update, Levels::view)
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
    Set(f32),
}

struct Levels {
    // you can compose multiple together anyway
    value: f32,
}

impl Default for Levels {
    fn default() -> Self {
        Self {
            value: 0.0,
        }
    }
}

impl Levels {
    fn title(&self) -> String {
        "Audio Levels".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Set(level) => { 
                self.value = level;
                Task::none()
            }
        }
    }

    //fn view(&self) -> Element<'_, Message> {
    fn view(&self) -> Column<Message> {
        let step = container("")
            //.padding(10)
            .style(rounded_box);
        let step_1 = container("")
            .padding(10)
            .style(colored_box(0x0aac0a));
        let step_2 = container("")
            .padding(20)
            .style(colored_box(0x0bc40b));
        let step_3 = container("")
            .padding(30)
            .style(colored_box(0xf329f3));
        column![step, step_1, step_2, step_3]
    }
}

pub fn colored_box(hex: i32) -> impl Fn(&Theme) -> container::Style {
    //let palette = theme.extended_palette();
    move |theme| {
        container::Style {
            background: Some(color!(hex).into()),
            //background: Some(palette.background.weak.color.into()),

            border: border::rounded(2),
            ..container::Style::default()
        }
    }
}

//
//
///// A rounded [`Container`] with a background.
//pub fn colored_box(hex: i32) -> container::Style {
//    //let palette = theme.extended_palette();
//    container::Style {
//        background: Some(color!(hex).into()),
//        //background: Some(palette.background.weak.color.into()),
//
//        border: border::rounded(2),
//        ..container::Style::default()
//    }
//}

pub fn rounded_box(theme: &Theme) -> container::Style {
    //let palette = theme.extended_palette();
    container::Style {
        background: Some(color!(0x0ddc0d).into()),
        //background: Some(palette.background.weak.color.into()),

        border: border::rounded(2),
        ..container::Style::default()
    }
}


