
use std::path;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{channel, Sender, Receiver};

#[derive(Clone)]
enum Command<T> {
    Frame(Vec<T>),
    Stop,
}

// this is the inner variables moved into the thread
struct RunContext<P>
where P: AsRef<path::PathBuf> {
    path: P,
    spec: Mp3Spec,
    commands: Receiver<Command<Vec<f32>>>,
}

// this is the outer shell "remote" held by the master
pub struct WriterStream {
    thread: Option<JoinHandle<()>>,
    //chan: mpsc::Receiver<Command<Vec<f32>>>,
    commands: Sender<Command<Vec<f32>>>,
    //error_callback: &mut dyn FnMut(StreamError),
}

impl WriterStream {
    pub fn new<P: AsRef<path::PathBuf>>(spec: Mp3Spec, path: P, error_callback: &mut dyn FnMut(StreamError)) -> WriterStream {
        let (tx, rx) = channel();
        let run_context = RunContext {
            path,
            spec,
            commands: rx,
        };
        let thread = thread::Builder::new()
            .name("writer_stream".to_owned())
            .spawn(move || run_input(run_context, error_callback))
            .unwrap();
        WriterStream {
            thread: Some(thread),
            commands: tx,
        }
    }

    pub fn write_frame<T>(&self, frame: Vec<T>) -> Result<(), SendError> {
        self.push_command(Command::Frame(frame))
    }
    //TODO: SendError
    fn push_command(&self, command: Command) -> Result<(), SendError(Command)>> {
        self.commands.send(command)?;
        Ok(())
    }
}

impl Drop for WriterStream {
    #[inline]
    fn drop(&mut self) {
        if self.push_command(Command::Stop).is_ok() {
            self.thread.take().unwrap().join().unwrap();
        }
    }
}

fn run_input<P: AsRef<path::PathBuf>>(
    mut run_context: RunContext<P>, 
    error_callback: &mut dyn FnMut(StreamError),
) {
    loop {
        // try_iter will not block, here we want to
        let sample = match run_context.commands.recv() {
            Some(Command::Frame(sample)) => sample,
            _ => return,
        };

        // 1. Add sample to buffer
        // 2. If buffer >= 2min, launch thread to checkpoint
    }
}
