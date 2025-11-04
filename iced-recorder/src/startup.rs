use cpal::platform::Host;
//use clap::Parser;
//use cpal::{InputCallbackInfo, OutputCallbackInfo};
//use cpal::platform::{Device, Host};
//use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
//use cpal::{FromSample, Sample};
//use std::fs::File;
//use std::io::BufWriter;
//use std::sync::{Arc, Mutex};
//use anyhow::anyhow;
//use std::sync::mpsc::{channel, Receiver, SendError, Sender};
//use std::thread::{self, JoinHandle};


//#[derive(Parser, Debug)]
//#[command(version, about = "CPAL record_wav example", long_about = None)]
//pub struct Opt {
//    /// The audio device to use
//    #[arg(short, long, default_value_t = String::from("default"))]
//    input_device: String,
//
//    #[arg(short, long, default_value_t = String::from("default"))]
//    output_device: String,
//
//    /// Use the JACK host
//    #[cfg(all(
//        any(
//            target_os = "linux",
//            target_os = "dragonfly",
//            target_os = "freebsd",
//            target_os = "netbsd"
//        ),
//        feature = "jack"
//    ))]
//    #[arg(short, long)]
//    #[allow(dead_code)]
//    jack: bool,
//}


//pub fn get_host(opt: &Opt) -> Host {
pub fn get_host() -> Host {
    //// Conditionally compile with jack if the feature is specified.
    //#[cfg(all(
    //    any(
    //        target_os = "linux",
    //        target_os = "dragonfly",
    //        target_os = "freebsd",
    //        target_os = "netbsd"
    //    ),
    //    feature = "jack"
    //))]
    //// Manually check for flags. Can be passed through cargo with -- e.g.
    //// cargo run --release --example beep --features jack -- --jack
    //let host = if opt.jack {
    //    cpal::host_from_id(cpal::available_hosts()
    //        .into_iter()
    //        .find(|id| *id == cpal::HostId::Jack)
    //        .expect(
    //            "make sure --features jack is specified. only works on OSes where jack is available",
    //        )).expect("jack host unavailable")
    //} else {
    //    cpal::default_host()
    //};

    //#[cfg(any(
    //    not(any(
    //        target_os = "linux",
    //        target_os = "dragonfly",
    //        target_os = "freebsd",
    //        target_os = "netbsd"
    //    )),
    //    not(feature = "jack")
    //))]
    let host = cpal::default_host();
    return host;
}

