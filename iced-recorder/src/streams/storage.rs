
use std::io;
use std::fs;
use std::path;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{channel, Sender, SendError, Receiver};
use crate::storage::{self, Mp3Spec};
use crate::streams::enums::{Command, StreamError};



//#[derive(thiserror::Error, Debug, Clone)]
//pub enum StreamError {
//    #[error("unable to get spawn child thread to write checkpoint: {0}")]
//    ChildThreadFailedToSpawn(String),
//    //#[error("unable to get config from stream device: {0}")]
//    //StreamConfigError(String),
//}

//#[derive(thiserror::Error, Debug, Clone)]
//pub enum ChildThreadError {
//    #[error("unable to get config from stream device: {0}")]
//    StreamConfigError(String),
//}


//#[derive(Clone, Debug)]
//pub enum Command {
//    Frame(Vec<f32>),
//    Stop,
//}

#[derive(Clone, Debug)]
enum ChildStatus {
    Error(storage::Mp3WriterError),
    SpawnError(String),
    RecvError(String),
    Success,
    Pending,
}

// this is the inner variables moved into the thread
//struct RunContext<P>
//where P: AsRef<path::PathBuf> {
struct RunContext {
    path: path::PathBuf,
    spec: Mp3Spec,
    commands: Receiver<Command>,

    // state
    buffer: Vec<f32>,
    num_checkpoints: usize,
    checkpoint_threads: Vec<Option<JoinHandle<()>>>,
    checkpoint_results: Vec<ChildStatus>,
    checkpoint_recvs: Vec<Receiver<ChildStatus>>,
    checkpoint_paths: Vec<path::PathBuf>,
}

// this is the outer shell "remote" held by the master
//#[derive(Clone, Debug)]
pub struct WriterStream {
    thread: Option<JoinHandle<()>>,
    //chan: mpsc::Receiver<Command<Vec<f32>>>,
    commands: Sender<Command>,
    //error_callback: &mut dyn FnMut(StreamError),
}

impl WriterStream {
    // changed AsRef<PathBuf> to just PathBuf since i need to send it to the thread
    pub fn new<E>(spec: Mp3Spec, path: path::PathBuf, mut error_callback: E) -> WriterStream 
    where
        E: FnMut(StreamError) + Send + 'static,
    {
        let (tx, rx) = channel();
        let run_context = RunContext {
            path,
            spec,
            commands: rx,

            buffer: Vec::new(),
            num_checkpoints: 0,
            checkpoint_threads: Vec::new(),
            checkpoint_results: Vec::new(),
            checkpoint_recvs: Vec::new(),
            checkpoint_paths: Vec::new(),
        };
        let thread = thread::Builder::new()
            .name("writer_stream".to_owned())
            .spawn(move || run_input(run_context, &mut error_callback))
            .unwrap();
        WriterStream {
            thread: Some(thread),
            commands: tx,
        }
    }

