use iced::futures::{SinkExt, Stream};
use iced::stream::try_channel;
//use iced::task::{Sipper, sipper};
use std::sync::mpsc::{Receiver};
use crate::streams::enums::UiUpdate;

pub fn progress(rx: Receiver<UiUpdate>) -> impl Stream<Item = Result<f32, ()>> {
    try_channel(100, move |mut output| async move {
        loop {
            let x = match rx.recv() {
                Ok(UiUpdate::Pulse(x)) => x,
                Ok(UiUpdate::Stop) => break,
                Err(e) => {
                    print!("[progress-notifier] Error: {}", e);
                    break;
                },
            };
            output.send(x).await;
        }
        Ok(())
    })
}

//pub fn progress(rx: Receiver<UiUpdate>) -> impl Sipper<(), f32> {
//    sipper(async move |mut output| {
//        loop {
//            let x = match rx.recv() {
//                Ok(UiUpdate::Progress(x)) => x,
//                Ok(UiUpdate::Stop) => break,
//                Err(e) => {
//                    print!("[progress-notifier] Error: {}", e);
//                    break;
//                },
//            };
//            output.send(x).await;
//        }
//        Ok(())
//    })
//}
