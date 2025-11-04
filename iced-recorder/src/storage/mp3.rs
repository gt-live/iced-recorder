use std::io::{self, BufWriter};
use std::path;
use std::fs;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Mp3WriterError {
    #[error("unable to write to file: {0}")]
    WriteError(String),
    #[error("unable to create write to file: {0}")]
    CreateError(String),
    #[error("unable to build lame encoder: {0}")]
    EncoderBuildError(String),
    #[error("encoder unable to build encode input: {0}")]
    EncoderEncodeError(String),
    #[error("unable to write mp3 to file: {0}")]
    WriteToFileError(String),
}

#[derive(Debug, Clone, Copy)]
pub struct Mp3Spec {
    pub channels: u8,
    pub sample_rate: usize,
    pub bits_per_sample: u16,
}

pub struct Mp3Writer<W> 
    where W: io::Write + io::Seek
{
    spec: Mp3Spec,
    writer: W,
    encoder: mp3lame_encoder::Encoder,
}

impl<W> Mp3Writer<W> 
    where W: io::Write + io::Seek
{

    fn new(writer: W, spec: Mp3Spec) -> Result<Mp3Writer<W>, Mp3WriterError> 
    where
        W: io::Write + io::Seek,
    {
        let chans: usize = spec.channels.into();
        let samples_per_15min: usize = (spec.sample_rate * 60 * 15) as usize;
        let samples_per_15min_interleaved: usize = samples_per_15min * chans;
        let encoder = Self::initialize_encoder(&spec)?;
        Ok(Mp3Writer {
            spec,
            writer,
            encoder,
        })
    }

    pub fn create<P: AsRef<path::Path>>(filename: P, spec: Mp3Spec) -> Result<Mp3Writer<io::BufWriter<fs::File>>, Mp3WriterError> {
        let file = fs::File::create(filename).map_err(|error| Mp3WriterError::CreateError(error.to_string()))?;
        let buf_writer = io::BufWriter::new(file);
        Mp3Writer::new(buf_writer, spec)
    }

    pub fn create_and_write<P: AsRef<path::Path>>(filename: P, spec: Mp3Spec, buffer_in: Vec<f32>) -> Result<(), Mp3WriterError> {
        let mut writer = Mp3Writer::<W>::create(filename, spec)?;
        writer.write_to_path(buffer_in)
    }

    // TODO: write an actual checkpoint
    //fn write_to_path(&mut self, buffer_in: &[f32]) -> Result<(), Mp3WriterError> {
    pub fn write_to_path(&mut self, buffer_in: Vec<f32>) -> Result<(), Mp3WriterError> {
        let mut buffer_out = Mp3Writer::<W>::initialize_buffer(buffer_in.len());
        //let buffer_out = Mp3Writer::<W>::initialize_buffer(&self.spec, self.samples_buffered);

        let input = mp3lame_encoder::InterleavedPcm(&buffer_in);
        println!("Encoding buffer...");
        let encoded_size = self.encoder.encode(
            input,
            buffer_out.spare_capacity_mut()
        ).map_err(|error| Mp3WriterError::EncoderEncodeError(error.to_string()))?;

        // reserve can reserve more than the value you give it
        unsafe {
            buffer_out.set_len(buffer_out.len().wrapping_add(encoded_size));
        }

        let encoded_size = self.encoder.flush::<mp3lame_encoder::FlushNoGap>(buffer_out.spare_capacity_mut())
            .map_err(|error| Mp3WriterError::EncoderEncodeError(error.to_string()))?;
        unsafe {
            buffer_out.set_len(buffer_out.len().wrapping_add(encoded_size));
        }

        println!("Writing to buffer...");
        self.writer.write(&buffer_out)
            .map_err(|error| Mp3WriterError::WriteToFileError(error.to_string()))?;
        Ok(())
    }

    fn initialize_encoder(spec: &Mp3Spec) -> Result<mp3lame_encoder::Encoder, Mp3WriterError> {
        let mut mp3_encoder = match mp3lame_encoder::Builder::new() {
            None => return Err(Mp3WriterError::EncoderBuildError("failed to get new builder".to_string())),
            Some(builder) => builder,
        };
        mp3_encoder.set_num_channels(spec.channels as u8)
            .map_err(|error| Mp3WriterError::EncoderBuildError(error.to_string()))?;
        mp3_encoder.set_sample_rate(spec.sample_rate as u32)
            .map_err(|error| Mp3WriterError::EncoderBuildError(error.to_string()))?;
        // TODO: find out bitrate
        mp3_encoder.set_brate(mp3lame_encoder::Bitrate::Kbps320)
            .map_err(|error| Mp3WriterError::EncoderBuildError(error.to_string()))?;
        mp3_encoder.set_quality(mp3lame_encoder::Quality::Best)
            .map_err(|error| Mp3WriterError::EncoderBuildError(error.to_string()))?;

        // let it blow up first so i can see if it works
        let mp3_encoder = mp3_encoder
            .build()
            .map_err(|error| Mp3WriterError::EncoderBuildError(error.to_string()))?;
            //.expect("Unable to build");
        Ok(mp3_encoder)
    }

    fn initialize_buffer(len_buffer: usize) -> Vec<u8> {
        let mut mp3_out_buffer = Vec::new();
        mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(len_buffer));
        mp3_out_buffer
    }
    //fn initialize_buffer(spec: &Mp3Spec, num_samples: usize) -> Vec<u8> {
    //    let chans: usize = spec.channels.into();
    //    //let samples_per_15min: usize = (spec.sample_rate * 60 * 15) as usize;
    //    //let samples_per_15min_interleaved: usize = samples_per_15min * chans;
    //    let sample_count_interleaved = num_samples * chans;
    //    //mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_per_15min));
    //    let mut mp3_out_buffer = Vec::new();
    //    mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(sample_count_interleaved));
    //    mp3_out_buffer
    //}
}
