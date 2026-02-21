//! Optical Character Recognition (OCR)
//!
//! Bridges Tesseract or LLM Vision endpoints to extract dense text from images
//! sent to the agent by users or web scrapers.

use anyhow::Result;
use tracing::info;

pub struct OcrService;

impl OcrService {
    /// Dispatches an image to a Vision model to read and layout all discernible text.
    pub async fn extract_text(image_path: &str) -> Result<String> {
        info!("Running OCR detection on image file: {}", image_path);
        
        // MOCK: read image bytes, POST to GPT-4V or run local Tesseract
        Ok("extracted image text from document mock".into())
    }
}
