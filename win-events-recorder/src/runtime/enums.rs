use crate::settings::{DeviceName, InputDeviceSelection, OutputDeviceSelection};
use std::path;


#[derive(Debug, Clone)]
pub enum Event {
    LoadDevices(InputDeviceSelection, OutputDeviceSelection),
    LoadPath(path::PathBuf),
    AudioDeviceNotAvailable,
    AudioDeviceUnspecifiedError(String),

    Stop,
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum RuntimeError {
    #[error("unable get device: {0}")]
    GetIoDeviceError(String),
    #[error("unable build new context head: {0}")]
    ContextHeadBuildError(String),
    #[error("unable build new context: {0}")]
    ContextBuildError(String),

    #[error("unable play device: {0}")]
    PlayStreamError(String),
}
