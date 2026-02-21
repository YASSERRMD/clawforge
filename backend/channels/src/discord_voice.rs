//! Discord Voice Adapter
//!
//! Exposes bindings required to join a Discord Server's voice channel
//! and pipe TTS buffers into the UDP streams.

use anyhow::Result;
use tracing::info;

pub struct DiscordVoice;

impl DiscordVoice {
    /// Invokes the `songbird` crate or raw voice handlers to connect to a voice channel.
    pub async fn join_voice_channel(guild_id: u64, channel_id: u64) -> Result<()> {
        info!("Joining Discord voice channel {} in guild {}", channel_id, guild_id);
        // MOCK: Initiate Gateway Voice State Update
        Ok(())
    }

    /// Continuously pipes an incoming byte array representing PCM audio to the active voice stream.
    pub async fn play_tts_buffer(audio_pcm: &[u8]) -> Result<()> {
        info!("Playing {} bytes of audio to Discord VC", audio_pcm.len());
        Ok(())
    }

    /// Cleanly exits the active voice session.
    pub async fn leave_voice_channel(guild_id: u64) -> Result<()> {
        info!("Leaving voice channel in guild {}", guild_id);
        Ok(())
    }
}
