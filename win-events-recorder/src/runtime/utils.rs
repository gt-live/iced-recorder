use std::sync::mpsc::{Receiver, Sender};
use crate::runtime;

// io -> rt 
pub fn gen_audio_err_fn(sender: Sender<runtime::Event>, id: &str) -> (impl FnMut(cpal::StreamError) + Send) {
    move |err| {
        eprintln!("[audio_err_fn][{id}] an error occurred on stream: {err}");
        let audio_err_event = match err {
            cpal::StreamError::DeviceNotAvailable => runtime::Event::AudioDeviceNotAvailable,
            cpal::StreamError::BackendSpecific { err } => runtime::Event::AudioDeviceUnspecifiedError(err.to_string()),
        };
        if let Err(e) = sender.send(audio_err_event) {
            eprintln!("[audio_err_fn][{id}] an error occurred while sending UiUpdate: {e}");
        };
    }
}
