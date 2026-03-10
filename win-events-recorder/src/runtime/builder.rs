use std::sync::mpsc::Sender;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SupportedStreamConfig;
use crate::streams;
use crate::runtime::{self, RuntimeController, RuntimeError};
use crate::settings::{DeviceName, InputDeviceSelection, OutputDeviceSelection};



#[derive(thiserror::Error, Debug, Clone)]
pub enum RuntimeBuilderError {
    #[error("unable get {1} device: {0}")]
    GetIoDeviceError(String, String),
    #[error("unable get device config: {0}")]
    GetIoDeviceConfigError(String),
    #[error("unable build device stream: {0}")]
    BuildStreamError(String),
    #[error("Unsupported {1} sample format: {0}")]
    UnsupportedSampleFormat(cpal::SampleFormat, String),
    #[error("Error playing stream: {0}")]
    PlayStreamError(String),

    #[error("Missing writer_sender")]
    MissingWriterSender,
    #[error("Missing app_sender")]
    MissingUiSender,
    #[error("Missing runtime events_sender")]
    MissingEventsSender,
    #[error("Invalid writer_config: {0}")]
    InvalidWriterConfig(String),
}

pub(crate) struct ContextHead {
    // consider packaging these into a Head, that can be swapped out as needed
    //mic_stream: Option<cpal::Stream>,
    //speaker_stream: Option<cpal::Stream>,
    pub(crate) mic_stream: cpal::Stream,
    pub(crate) speaker_stream: cpal::Stream,
    pub(crate) bridge_thread: Option<streams::Controller>,
    //pub(crate) ui_thread: Option<task::Handle>,
    pub(crate) ui_sender: Sender<streams::UiUpdate>,

    //ui_receiver: Option<Receiver<streams::UiUpdate>>,
    mic_name: InputDeviceSelection,
    speaker_name: OutputDeviceSelection,

    // to make getting builder easier, do not actually use this please
    writer_sender: Sender<streams::Command>,
    events_sender: Sender<runtime::Event>,

    // data
    speaker_config: SupportedStreamConfig,
    mic_config: SupportedStreamConfig,
}

impl ContextHead {
    pub(crate) fn play(&mut self) -> Result<(), RuntimeError> {
        self.mic_stream.play().map_err(|error| RuntimeError::PlayStreamError(error.to_string()))?;
        self.speaker_stream.play().map_err(|error| RuntimeError::PlayStreamError(error.to_string()))?;
        Ok(())
    }

    pub(crate) fn get_mic_config(&self) -> SupportedStreamConfig { self.mic_config.clone() }
    pub(crate) fn get_speaker_config(&self) -> SupportedStreamConfig { self.speaker_config.clone() }

    pub(crate) fn get_builder(&self) -> ContextHeadBuilder {
        ContextHeadBuilder {
            mic_name: self.mic_name.clone(),
            speaker_name: self.speaker_name.clone(),
            //mic_name: "DEFAULT".to_string(),
            //speaker_name: "DEFAULT".to_string(),
            writer_sender: Some(self.writer_sender.clone()),
            events_sender: Some(self.events_sender.clone()),
            app_sender: Some(self.ui_sender.clone()),
        }
    }
}

impl Drop for ContextHead {
    #[inline]
    fn drop(&mut self) {
        drop(self.bridge_thread.take())
        //drop(self.speaker_stream)
        //if self.push_command(Command::Stop).is_ok() {
        //    self.thread.take().unwrap().join().unwrap();
        //}
    }
}

pub(crate) struct ContextHeadBuilder {
    mic_name: InputDeviceSelection, 
    speaker_name: OutputDeviceSelection,

    app_sender: Option<Sender<streams::UiUpdate>>,
    events_sender: Option<Sender<runtime::Event>>,
    writer_sender: Option<Sender<streams::Command>>,
    //writer_stream: Option<streams::WriterStream>,
}

impl Default for ContextHeadBuilder {
    fn default() -> Self {
        ContextHeadBuilder {
            mic_name: InputDeviceSelection::Default,
            speaker_name: OutputDeviceSelection::Default,
            //mic_name: "DEFAULT".to_string(),
            //speaker_name: "DEFAULT".to_string(),
            writer_sender: None,
            events_sender: None,
            app_sender: None,
        }
    }
}

//struct ContextHeadBuilderArg {
//    mic_name: String,
//    speaker_name: String,
//}

impl ContextHeadBuilder {
    pub(crate) fn set_devices(mut self, mic_name: &str, speaker_name: &str) -> Self {
        //self.mic_name = mic_name.to_string();
        //self.speaker_name = speaker_name.to_string();
        //self
        Self {
            mic_name: InputDeviceSelection::Selected(DeviceName(mic_name.to_string())),
            speaker_name: OutputDeviceSelection::Selected(DeviceName(speaker_name.to_string())),
            ..self
        }
    }

    pub(crate) fn set_writer_sender(mut self, sender: Sender<streams::Command>) -> Self {
        Self {
            writer_sender: Some(sender),
            ..self
        }
    }

    pub(crate) fn set_events_sender(mut self, sender: Sender<runtime::Event>) -> Self {
        Self {
            events_sender: Some(sender),
            ..self
        }
    }

    pub(crate) fn set_app_sender(mut self, sender: Sender<streams::UiUpdate>) -> Self {
        Self {
            app_sender: Some(sender),
            ..self
        }
    }

