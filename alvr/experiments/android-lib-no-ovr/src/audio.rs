use alvr_audio;
use alvr_common::*;
use alvr_session::AudioConfig;
use alvr_sockets::{StreamReceiver, StreamSender, AUDIO};
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    future,
    mem,
    sync::Arc,
    sync::mpsc as smpsc,
    thread,
};
use tokio::sync::mpsc as tmpsc;

#[cfg(target_os = "android")]
use oboe::*;

pub async fn play_audio_loop_nop(
    mut game_audio_receiver: StreamReceiver<(), AUDIO>,
) -> StrResult {
    loop {
        game_audio_receiver.recv().await?;
    }
}

pub async fn play_audio_loop(
    game_audio_receiver: StreamReceiver<(), AUDIO>,
    audio_config: AudioConfig,
    sample_rate: u32,
) -> StrResult {
    #[cfg(target_os = "android")]
        {
            play_audio_loop_android(
                game_audio_receiver,
                audio_config,
                sample_rate
            ).await
        }

    #[cfg(not(target_os = "android"))]
        {
            play_audio_loop_nop(game_audio_receiver).await
        }
}

pub async fn record_audio_loop_nop(
    microphone_sender: StreamSender<(), AUDIO>,
) -> StrResult {
    future::pending().await
}

pub async fn record_audio_loop(
    microphone_sender: StreamSender<(), AUDIO>,
    sample_rate: u32,
) -> StrResult {
    #[cfg(target_os = "android")]
        {
            record_audio_loop_android(
                microphone_sender,
                sample_rate
            ).await
        }

    #[cfg(not(target_os = "android"))]
        {
            record_audio_loop_nop(microphone_sender).await
        }
}

#[cfg(target_os = "android")]
struct PlayerCallback {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    batch_frames_count: usize,
}

#[cfg(target_os = "android")]
impl AudioOutputCallback for PlayerCallback {
    type FrameType = (f32, Stereo);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioOutputStreamSafe,
        out_frames: &mut [(f32, f32)],
    ) -> DataCallbackResult {
        let samples = alvr_audio::get_next_frame_batch(
            &mut *self.sample_buffer.lock(),
            2,
            self.batch_frames_count,
        );

        for f in 0..out_frames.len() {
            out_frames[f] = (samples[f * 2], samples[f * 2 + 1]);
        }

        DataCallbackResult::Continue
    }
}

#[cfg(target_os = "android")]
async fn play_audio_loop_android(
    game_audio_receiver: StreamReceiver<(), AUDIO>,
    audio_config: AudioConfig,
    sample_rate: u32,
) -> StrResult {
    let batch_frames_count = sample_rate as usize * audio_config.batch_ms as usize / 1000;
    let average_buffer_frames_count =
        sample_rate as usize * audio_config.average_buffering_ms as usize / 1000;

    let sample_buffer = Arc::new(Mutex::new(VecDeque::new()));

    // store the stream in a thread (because !Send) and extract the playback handle
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    thread::spawn({
        let sample_buffer = Arc::clone(&sample_buffer);
        move || -> StrResult {
            let mut stream = trace_err!(AudioStreamBuilder::default()
                .set_shared()
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sample_rate(sample_rate as _)
                .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
                .set_stereo()
                .set_f32()
                .set_frames_per_callback(batch_frames_count as _)
                .set_output()
                .set_usage(Usage::Game)
                .set_callback(PlayerCallback {
                    sample_buffer,
                    batch_frames_count,
                })
                .open_stream())?;

            trace_err!(stream.start())?;

            shutdown_receiver.recv().ok();

            // Note: Oboe crahes if stream.stop() is NOT called on AudioPlayer
            stream.stop_with_timeout(0).ok();

            Ok(())
        }
    });

    alvr_audio::receive_samples_loop(
        game_audio_receiver,
        sample_buffer,
        2,
        batch_frames_count,
        average_buffer_frames_count,
    ).await
}

#[cfg(target_os = "android")]
struct RecorderCallback {
    sender: tmpsc::UnboundedSender<Vec<u8>>,
}

#[cfg(target_os = "android")]
impl AudioInputCallback for RecorderCallback {
    type FrameType = (i16, Mono);

    fn on_audio_ready(
        &mut self,
        _: &mut dyn AudioInputStreamSafe,
        frames: &[i16],
    ) -> DataCallbackResult {
        let mut sample_buffer = Vec::with_capacity(frames.len() * mem::size_of::<i16>());

        for frame in frames {
            sample_buffer.extend(&frame.to_ne_bytes());
        }

        self.sender.send(sample_buffer).ok();

        DataCallbackResult::Continue
    }
}

#[cfg(target_os = "android")]
async fn record_audio_loop_android(
    mut microphone_sender: StreamSender<(), AUDIO>,
    sample_rate: u32,
) -> StrResult {
    let (_shutdown_notifier, shutdown_receiver) = smpsc::channel::<()>();
    let (data_sender, mut data_receiver) = tmpsc::unbounded_channel();

    thread::spawn(move || -> StrResult {
        let mut stream = trace_err!(AudioStreamBuilder::default()
            .set_shared()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sample_rate(sample_rate as _)
            .set_sample_rate_conversion_quality(SampleRateConversionQuality::Fastest)
            .set_mono()
            .set_i16()
            .set_input()
            .set_usage(Usage::VoiceCommunication)
            .set_input_preset(InputPreset::VoiceCommunication)
            .set_callback(RecorderCallback {
                sender: data_sender
            })
            .open_stream())?;

        trace_err!(stream.start())?;

        shutdown_receiver.recv().ok();

        // This call gets stuck if the headset goes to sleep, but finishes when the headset wakes up
        stream.stop_with_timeout(0).ok();

        Ok(())
    });

    while let Some(data) = data_receiver.recv().await {
        let mut buffer = microphone_sender.new_buffer(&(), data.len())?;
        buffer.get_mut().extend(data);
        microphone_sender.send_buffer(buffer).await.ok();
    }

    Ok(())
}