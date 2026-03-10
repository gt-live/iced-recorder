use iced::futures::{SinkExt, Stream};
use iced::stream::try_channel;
//use iced::task::{Sipper, sipper};
use std::sync::mpsc::{Receiver};
use crate::streams::enums::{UiMessage, UiUpdate};

use std::sync::mpsc::{Sender};
use crate::streams::UiUpdate;

pub fn app_notifier(rx: Receiver<UiUpdate>) -> impl Stream<Item = Result<UiMessage, ()>> {
    try_channel(100, move |mut output| async move {
        loop {
            eprintln!("[progress-notifier] Pre-loop");
            let ui_message = match rx.recv() {
                Ok(UiUpdate::Pulse(x)) => {
                    let amp_in_db = f32_to_db(x);
                    //eprintln!("[progress-notifier] decibel {amp_in_db}");
                    UiMessage::Decibel(amp_in_db)
                },
                Ok(UiUpdate::Stop) => break,
                //Ok(UiUpdate::Stop) => return Ok(()),
                Ok(UiUpdate::AudioIoError) => {
                    //eprintln!("[progress-notifier] AudioError");
                    output.send(UiMessage::AudioIoError).await;
                    continue;
                    //return Ok(());
                },
                Err(e) => {
                    println!("[progress-notifier] Error: {}", e);
                    //break;
                    continue;
                },
            };
            //eprintln!("[progress-notifier] Pre-send");
            if let Err(e) = output.send(ui_message).await {
                eprintln!("[ui_thread] error occurred while sending UiMessage: {e}");
            };
            eprintln!("[progress-notifier] Post-send");
        }
        Ok(())
    })
}

//pub fn gen_input_audio_err_fn(sender: Sender<UiUpdate>) -> (impl FnMut(cpal::StreamError) + Send) {
//    gen_audio_err_fn(sender, "input")
//}
//pub fn gen_output_audio_err_fn(sender: Sender<UiUpdate>) -> (impl FnMut(cpal::StreamError) + Send) {
//    gen_audio_err_fn(sender, "output")
//}
//pub fn gen_audio_err_fn(sender: Sender<UiUpdate>, id: &str) -> (impl FnMut(cpal::StreamError) + Send) {
//    move |err| {
//        eprintln!("[audio_err_fn][{id}] an error occurred on stream: {err}");
//        if let Err(e) = sender.send(UiUpdate::AudioIoError) {
//            eprintln!("[audio_err_fn][{id}] an error occurred while sending UiUpdate: {e}");
//        };
//    }
//}

// fourier properties 2, module 3, dft 2
fn f32_to_db(amp: f32) -> f32 {
    20.0 * amp.abs().log10()
}