    pub(crate) fn try_build(mut self) -> Result<ContextHead, RuntimeBuilderError> {
        let (mic_device, speaker_device) = Self::try_devices(&self.mic_name, &self.speaker_name)?;
        let mic_config = mic_device.default_input_config()
            .map_err(|error| RuntimeBuilderError::GetIoDeviceConfigError(error.to_string()))?;
        let speaker_config = mic_device.default_output_config()
            .map_err(|error| RuntimeBuilderError::GetIoDeviceConfigError(error.to_string()))?;

        //let writer = streams::WriterStream::new(spec, self.recording_path.clone(), err_fn);
        let writer_sender = match self.writer_sender.take() {
            Some(sender) => sender,
            None => return Err(RuntimeBuilderError::MissingWriterSender),
        };

        //let (ui_sender, ui_receiver) = channel();
        //let to_ui_err_fn = streams::gen_audio_err_fn(ui_sender.clone(), "both");
        let app_sender = match self.app_sender.take() {
            Some(sender) => sender,
            None => return Err(RuntimeBuilderError::MissingUiSender),
        };
        let events_sender = match self.events_sender.take() {
            Some(sender) => sender,
            None => return Err(RuntimeBuilderError::MissingEventsSender),
        };

        let err_fn = |err| eprintln!("[control_stream] an error occurred on stream: {}", err);
        let controller = streams::Controller::new(writer_sender, app_sender.clone(), err_fn);
        let (mut sender_speaker, mut sender_mic) = controller.get_senders();

        let to_ui_err_fn = runtime::gen_audio_err_fn(events_sender.clone(), "both");
        let mic_stream = match mic_config.sample_format() {
            cpal::SampleFormat::F32 => {
                mic_device.build_input_stream(
                    &mic_config.into(),
                    move |data, _| sender_mic(data.to_vec()),
                    //err_fn,
                    to_ui_err_fn,
                    None,
                ).map_err(|error| RuntimeBuilderError::BuildStreamError(error.to_string()))?
            },
            sample_format =>  return Err(RuntimeBuilderError::UnsupportedSampleFormat(sample_format, "input".to_string())),
        };

        //let to_ui_err_fn = streams::gen_audio_err_fn(app_sender.clone(), "both");
        let to_ui_err_fn = runtime::gen_audio_err_fn(events_sender.clone(), "both");
        let speaker_stream = match speaker_config.sample_format() {
            cpal::SampleFormat::F32 => {
                speaker_device.build_input_stream(
                    &speaker_config.into(),
                    move |data, _| sender_speaker(data.to_vec()),
                    //err_fn,
                    to_ui_err_fn,
                    None,
                ).map_err(|error| RuntimeBuilderError::BuildStreamError(error.to_string()))?
            },
            sample_format =>  return Err(RuntimeBuilderError::UnsupportedSampleFormat(sample_format, "output".to_string())),
        };

        // UI Thread

        Ok(ContextHead {
            mic_stream,
            speaker_stream,
            bridge_thread: Some(controller),
            ui_sender: app_sender,

            //mic_name: InputDeviceSelection::Selected(DeviceName(self.mic_name.clone())),
            mic_name: self.mic_name.clone(),
            speaker_name: self.speaker_name.clone(),
            //speaker_name: OutputDeviceSelection::Selected(self.speaker_name.clone()),

            // for builder
            events_sender,
            writer_sender,

            // for data
            mic_config,
            speaker_config,
        })
    }

    fn try_devices(mic_name: InputDeviceSelection, speaker_name: OutputDeviceSelection) -> Result<(cpal::Device, cpal::Device), RuntimeBuilderError> {
        match (mic_name, speaker_name) {
            //(InputDeviceSelection::Selected(input), OutputDeviceSelection::Selected(output)) => return Self::try_named_devices(&input.to_string(), &output.to_string()),
            (InputDeviceSelection::Selected(input), OutputDeviceSelection::Selected(output)) => return Self::try_named_devices(input.as_ref(), output.as_ref()),
            (_, _) => return Self::try_default_devices(),
        }
        //if mic_name == InputDeviceSelection::Default || speaker_name == OutputDeviceSelection::Default {
        //    return Self::try_default_devices();
        //}
        //return Self::try_named_devices(mic_name, speaker_name);
    }

    fn try_named_devices(mic_name: &str, speaker_name: &str) -> Result<(cpal::Device, cpal::Device), RuntimeBuilderError> {
        let host = cpal::default_host();
        let mic_device = match host.input_devices()
            .map_err(|err| RuntimeBuilderError::GetIoDeviceError(err.to_string(), "input_devices".to_string()))?
            .find(|x| x.name().map(|y| y == mic_name).unwrap_or(false)) {
            Some(device) => device,
            None => return Err(RuntimeBuilderError::GetIoDeviceError("requested input device not found".to_string(), "input_device".to_string())),
        };
        let speaker_device = match host.output_devices()
            .map_err(|err| RuntimeBuilderError::GetIoDeviceError(err.to_string(), "output_devices".to_string()))?
            .find(|x| x.name().map(|y| y == speaker_name).unwrap_or(false)) {
            Some(device) => device,
            None => return Err(RuntimeBuilderError::GetIoDeviceError("requested output device not found".to_string(), "output_device".to_string())),
        };
        return Ok((mic_device, speaker_device))
    }

    fn try_default_devices() -> Result<(cpal::Device, cpal::Device), RuntimeBuilderError> {
        let host = cpal::default_host();
        let input_device = match host.default_input_device() {
            Some(device) => device,
            None => return Err(RuntimeBuilderError::GetIoDeviceError("no default device found".to_string(), "default_input".to_string())),
        };
        let output_device = match host.default_output_device() {
            Some(device) => device,
            None => return Err(RuntimeBuilderError::GetIoDeviceError("no default device found".to_string(), "default_output".to_string())),
        };
        return Ok((input_device, output_device))
    }
}
