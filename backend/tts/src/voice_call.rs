/// Voice-call stub — placeholder for PSTN/VoIP integration.
///
/// Full implementation would integrate with Twilio, Vonage, or SIP.
use serde::{Deserialize, Serialize};

/// Status of a voice call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallStatus {
    Queued,
    Ringing,
    InProgress,
    Completed,
    Failed,
    Busy,
    NoAnswer,
}

/// An outbound voice call record.
#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceCall {
    pub call_id: String,
    pub to: String,
    pub from: String,
    pub status: CallStatus,
    pub duration_secs: Option<u32>,
}

/// Stub: initiate an outbound call.
/// In production, this would call the Twilio/Vonage API.
pub async fn initiate_call(to: &str, from: &str, _twiml_url: &str) -> VoiceCall {
    tracing::warn!(
        "[VoiceCall] Voice call to {} is a stub — integrate Twilio/Vonage for production",
        to
    );
    VoiceCall {
        call_id: format!("stub-{}", uuid::Uuid::new_v4()),
        to: to.to_string(),
        from: from.to_string(),
        status: CallStatus::Queued,
        duration_secs: None,
    }
}
