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
    //buffer: Vec<Vec<f32>>,
    buffer_interleaved: Vec<f32>,
    buffer_mp3: Vec<u8>,
    finalized: bool,

    samples_buffered: usize,
    samples_to_buffer: usize,
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
            spec: spec,
            writer: writer,
            //sampler_writer_buffer: Vec::new(),
            //buffer: vec![Vec::with_capacity(samples_per_15min); chans],
            //buffer_mp3: mp3_out_buffer,
            // input buffer doesnt have to be exact, this is just an arbitrary number of sampels
            buffer_interleaved: Vec::with_capacity(samples_per_15min_interleaved),
            buffer_mp3: Vec::new(),
            
            finalized: false,

            samples_buffered: 0,
            samples_to_buffer: samples_per_15min_interleaved,
            encoder,
        })
    }

    pub fn create<P: AsRef<path::Path>>(filename: P, spec: Mp3Spec) -> Result<Mp3Writer<io::BufWriter<fs::File>>, Mp3WriterError> {
        let file = fs::File::create(filename).map_err(|error| Mp3WriterError::CreateError(error.to_string()))?;
        let buf_writer = io::BufWriter::new(file);
        Mp3Writer::new(buf_writer, spec)
    }

    //fn write_sample<S: cpal::Sample>(&mut self, sample: S) -> Result<(), Mp3WriterError> {
    pub fn write_sample(&mut self, sample: f32) -> Result<(), Mp3WriterError> {
        self.write_sample_to_buffer(sample)
        // also check if to create checkpoint
    }
    // use mp3lame_encoder::PCMInterleaved
    fn write_sample_to_buffer(&mut self, sample: f32) -> Result<(), Mp3WriterError> {
        self.buffer_interleaved.push(sample);
        self.samples_buffered += 1;
        Ok(())
    }
    //fn write_sample_to_buffer(&mut self, sample: f32) -> Result<(), Mp3WriterError> {
    //    let chan: u64 = self.samples_buffered % self.spec.channels as u64;
    //    self.buffer[chan as usize].push(sample);
    //    self.samples_buffered += 1;
    //    //self.data_bytes_written + = self.bytes_per_sample as u32;
    //    Ok(())
    //}
    //fn write_checkpoint(&mut self) -> Result<(), Mp3WriterError> {
    //    if self.samples_buffered < self.samples_to_buffer * self.spec.channels as usize {
    //        return Ok(());
    //    }
    //    // TODO: write to actual checkpoint
    //    Ok(())
    //}
    pub fn finalize(&mut self) -> Result<(), Mp3WriterError> {
        println!("FINALIZE CALLED");
        self.write_to_path()
    }

    // TODO: write an actual checkpoint
    pub fn write_to_path(&mut self) -> Result<(), Mp3WriterError> {
        self.buffer_mp3 = Mp3Writer::<W>::initialize_buffer(&self.spec, self.samples_buffered);

        let input = mp3lame_encoder::InterleavedPcm(&self.buffer_interleaved);

        //let input = mp3lame_encoder::DualPcm{
        //    left: self.buffer[0].as_slice(),
        //    right: self.buffer[1].as_slice(),
        //};
        println!("Encoding buffer...");
        let encoded_size = self.encoder.encode(
            input,
            self.buffer_mp3.spare_capacity_mut()
        ).map_err(|error| Mp3WriterError::EncoderEncodeError(error.to_string()))?;

        // reserve can reserve more than the value you give it
        unsafe {
            self.buffer_mp3.set_len(self.buffer_mp3.len().wrapping_add(encoded_size));
        }

        let encoded_size = self.encoder.flush::<mp3lame_encoder::FlushNoGap>(self.buffer_mp3.spare_capacity_mut())
            .map_err(|error| Mp3WriterError::EncoderEncodeError(error.to_string()))?;
        unsafe {
            self.buffer_mp3.set_len(self.buffer_mp3.len().wrapping_add(encoded_size));
        }

        println!("Writing to buffer...");
        self.writer.write(&self.buffer_mp3)
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
        mp3_encoder.set_sample_rate(spec.sample_rate)
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

    fn initialize_buffer(spec: &Mp3Spec, num_samples: usize) -> Vec<u8> {
        let chans: usize = spec.channels.into();
        //let samples_per_15min: usize = (spec.sample_rate * 60 * 15) as usize;
        //let samples_per_15min_interleaved: usize = samples_per_15min * chans;
        let sample_count_interleaved = num_samples * chans;
        //mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(samples_per_15min));
        let mut mp3_out_buffer = Vec::new();
        mp3_out_buffer.reserve(mp3lame_encoder::max_required_buffer_size(sample_count_interleaved));
        mp3_out_buffer
    }
}
