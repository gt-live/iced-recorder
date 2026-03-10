use crate::error::RecorderError;
use crate::storage;
use crate::streams;
use std::sync::mpsc::{channel, Sender, SendError, Receiver, RecvError};
use std::sync::{Arc, Mutex};
use std::io::{self, BufWriter};
use std::fs::{self, File};
//use std::error::Error;

use std::thread::{self, JoinHandle};
use crate::streams::enums::{StreamError, Command, UiUpdate};


//// consider bringing this out
//#[derive(thiserror::Error, Debug, Clone)]
//pub enum StreamError {
//    #[error("unable to get spawn child thread to write checkpoint: {0}")]
//    ChildThreadFailedToSpawn(String),
//    #[error("unable to send on channel")]
//    SendError,
//    #[error("unable to recv on channel")]
//    RecvError,
//}

// TODO: make this make more sense
pub fn assert_wav_spec_from_configs(config: &cpal::SupportedStreamConfig, spec: storage::Mp3Spec) -> bool {
    let sample_rate = config.sample_rate().0;
    let sample_size = config.sample_format().sample_size();
    let channels = config.channels();
    spec.sample_rate == sample_rate as usize && spec.bits_per_sample as usize == sample_size * 8
}

// FUTURE: support mismatching sample_rate, sample_size
//fn wav_spec_from_configs(config_a: &cpal::SupportedStreamConfig, config_b: &cpal::SupportedStreamConfig) -> Result<hound::WavSpec, RecorderError> {
pub fn wav_spec_from_configs(config_a: &cpal::SupportedStreamConfig, config_b: &cpal::SupportedStreamConfig) -> Result<storage::Mp3Spec, RecorderError> {
    let sample_rate_a = config_a.sample_rate().0;
    let sample_rate_b = config_b.sample_rate().0;
    if sample_rate_a != sample_rate_b {
        return Err(RecorderError::UnsupportedSampleRate(sample_rate_a, sample_rate_b))
    }

    // sample size is also determined by frame size i think
    let sample_size_a = config_a.sample_format().sample_size();
    let sample_size_b = config_b.sample_format().sample_size();
    if sample_size_a != sample_size_b {
        return Err(RecorderError::UnsupportedSampleSize(sample_size_a, sample_size_b))
    }
    //let sample_size = std::cmp::min(sample_size_a, sample_size_b);

    //// HOUND
    //Ok(hound::WavSpec {
    //    channels: 2,
    //    sample_rate: sample_rate_a as _,
    //    bits_per_sample: (sample_size_a * 8) as _,
    //    sample_format: sample_format(config_a.sample_format()),
    //})
    // MP3
    Ok(storage::Mp3Spec {
        channels: 2,
        sample_rate: sample_rate_a as _,
        bits_per_sample: (sample_size_a * 8) as _,
        //sample_format: sample_format(config_a.sample_format()),
    })
}


//#[derive(Clone, Debug)]
//pub enum Command<T> {
//    Frame(Vec<T>),
//    //Error(usize),
//    Stop,
//}
//#[derive(Clone, Debug)]
//pub enum Command {
//    Frame(Vec<f32>),
//    Stop,
//}

struct RunContext {
    recv_speaker: Receiver<Command>,
    recv_mic: Receiver<Command>,
    chained_sender: Sender<Command>,
    ui_sender: Sender<UiUpdate>,
}

pub struct Controller {
    thread: Option<JoinHandle<()>>,
    commands_speaker: Sender<Command>,
    commands_mic: Sender<Command>,
}

impl Drop for Controller {
    #[inline]
    fn drop(&mut self) {
        if self.push_command(Command::Stop).is_ok() {
            self.thread.take().unwrap().join().unwrap();
        }
    }
}