    pub fn write_frame(&self, frame: Vec<f32>) -> Result<(), SendError<Command>> {
        self.push_command(Command::Frame(frame))
    }
    fn push_command(&self, command: Command) -> Result<(), SendError<Command>> {
        self.commands.send(command)?;
        Ok(())
    }
    pub fn get_sender(&self) -> Sender<Command> {
        self.commands.clone()
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

// change this to T instead of f32
//fn run_input<P: AsRef<path::PathBuf>>(
fn run_input(
    mut run_context: RunContext, 
    error_callback: &mut dyn FnMut(StreamError),
) {
    loop {
        let last_loop = false;
        // try_iter will not block, here we want to
        let mut sample = match run_context.commands.recv() {
            Ok(Command::Frame(sample)) => sample,
            _ => break,
        };

        // You need to do one more loop if its a break

        // 1. Add sample to buffer
        run_context.buffer.append(&mut sample);
        // 2. If buffer >= 2min, launch thread to checkpoint
        if !should_checkpoint(run_context.spec, run_context.buffer.len()) {
            continue
        }
        // 3. To checkpoint
        let idx = run_context.num_checkpoints; // for addressing into slice
        run_context.num_checkpoints += 1;
        let str_num_checkpoint = format!("{:04}", run_context.num_checkpoints);
        let thread_name = "checkpoint_".to_string() + &str_num_checkpoint;
        let mut checkpoint_file_name = thread_name.clone();
        checkpoint_file_name.push_str(".tmp");
        //let checkpoint_path = gen_checkpoint_path(run_context.path, run_context.num_checkpoints);
        let mut checkpoint_path = run_context.path.clone();
        //let checkpoint_path = run_context.path.as_ref().clone();
        checkpoint_path.set_file_name(checkpoint_file_name);

        let (tx, rx) = channel();
        let checkpoint_path_2 = checkpoint_path.clone();
        run_context.checkpoint_recvs.push(rx);
        run_context.checkpoint_paths.push(checkpoint_path_2);
        run_context.checkpoint_results.push(ChildStatus::Pending);

        let checkpoint_buffer = std::mem::take(&mut run_context.buffer);
        let thread_result = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
    //pub fn create<P: AsRef<path::Path>>(filename: P, spec: Mp3Spec) -> Result<Mp3Writer<io::BufWriter<fs::File>>, Mp3WriterError> {
                match storage::Mp3Writer::<io::BufWriter<fs::File>>::create_and_write(checkpoint_path, run_context.spec, checkpoint_buffer) {
                    Ok(()) => tx.send(ChildStatus::Success),
                    Err(e) => tx.send(ChildStatus::Error(e)),
                };
                return
                //let mp3writer = match storage::Mp3Writer::create(checkpoint_path, run_context.spec) {
                //    Ok(writer) => writer,
                //    Err(e) => {
                //        tx.send(ChildStatus::Error(e));
                //        return;
                //    },
                //};
                ////let mp3writer = storage::Mp3Writer::create(checkpoint_path, run_context.spec)
                ////    .map_err(|error| storage::Mp3WriterError:: error.to_string())?;
                //match mp3writer.write_to_path(checkpoint_buffer) {
                //    Ok(()) => tx.send(ChildStatus::Success),
                //    Err(e) => tx.send(ChildStatus::Error(e)),
                //};
                //return;
            })
            .map_err(|error| StreamError::ChildThreadFailedToSpawn(error.to_string())); //?;
        let thread = match thread_result {
            Ok(handle) => handle,
            Err(e) => {
                error_callback(e.clone());
                run_context.checkpoint_results[idx] = ChildStatus::SpawnError(e.to_string());
                //run_context.checkpoint_results.pop();
                //run_context.checkpoint_results.push(ChildStatus::SpawnError(e.to_string()));
                run_context.checkpoint_threads.push(None);
                continue;
            },
        };
        run_context.checkpoint_threads.push(Some(thread));
    }
    // todo: CLEANUP
    for i in 0..run_context.checkpoint_threads.len() {
        let mut maybe_handle = None;
        std::mem::swap(&mut run_context.checkpoint_threads[i], &mut maybe_handle);
        match maybe_handle {
            // WARNING unwrap here
            Some(handle) => handle.join().unwrap(),
            None => println!("No handle"),
        }
        let child_status = match run_context.checkpoint_recvs[i].recv() {
            Ok(status) => status,
            Err(e) => ChildStatus::RecvError(e.to_string()),
        };
        run_context.checkpoint_results[i] = child_status;
        println!("{:?} {:?}", run_context.checkpoint_results[i], run_context.checkpoint_paths[i])
    }
}


// Notes
// 1. StreamInner to hold state of buffer
// 2. StreamInner's process_input(&self, frame: Stream(T)) to do the run op

////////////////////////// 
//pub struct Mp3Spec {
//    pub channels: u8,
//    pub sample_rate: usize,
//    pub bits_per_sample: u16,
//}

// checkpoints every 2 mins
fn should_checkpoint(spec: Mp3Spec, buff_size: usize) -> bool {
    let samples_per_min: usize = (spec.sample_rate * 60) as usize;
    let samples_per_min_interleaved: usize = samples_per_min * spec.channels as usize;
    samples_per_min_interleaved * 2 <= buff_size
}


//fn gen_checkpoint_path<P: AsRef<path::PathBuf>>(base_path: P, checkpoint_num: usize) -> path::PathBuf {
//    //let dir_path = base_path.parent().unwrap;
//}
