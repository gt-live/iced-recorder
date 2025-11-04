use std::thread::{self, JoinHandle};
use std::sync::mpsc::{channel, Receiver, SendError, Sender};


pub enum Command {
    PlayStream,
    PauseStream,
    Terminate,
}

pub struct Stream {
    /// The high-priority audio processing thread calling callbacks.
    /// Option used for moving out in destructor.
    ///
    /// TODO: Actually set the thread priority.
    thread: Option<JoinHandle<()>>,

    // Commands processed by the `run()` method that is currently running.
    // `pending_scheduled_event` must be signalled whenever a command is added here, so that it
    // will get picked up.
    commands: Sender<Command>,
}

struct RunContext {
    stream: StreamInner,
    commands: Receiver<Command>,
}

pub struct StreamInner {
    // True if the stream is currently playing. False if paused.
    pub playing: bool,
    // Number of frames of audio data in the underlying buffer allocated by WASAPI.
    pub max_frames_in_buffer: u32,
    // Number of bytes that each frame occupies.
    pub bytes_per_frame: u16,
    // The configuration with which the stream was created.
    pub config: crate::StreamConfig,
    // The sample format with which the stream was created.
    pub sample_format: SampleFormat,
}

fn run_input(
    mut run_ctxt: RunContext, 
    error_callback: &mut dyn FnMut(StreamError),
) {
    loop {
        match process_commands_and_await_signal(&mut run_ctxt, error_callback) {
            Some(ControlFlow::Break) => break,
            Some(ControlFlow::Continue) => continue,
            None => (),
        }
        // run the shit
        //match process_input(
        //    &run_ctxt.stream,
        //    capture_client,
        //    data_callback,
        //    error_callback,
        //) {
        //    ControlFlow::Break => break,
        //    ControlFlow::Continue => continue,
        //}
    }
}

impl StreamTrait for Stream {
    fn play(&self) -> Result<(), PlayStreamError> {
        self.push_command(Command::PlayStream)
            .map_err(|_| crate::error::PlayStreamError::DeviceNotAvailable)?;
        Ok(())
    }
    fn pause(&self) -> Result<(), PauseStreamError> {
        self.push_command(Command::PauseStream)
            .map_err(|_| crate::error::PauseStreamError::DeviceNotAvailable)?;
        Ok(())
    }
}

impl Stream {
    pub(crate) fn new(stream_inner: StreamInner) -> Stream
    {
        let (tx, rx) = channel();

        let run_context = RunContext {
            handles: Vec::new(),
            stream: stream_inner,
            commands: rx,
        };

        let run_context = RunContext {
            handles: Vec::new(),
            stream: stream_inner,
            commands: rx,
        };

        let thread = thread::Builder::new()
            .name("cpal_wasapi_in".to_owned())
            .spawn(move || run_input(run_context))
            .unwrap();

        Stream {
            thread: Some(thread),
            commands: tx,
        }
    }

    #[inline]
    fn push_command(&self, command: Command) -> Result<(), SendError<Command>> {
        self.commands.send(command)?;
        unsafe {
            Threading::SetEvent(self.pending_scheduled_event).unwrap();
        }
        Ok(())
    }
}

impl Drop for Stream {
    #[inline]
    fn drop(&mut self) {
        if self.push_command(Command::Terminate).is_ok() {
            self.thread.take().unwrap().join().unwrap();
        }
    }
}


// Process any pending commands that are queued within the `RunContext`.
// Returns `true` if the loop should continue running, `false` if it should terminate.
fn process_commands(run_context: &mut RunContext) -> Result<bool, StreamError> {
    // Process the pending commands.
    for command in run_context.commands.try_iter() {
        match command {
            Command::PlayStream => {
                if !run_context.stream.playing {
                    run_context.stream.playing = true;
                }
            },
            Command::PauseStream => {
                if run_context.stream.playing {
                    run_context.stream.playing = false;
                }
            },
            Command::Terminate => {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn process_commands_and_await_signal(
    run_context: &mut RunContext,
    error_callback: &mut dyn FnMut(StreamError),
) -> Option<ControlFlow> {
    // Process queued commands.
    match process_commands(run_context) {
        Ok(true) => (),
        Ok(false) => return Some(ControlFlow::Break),
        Err(err) => {
            error_callback(err);
            return Some(ControlFlow::Break);
        }
    };
    None
}


enum ControlFlow {
    Break,
    Continue,
}
