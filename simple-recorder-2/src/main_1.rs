//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use cpal::{InputCallbackInfo, OutputCallbackInfo};
use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::thread::{self, JoinHandle};

#[derive(Parser, Debug)]
#[command(version, about = "CPAL record_wav example", long_about = None)]
struct Opt {
    /// The audio device to use
    #[arg(short, long, default_value_t = String::from("default"))]
    input_device: String,

    #[arg(short, long, default_value_t = String::from("default"))]
    output_device: String,

    /// Use the JACK host
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
    #[arg(short, long)]
    #[allow(dead_code)]
    jack: bool,
}


fn get_host(opt: &Opt) -> Host {
    // Conditionally compile with jack if the feature is specified.
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
    // Manually check for flags. Can be passed through cargo with -- e.g.
    // cargo run --release --example beep --features jack -- --jack
    let host = if opt.jack {
        cpal::host_from_id(cpal::available_hosts()
            .into_iter()
            .find(|id| *id == cpal::HostId::Jack)
            .expect(
                "make sure --features jack is specified. only works on OSes where jack is available",
            )).expect("jack host unavailable")
    } else {
        cpal::default_host()
    };

    #[cfg(any(
        not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        )),
        not(feature = "jack")
    ))]
    let host = cpal::default_host();
    return host;
}

fn print_devices(host: &Host) -> Result<(), anyhow::Error> {
    println!("XXXXXX Listing Input Devices XXXXXX");
    host.input_devices()?
        .for_each(|x| println!("input_device={}", x.name().unwrap_or("error".to_string()))); // .for_each(|y| println!("input_device={}", y ));
    println!("XXXXXX Listing Output Devices XXXXXX");
    host.output_devices()?
        .for_each(|x| println!("output_device={}", x.name().unwrap_or("error".to_string())));
    println!("XXXXXX End XXXXXX");
    Ok(())
}

fn get_input_device(opt: &Opt, host: &Host) -> Result<Device, anyhow::Error> {
    // Set up the input device and stream with the default input config.
    let device = if opt.input_device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == opt.input_device).unwrap_or(false))
    }
    .expect("failed to find input device");
    Ok(device)
}

// will output_device ever change?
fn get_output_device(opt: &Opt, host: &Host) -> Result<Device, anyhow::Error> {
    // Set up the output device and stream with the default output config.
    let device = if opt.output_device == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?
            .find(|x| x.name().map(|y| y == opt.output_device).unwrap_or(false))
    }
    .expect("failed to find output device");
    Ok(device)
}

// TODO: make this proper
// For now use config_a for the one with channel of 2
fn wav_spec_from_configs(config_a: &cpal::SupportedStreamConfig, config_b: &cpal::SupportedStreamConfig) -> Result<hound::WavSpec, anyhow::Error> {
    let sample_rate_a = config_a.sample_rate().0;
    let sample_rate_b = config_b.sample_rate().0;
    if sample_rate_a != sample_rate_b {
        // this will be calc properly next time
        return Err(anyhow!("Sample rate does not match: a={}, b={}", sample_rate_a, sample_rate_b));
    }

    let sample_size_a = config_a.sample_format().sample_size();
    let sample_size_b = config_b.sample_format().sample_size();
    if sample_size_a != sample_size_b {
        print!("Sample size does not match: a={}, b={}", sample_size_a, sample_size_b);
        // sample size is also determined by frame size i think
        //return Err(anyhow!("Sample size does not match: a={}, b={}", sample_size_a, sample_size_b));
    }
    let sample_size = std::cmp::min(sample_size_a, sample_size_b) * 8;

    Ok(hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate_a as _,
        bits_per_sample: (sample_size * 8) as _,
        sample_format: sample_format(config_a.sample_format()), // TODO: fix this
    })
}

fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::parse();
    let host = get_host(&opt);

    let _ = print_devices(&host)?;
    let input_device = get_input_device(&opt, &host)?;
    let output_device = get_output_device(&opt, &host)?;
    println!("Input device: {}", input_device.name()?);
    println!("Output device: {}", output_device.name()?);

    let input_config = input_device
        .default_input_config()
        .expect("Failed to get default input config");
    println!("Default input config: {:?}", input_config);
    let output_config = output_device
        .default_output_config()
        .expect("Failed to get default output config");
    println!("Default output config: {:?}", output_config);

    // The WAV file we're recording to.
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    let spec = wav_spec_from_configs(&input_config, &output_config)?;
    let writer = hound::WavWriter::create(PATH, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));


    // 1. Create channel to for stream_a
    let (ti, ri) = channel();
    // 2. Create channel to for stream_b
    let (to, ro) = channel();
    // 3. Create closure for stream_a
    // 3. counter for stream_b for channels
    // WAIT need to deal with 4chan and 2chan audio
    // WRAP ti and to
    // https://users.rust-lang.org/t/moving-a-sender-into-a-fn-closure/33875/5
    //let ti_lock = Mutex::new(Some(ti));


    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    // Run the input stream on a separate thread.
    let writer_2 = writer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let input_stream = match input_config.sample_format() {
        cpal::SampleFormat::I8 => {
            println!("Unsupported format I8");
            return Err(anyhow!("Unsupported format I8"));
        },
        cpal::SampleFormat::I16 => {
            println!("Unsupported format I16");
            return Err(anyhow!("Unsupported format I16"));
        },
        cpal::SampleFormat::I32 => {
            println!("Unsupported format I32");
            return Err(anyhow!("Unsupported format I32"));
        },
        cpal::SampleFormat::F32 => {
            input_device.build_input_stream(
                &input_config.into(),
                move |data, info: &_| {send_frame::<f32, f32>(data, info, ti.clone())},
                err_fn,
                None,
            )?
        },
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format '{sample_format}'"
            )))
        },
    };

    let output_stream = match output_config.sample_format() {
        cpal::SampleFormat::I8 => {
            println!("Unsupported format I8");
            return Err(anyhow!("Unsupported format I8"));
        },
        cpal::SampleFormat::I16 => {
            println!("Unsupported format I16");
            return Err(anyhow!("Unsupported format I16"));
        },
        cpal::SampleFormat::I32 => {
            println!("Unsupported format I32");
            return Err(anyhow!("Unsupported format I32"));
        },
        cpal::SampleFormat::F32 => {
            output_device.build_input_stream(
                &output_config.into(),
                move |data, info: &_| send_frame_out::<f32, f32>(data, info, to.clone()),
                err_fn,
                None,
            )?
        },
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format '{sample_format}'"
            )))
        },
    };



    //let stream = match input_config.sample_format() {
    //    cpal::SampleFormat::I8 => {
    //        input_device.build_input_stream(
    //            &input_config.into(),
    //            move |data, info: &_| send_frame::<i8, i8>(data, info, ti),
    //            //move |data, _: &_| write_input_data::<i8, i8>(data, &writer_2),
    //            err_fn,
    //            None,
    //        )?
    //    },
    //    cpal::SampleFormat::I16 => input_device.build_input_stream(
    //        &input_config.into(),
    //        //move |data, _: &_| write_input_data::<i16, i16>(data, &writer_2),
    //        move |data, info: &_| send_frame::<i16, i16>(data, info, ti),
    //        err_fn,
    //        None,
    //    )?,
    //    cpal::SampleFormat::I32 => input_device.build_input_stream(
    //        &input_config.into(),
    //        move |data, _: &_| write_input_data::<i32, i32>(data, &writer_2),
    //        err_fn,
    //        None,
    //    )?,
    //    cpal::SampleFormat::F32 => input_device.build_input_stream(
    //        &input_config.into(),
    //        move |data, _: &_| write_input_data::<f32, f32>(data, &writer_2),
    //        err_fn,
    //        None,
    //    )?,
    //    sample_format => {
    //        return Err(anyhow::Error::msg(format!(
    //            "Unsupported sample format '{sample_format}'"
    //        )))
    //    }
    //};

    //stream.play()?;
    //write_frame(ri, ro, &writer_2);

    let thread = thread::Builder::new()
        .name("cpal_wasapi_in".to_owned())
        .spawn(move || write_frame(ri, ro, &writer_2))
        //.spawn(move || run_input(run_context, &mut data_callback, &mut error_callback))
        .unwrap();

    input_stream.play()?;
    output_stream.play()?;


    // Let recording go for roughly three seconds.
    std::thread::sleep(std::time::Duration::from_secs(3));
    drop(input_stream);
    drop(output_stream);


    writer.lock().unwrap().take().unwrap().finalize()?;
    println!("Recording {} complete!", PATH);
    Ok(())
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

enum Command<T> {
    Frame(T, T),
    Error(usize),
    Stop,
}

fn send_frame_out<T, U>(input: &[T], info: &OutputCallbackInfo, chan: Sender<Command<U>>)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T> + std::fmt::Display,
{
  println!("info={:?}", info.timestamp());
  if input.len() < 2 {
      chan.send(Command::Error(input.len())).unwrap();
      return;
  }
  chan.send(
    Command::Frame(
      U::from_sample(input[0]),
      U::from_sample(input[1]),
    )
  ).unwrap();
}

fn send_frame<T, U>(input: &[T], info: &InputCallbackInfo, chan: Sender<Command<U>>)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T> + std::fmt::Display,
{
  println!("info={:?}", info.timestamp());
  if input.len() < 2 {
      chan.send(Command::Error(input.len())).unwrap();
      return;
  }
  chan.send(
    Command::Frame(
      U::from_sample(input[0]),
      U::from_sample(input[1]),
    )
  ).unwrap();
}

fn write_frame<T, U>(ra: Receiver<Command<T>>, rb: Receiver<Command<U>>, writer: &WavWriterHandle) -> Result<bool, anyhow::Error>
where
    T: Sample + hound::Sample + std::fmt::Display,
    U: Sample + hound::Sample + std::fmt::Display,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            //writer.write_sample(sample).ok();
            loop {
                match ra.recv() {
                    Err(e) => {
                        println!("error recv_a: {}", e);
                        return Ok(false);
                    },
                    Ok(Command::Stop) => {
                        return Ok(false);
                    },
                    Ok(Command::Error(e)) => {
                        println!("error command_a: {}", e);
                        return Ok(false);
                    },
                    Ok(Command::Frame(a0, a1)) => {
                        match rb.recv() {
                            Err(e) => {
                                println!("error b: {}", e);
                                return Ok(false);
                            },
                            Ok(Command::Stop) => {
                                return Ok(false);
                            },
                            Ok(Command::Error(e)) => {
                                println!("error command_b: {}", e);
                                return Ok(false);
                            },
                            Ok(Command::Frame(b0, b1)) => {
                                let s0 = a0.as_i16() + b0.as_i16();
                                let s1 = a1.as_i16() + b1.as_i16();
                                writer.write_sample(s0).ok();
                                writer.write_sample(s1).ok();
                            },
                        };
                    },
                };
            };
        }
    }
    return Ok(false);
}

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T> + std::fmt::Display,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                println!("sample={}", sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}
