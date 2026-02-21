//! MIME type detection for media files.
//!
//! Used by the media pipeline to correctly label stored files.

use std::path::Path;

/// Detect MIME type by file extension.
pub fn detect_mime_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // Images
        "jpg" | "jpeg" => "image/jpeg",
        "png"          => "image/png",
        "gif"          => "image/gif",
        "webp"         => "image/webp",
        "svg"          => "image/svg+xml",
        "avif"         => "image/avif",
        "bmp"          => "image/bmp",
        "ico"          => "image/x-icon",
        "tiff" | "tif" => "image/tiff",

        // Audio
        "mp3"          => "audio/mpeg",
        "ogg"          => "audio/ogg",
        "wav"          => "audio/wav",
        "flac"         => "audio/flac",
        "m4a"          => "audio/mp4",
        "opus"         => "audio/opus",
        "aac"          => "audio/aac",

        // Video
        "mp4"          => "video/mp4",
        "webm"         => "video/webm",
        "mkv"          => "video/x-matroska",
        "mov"          => "video/quicktime",
        "avi"          => "video/x-msvideo",
        "ogv"          => "video/ogg",

        // Documents
        "pdf"          => "application/pdf",
        "txt"          => "text/plain",
        "md"           => "text/markdown",
        "html" | "htm" => "text/html",
        "json"         => "application/json",
        "xml"          => "application/xml",
        "csv"          => "text/csv",

        _              => "application/octet-stream",
    }
}

/// Whether a MIME type is for an image.
pub fn is_image(mime: &str) -> bool {
    mime.starts_with("image/")
}

/// Whether a MIME type is for audio.
pub fn is_audio(mime: &str) -> bool {
    mime.starts_with("audio/")
}

/// Whether a MIME type is for video.
pub fn is_video(mime: &str) -> bool {
    mime.starts_with("video/")
}

/// Whether a file is safe to serve inline (not just download).
pub fn is_inline_safe(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp"
        | "audio/mpeg" | "audio/ogg" | "audio/wav"
        | "video/mp4" | "video/webm"
        | "text/plain" | "application/pdf"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_jpeg() {
        assert_eq!(detect_mime_type(&PathBuf::from("photo.jpg")), "image/jpeg");
    }

    #[test]
    fn detects_mp3() {
        assert_eq!(detect_mime_type(&PathBuf::from("speech.mp3")), "audio/mpeg");
    }

    #[test]
    fn unknown_extension_fallback() {
        assert_eq!(detect_mime_type(&PathBuf::from("file.xyz")), "application/octet-stream");
    }
}
