use std::sync::mpsc;
// TODO: see how other applications do the error handling
#[derive(thiserror::Error, Debug, Clone)]
pub enum RecorderError {
    #[error("unable to get config from stream device: {0}")]
    StreamConfigError(String),
    //StreamConfigError(#[from] cpal::DefaultStreamConfigError),
    #[error("Merging of streams with differing sample rate currently not supported `{0}`, `{1}`")]
    UnsupportedSampleRate(u32, u32),
    #[error("Merging of streams with differing sample size currently not supported `{0}`, `{1}`")]
    UnsupportedSampleSize(usize, usize),

    #[error("Unable to create file at path: {0}")]
    FileCreationError(String),
    //FileCreationError(#[from] hound::Error),
    //FileCreationError(#[from] hound::Error, String)
    #[error("Unsupported {1} sample format: {0}")]
    UnsupportedSampleFormat(cpal::SampleFormat, String),

    #[error("Expected wavwriter in iced-app state but not found")]
    WavWriterNotFoundError,

    #[error("Error building stream: {0}")]
    BuildStreamError(String),
    //BuildStreamError(#[from] cpal::BuildStreamError),
    #[error("Error playing stream: {0}")]
    PlayStreamError(String),
    //PlayStreamError(#[from] cpal::PlayStreamError),

    #[error("Attempted to send on close channel")]
    StreamFrameSendError,
    #[error("Sending channel {1} disconnected: {0}")]
    StreamFrameRecvError(mpsc::RecvError, String),

    #[error("unable to end recording (send to stream channel): {0}")]
    StreamStopError(String),

    #[error("unable to write frame to storage stream: {0}")]
    StorageStreamWriteError(String),

    #[error("unable to find join_handle for control thread in state")]
    StateJoinHandleNotFound,

    #[error("unable to spawn control thread: {0}")]
    ThreadControlThreadFailedToSpawn(String),
    #[error("unable to execute join for control thread")]
    ThreadControlThreadFailedToJoin,
    //ThreadControlThreadFailedToJoin(String),
    #[error("unable to write to file: {0}")]
    FileWriterWriterError(String),
    #[error("unable to create write to file: {0}")]
    FileWriterCreateError(String),
    #[error("unable to finalize to file: {0}")]
    FileWriterFinalizeError(String),
}
