
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

//#[derive(Clone, Debug)]
//pub enum UiCommand {
//    Progress(f32),
//    Stop,
//}

#[derive(Debug, Clone)]
pub enum UiUpdate {
    Pulse(f32),
    Error(StreamError),
    Stop,
}
