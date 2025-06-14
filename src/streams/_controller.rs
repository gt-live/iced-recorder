use crate::error::RecorderError;
use crate::storage;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::io::{self, BufWriter};
use std::fs::{self, File};

#[derive(Clone)]
enum StreamCommand<T> {
    Frame(Vec<T>),
    //Error(usize),
    Stop,
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

pub fn send_frame<T: cpal::Sample>(input: &[T], chan: mpsc::Sender<StreamCommand<T>>) {
  if let Err(_) = chan.send(StreamCommand::Frame(input.to_vec())) {
      //Task::done(Message::Error(RecorderError::StreamFrameSendError));
  }
}

type WavWriterHandle = Arc<Mutex<Option<storage::Mp3Writer<BufWriter<File>>>>>;
//type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

// FUTURE: use a builder to generate ?
pub fn write_frame(ra: mpsc::Receiver<StreamCommand<f32>>, rb: mpsc::Receiver<StreamCommand<f32>>, writer: &WavWriterHandle) -> Result<bool, RecorderError>
//fn write_frame<T>(ra: mpsc::Receiver<StreamCommand<T>>, rb: mpsc::Receiver<StreamCommand<T>>, writer: &WavWriterHandle) -> Result<bool, RecorderError>
//where
//    T: cpal::Sample<Signed = T> + hound::Sample, // + std::fmt::Display,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            loop {
                let sample_a = match ra.recv() {
                    Ok(StreamCommand::Frame(a)) => a,
                    Err(e) => return Err(RecorderError::StreamFrameRecvError(e, "a2".to_string())),
                    Ok(StreamCommand::Stop) => return Ok(false), 
                };
                let sample_b = match rb.recv() {
                    Ok(StreamCommand::Frame(b)) => b,
                    Err(e) => return Err(RecorderError::StreamFrameRecvError(e, "b4".to_string())),
                    Ok(StreamCommand::Stop) => return Ok(false), 
                };

                // TODO some way to customize it
                let mut a_iter = sample_a.into_iter();
                let mut b_iter = sample_b.into_iter();
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
                    writer.write_sample(s0).ok();
                    writer.write_sample(s1).ok();

                    if let None = b_iter.next() { break }
                    if let None = b_iter.next() { break }
                }
            };
        }
    }
    return Ok(false);
}