impl Controller {
    // chained sender is the next Stream in the chain
    pub fn new<E>(
        chained_sender: Sender<Command>, 
        ui_sender: Sender<UiUpdate>, 
        error_callback: E
    ) -> Controller
    where E: FnMut(StreamError) + Send + 'static,
    {
        //let (tx, rx) = channel::<Command>();
        let (commands_speaker, recv_speaker) = channel();
        let (commands_mic, recv_mic) = channel();
        let run_context = RunContext{
            recv_speaker,
            recv_mic,
            chained_sender,
            ui_sender,
        };

        let thread = thread::Builder::new()
            //.name("cpal_wasapi_in".to_owned())
            //.spawn(move || controller::write_frame(ro, ri, &writer_2))
            .name("controller".to_owned())
            .spawn(move || run_input(run_context, error_callback))
            .unwrap();

        Controller{
            thread: Some(thread),
            //commands: tx,
            commands_speaker,
            commands_mic,
        }
    }

    //fn get_senders(&self) -> (Sender<Command>, Sender<Command>) {
    //    (self.commands_speaker.clone(), self.commands_mic.clone())
    //}
    //pub fn get_senders<F, G>(&self) -> (F, G) 
    //where 
    //    F: FnMut(Vec<f32>) + Send + Sync,
    //    G: FnMut(Vec<f32>) + Send + Sync,
    pub fn get_senders<'a, 'b, 'c>(&'a self) -> (impl FnMut(Vec<f32>) + use<'b>, impl FnMut(Vec<f32>) + use<'c>) 
    {
        let t_speaker = self.commands_speaker.clone();
        let sender_speaker = move |v| {
            if let Err(e) = t_speaker.send(Command::Frame(v)) {
                println!("[sender_speaker] failed with: {}", e);
            };
        };

        let t_mic = self.commands_mic.clone();
        let sender_mic = move |v| {
            //t_mic.send(Command::Frame(v));
            if let Err(e) = t_mic.send(Command::Frame(v)) {
                println!("[sender_mic] failed with: {}", e);
            };
        };
        (sender_speaker, sender_mic)
    }

    pub fn get_senders_2(&self) -> (Sender<Command>, Sender<Command>) 
    {
        let t_speaker = self.commands_speaker.clone();
        let t_mic = self.commands_mic.clone();
        (t_speaker, t_mic)
    }

    // TODO: failed to send error, see storage, bring storage error out
    fn push_command(&self, command: Command) -> Result<(), SendError<Command>> {
        //self.commands.send(command)
        //do i really want to clone? the Stop sent here is likely cheap though
        self.commands_speaker.send(command.clone())?;
        self.commands_mic.send(command)?;
        Ok(())
    }
}


//pub fn send_frame<T: cpal::Sample>(input: &[T], chan: Sender<Command<T>>) {
//  //if let Err(_) = chan.send(Command::Frame(input.to_vec::<Vec<f32>>())) {
//  if let Err(_) = chan.send(Command::Frame(input.to_vec::<Vec<T>>())) {
//      //Task::done(Message::Error(RecorderError::StreamFrameSendError));
//  }
//}

