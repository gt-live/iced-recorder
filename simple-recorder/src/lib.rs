use std::time::Duration;
use cpal::{BuildStreamError, Data, InputCallbackInfo, SampleFormat, StreamConfig, StreamError};
use cpal::platform::{Device, Stream, StreamInner, SupportedInputConfigs, SupportedOutputConfigs};

// Composite Device contains multiple device to stream from

/// An opaque type that identifies an end point.
#[derive(Clone)]
pub struct CompositeDevice {
    devices: Vec<Device>,
}

/// A device that is capable of audio input and/or output.
///
/// Please note that `Device`s may become invalid if they get disconnected. Therefore, all the
/// methods that involve a device return a `Result` allowing the user to handle this case.
pub trait CompositeDeviceTrait {
    // advanced traits (associated types are unstable)
    //type SupportedInputConfigs = SupportedInputConfigs;
    //type SupportedOutputConfigs = SupportedOutputConfigs;
    type Stream = Stream;

    /// Create a dynamically typed input stream.
    fn build_input_stream_raw<D, E>(
        &self,
        config: &StreamConfig,
        sample_format: SampleFormat,
        data_callback: D,
        error_callback: E,
        timeout: Option<Duration>,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        D: FnMut(&Data, &InputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static;
}

// CompositeStream
// different from DeviceTrait because I want sig of data_callback to be different
impl CompositeDeviceTrait for CompositeDevice {
// call Device.build_input_stream_raw_inner
    fn build_input_stream_raw<D, E>(
        &self,
        config: &StreamConfig,
        sample_format: SampleFormat,
        data_callback: D,
        error_callback: E,
        _timeout: Option<Duration>,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        D: FnMut(&Data, &InputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static,
    {
        let stream_inner = self.build_input_stream_raw_inner(config, sample_format)?;
        Ok(Stream::new_input(
            stream_inner,
            data_callback,
            error_callback,
        ))
    }
}

/////////////////////////// Composite Stream ////////////////////
//
//
//use std::sync::mpsc::{channel, Receiver, SendError, Sender};
//use std::thread::{self, JoinHandle};
//use windows::Win32::Foundation;
//use windows::Win32::Foundation::HANDLE;
//use windows::Win32::System::Threading;
//
//pub struct CompositeStream {
//    /// The high-priority audio processing thread calling callbacks.
//    /// Option used for moving out in destructor.
//    ///
//    /// TODO: Actually set the thread priority.
//    thread: Option<JoinHandle<()>>,
//
//    // Commands processed by the `run()` method that is currently running.
//    // `pending_scheduled_event` must be signalled whenever a command is added here, so that it
//    // will get picked up.
//    commands: Sender<Command>,
//
//    // This event is signalled after a new entry is added to `commands`, so that the `run()`
//    // method can be notified.
//    pending_scheduled_event: Foundation::HANDLE,
//}
//
//impl CompositeStream {
//    pub(crate) fn new_input<D, E>(
//        stream_inner: StreamInner,
//        mut data_callback: D,
//        mut error_callback: E,
//    ) -> CompositeStream
//    where
//        D: FnMut(&Data, &InputCallbackInfo) + Send + 'static,
//        E: FnMut(StreamError) + Send + 'static,
//    {
//        let pending_scheduled_event = unsafe {
//            Threading::CreateEventA(None, false, false, windows::core::PCSTR(ptr::null()))
//        }
//        .expect("cpal: could not create input stream event");
//        let (tx, rx) = channel();
//
//        let run_context = RunContext {
//            handles: vec![pending_scheduled_event, stream_inner.event],
//            stream: stream_inner,
//            commands: rx,
//        };
//
//        let thread = thread::Builder::new()
//            .name("cpal_wasapi_in".to_owned())
//            .spawn(move || run_input(run_context, &mut data_callback, &mut error_callback))
//            .unwrap();
//
//        CompositeStream {
//            thread: Some(thread),
//            commands: tx,
//            pending_scheduled_event,
//        }
//    }
//
//}
//
//
//
//
//
//
//
//
//
//
//
//
//
//

