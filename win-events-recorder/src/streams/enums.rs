
// consider bringing this out
#[derive(thiserror::Error, Debug, Clone)]
pub enum StreamError {
    #[error("unable to get spawn child thread to write checkpoint: {0}")]
    ChildThreadFailedToSpawn(String),
    #[error("unable to send on channel")]
    SendError,
    #[error("unable to recv on channel")]
    RecvError,
}

#[derive(Clone, Debug)]
pub enum Command {
    Frame(Vec<f32>),
    Stop,
}

// cpal err_fn callback -> ui_thread
#[derive(Debug, Clone)]
pub enum UiUpdate {
    Pulse(f32),
    //Error(StreamError),
    AudioIoError,
    Stop,
}

// ui_thread -> view
#[derive(Debug, Clone)]
pub enum UiMessage {
    Decibel(f32),
    AudioIoError,
}