pub type WavWriterHandle = Arc<Mutex<Option<streams::WriterStream>>>;
//type WavWriterHandle = Arc<Mutex<Option<storage::Mp3Writer<BufWriter<File>>>>>;
//type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;
fn run_input<E>(run_context: RunContext, mut error_callback: E) 
where E: FnMut(StreamError) + Send + 'static,
{
    let mut count = 0;
    loop {
        count = (count + 1) % 4000;
        if count == 99 {
            println!("[write_frame] {}", count);
        }
        let sample_a = match run_context.recv_speaker.recv() {
            Ok(Command::Frame(a)) => a,
            Err(e) => { 
                error_callback(StreamError::RecvError);
                eprintln!("error receiving from speaker: {}", e);
                break;
            },
            Ok(Command::Stop) => break, 
        };
        let sample_b = match run_context.recv_mic.recv() {
            Ok(Command::Frame(b)) => b,
            Err(e) => { 
                error_callback(StreamError::RecvError);
                eprintln!("error receiving from speaker: {}", e);
                break;
            },
            Ok(Command::Stop) => break, //return Ok(false), 
        };
        let mut a_iter = sample_a.into_iter();
        let mut b_iter = sample_b.into_iter();
        let mut frame = Vec::new();

        loop {
            let a0 = match a_iter.next() {
                Some(a) => a,
                None => break,
            };
            let a1 = match a_iter.next() {
                Some(a) => a,
                None => break,
            };
            let b0 = match b_iter.next() {
                Some(b) => b,
                None => break,
            };
            let b1 = match b_iter.next() {
                Some(b) => b,
                None => break,
            };

            // FIX THIS
            //let s0 = a0.add_amp(b0);
            //let s1 = a1.add_amp(b1);
            let s0 = a0 + b0;
            let s1 = a1 + b1;

            frame.push(s0);
            frame.push(s1);

            if let None = b_iter.next() { break }
            if let None = b_iter.next() { break }
        }
        let ui_sample = frame[0];
        if let Err(e) = run_context.chained_sender.send(Command::Frame(frame)) {
            error_callback(StreamError::SendError);
            eprintln!("error sending to chained sender");
            break;
            //return Err(RecorderError::StorageStreamWriteError(e.to_string()));
        }
        if let Err(e) = run_context.ui_sender.send(UiUpdate::Pulse(ui_sample)) {
            println!("[write-frame] error sending ui sample: {}", ui_sample);
        }
    }
    println!("break for reasons");
    // cleanup here
    run_context.ui_sender.send(UiUpdate::Stop).unwrap();
}

// FUTURE: use a builder to generate ?
pub fn write_frame(ra: Receiver<Command>, rb: Receiver<Command>, writer: &WavWriterHandle) -> Result<bool, RecorderError>
//pub fn write_frame(ra: mpsc::Receiver<Command<f32>>, rb: mpsc::Receiver<StreamCommand<f32>>, writer: &WavWriterHandle) -> Result<bool, RecorderError>
//fn write_frame<T>(ra: mpsc::Receiver<Command<T>>, rb: mpsc::Receiver<StreamCommand<T>>, writer: &WavWriterHandle) -> Result<bool, RecorderError>
//where
//    T: cpal::Sample<Signed = T> + hound::Sample, // + std::fmt::Display,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            loop {
                let sample_a = match ra.recv() {
                    Ok(Command::Frame(a)) => a,
                    Err(e) => return Err(RecorderError::StreamFrameRecvError(e, "a2".to_string())),
                    Ok(Command::Stop) => return Ok(false), 
                };
                let sample_b = match rb.recv() {
                    Ok(Command::Frame(b)) => b,
                    Err(e) => return Err(RecorderError::StreamFrameRecvError(e, "b4".to_string())),
                    Ok(Command::Stop) => return Ok(false), 
                };

                // TODO some way to customize it
                let mut a_iter = sample_a.into_iter();
                let mut b_iter = sample_b.into_iter();
                let mut frame = Vec::new();
                loop {
                    let a0 = match a_iter.next() {
                        Some(a) => a,
                        None => break,
                    };
                    let a1 = match a_iter.next() {
                        Some(a) => a,
                        None => break,
                    };
                    let b0 = match b_iter.next() {
                        Some(b) => b,
                        None => break,
                    };
                    let b1 = match b_iter.next() {
                        Some(b) => b,
                        None => break,
                    };

                    // FIX THIS
                    //let s0 = a0.add_amp(b0);
                    //let s1 = a1.add_amp(b1);
                    let s0 = a0 + b0;
                    let s1 = a1 + b1;
                    //writer.write_sample(s0).ok();
                    //writer.write_sample(s1).ok();
                    frame.push(s0);
                    frame.push(s1);

                    if let None = b_iter.next() { break }
                    if let None = b_iter.next() { break }
                }
                if let Err(e) = writer.write_frame(frame) {
                    return Err(RecorderError::StorageStreamWriteError(e.to_string()));
                }
            };
        }
    }
    return Ok(false);
}
