use alvr_common::*;
use alvr_session::AudioConfig;
use alvr_sockets::{StreamReceiver, StreamSender, AUDIO};
use std::future;

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
    sample_rate: u32
) -> StrResult {
    #[cfg(target_os = "android")]
        {
            play_audio_loop_nop(game_audio_receiver).await // TODO implement
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
    sample_rate: u32
) -> StrResult {
    #[cfg(target_os = "android")]
        {
            record_audio_loop_nop(microphone_sender).await // TODO implement
        }

    #[cfg(not(target_os = "android"))]
        {
            record_audio_loop_nop(microphone_sender).await
        }
}