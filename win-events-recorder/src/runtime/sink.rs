use std::sync::mpsc::{Receiver, Sender};
use std::path;
use crate::streams;
use crate::settings;
use crate::storage;
use crate::runtime::{self, RuntimeController, RuntimeError, RuntimeBuilderError};

pub(crate) struct ContextTail {
    pub(crate) writer_thread: streams::WriterStream,

    write_spec: storage::Mp3Spec,
}

impl ContextTail {
    pub(crate) fn get_writer_sender(&self) -> Sender<streams::Command> {
        self.writer_thread.get_sender()
    }
    // if i need to add more, might be more worth it to generate another to_builder()
    pub(crate) fn get_writer_spec(&self) -> storage::Mp3Spec {
        self.write_spec.clone()
    }

    pub(crate) fn new(args: ContextTailBuilderArgs) -> ContextTail {
    }
}

pub(crate) struct ContextTailBuilder {
  write_path: path::PathBuf,
  write_spec: Option<storage::Mp3Spec>,
  writer_receiver: Option<Receiver<streams::Command>>,
}

pub (crate) struct ContextTailBuilderArgs {
    pub (crate) write_path: path::PathBuf,
    pub (crate) write_spec: storage::Mp3Spec,
    pub (crate) write_recv: Receiver<streams::Command>,
}

impl Default for ContextTailBuilder {
    fn default() -> Self {
        ContextTailBuilder {
            write_path: path::PathBuf::from(settings::PATH),
            writer_receiver: None,
            write_spec: None,
            //write_spec: storage::Mp3Spec {
            //    channels: 2,
            //    sample_rate: usize,
            //    bits_per_sample: u16,
            //},
        }
    }
}

impl ContextTailBuilder {
    pub(crate) fn set_write_path(self, write_path: path::PathBuf) -> Self {
        Self {
            write_path,
            ..self
        }
    }
    pub(crate) fn set_write_spec(self, write_spec: storage::Mp3Spec) -> Self {
        Self {
            write_spec: Some(write_spec),
            ..self
        }
    }
    pub(crate) fn set_write_spec(self, write_spec: storage::Mp3Spec) -> Self {
        Self {
            write_spec: Some(write_spec),
            ..self
        }
    }

    pub(crate) fn try_build(mut self) -> Result<ContextTail, RuntimeBuilderError> {
        let spec = match self.write_spec {
            Some(spec) => spec,
            None => return Err(RuntimeBuilderError::InvalidWriterConfig("missing config in builder".to_string())),
        };

        let err_fn = |err| eprintln!("[writer_stream] an error occurred on stream: {}", err);
        let writer = streams::WriterStream::new(spec, self.write_path.clone(), err_fn);
        //let writer_sender = writer.get_sender();
        Ok(ContextTail{
            writer_thread: writer,
            write_spec: spec,
        })
    }
}
