//! Text chunking utilities for simulating streaming output
//!
//! Since Claude CLI returns complete messages, we need to chunk them
//! to provide a better streaming experience.

#![allow(dead_code)] // Public API - may not be used internally

use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{Interval, interval};

/// Configuration for text chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Size of each chunk in characters
    pub chunk_size: usize,
    /// Delay between chunks in milliseconds
    pub chunk_delay_ms: u64,
    /// Whether to split at word boundaries
    pub word_boundary: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 20,      // ~3-5 words per chunk
            chunk_delay_ms: 50,  // 50ms between chunks for smooth streaming
            word_boundary: true, // Split at word boundaries for natural flow
        }
    }
}

/// A stream that chunks text into smaller pieces with delays
pub struct TextChunker {
    text: String,
    position: usize,
    config: ChunkConfig,
    interval: Interval,
}

impl TextChunker {
    /// Create a new text chunker
    pub fn new(text: String, config: ChunkConfig) -> Self {
        let interval = interval(Duration::from_millis(config.chunk_delay_ms));
        Self {
            text,
            position: 0,
            config,
            interval,
        }
    }

    /// Get the next chunk of text
    fn next_chunk(&mut self) -> Option<String> {
        if self.position >= self.text.len() {
            return None;
        }

        let remaining = &self.text[self.position..];
        let mut chunk_end = self.config.chunk_size.min(remaining.len());

        // If word_boundary is enabled, try to break at word boundaries
        if self.config.word_boundary && chunk_end < remaining.len() {
            // Look for the last space within the chunk
            if let Some(last_space) = remaining[..chunk_end].rfind(' ') {
                if last_space > 0 {
                    chunk_end = last_space + 1; // Include the space
                }
            } else {
                // No space found in chunk, look forward for the next space
                if let Some(next_space) = remaining[chunk_end..].find(' ') {
                    chunk_end = chunk_end + next_space + 1;
                }
            }
        }

        let chunk = remaining[..chunk_end].to_string();
        self.position += chunk_end;
        Some(chunk)
    }
}

impl Stream for TextChunker {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Wait for the interval
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => {
                // Get next chunk
                Poll::Ready(self.next_chunk())
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create a chunked stream from text
pub fn chunk_text(text: String, config: Option<ChunkConfig>) -> impl Stream<Item = String> {
    TextChunker::new(text, config.unwrap_or_default())
}

/// Split text into chunks for array processing
pub fn split_text_into_chunks(text: &str, config: &ChunkConfig) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut position = 0;

    while position < text.len() {
        let remaining = &text[position..];
        let mut chunk_end = config.chunk_size.min(remaining.len());

        if config.word_boundary && chunk_end < remaining.len() {
            if let Some(last_space) = remaining[..chunk_end].rfind(' ') {
                if last_space > 0 {
                    chunk_end = last_space + 1;
                }
            } else if let Some(next_space) = remaining[chunk_end..].find(' ') {
                chunk_end = chunk_end + next_space + 1;
            }
        }

        chunks.push(remaining[..chunk_end].to_string());
        position += chunk_end;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_text_basic() {
        let text = "Hello world, this is a test message.";
        let config = ChunkConfig {
            chunk_size: 10,
            chunk_delay_ms: 0,
            word_boundary: false,
        };

        let chunks = split_text_into_chunks(text, &config);
        assert_eq!(chunks[0], "Hello worl");
        assert_eq!(chunks[1], "d, this is");
    }

    #[test]
    fn test_split_text_word_boundary() {
        let text = "Hello world, this is a test message.";
        let config = ChunkConfig {
            chunk_size: 10,
            chunk_delay_ms: 0,
            word_boundary: true,
        };

        let chunks = split_text_into_chunks(text, &config);
        assert_eq!(chunks[0], "Hello ");
        assert_eq!(chunks[1], "world, ");
    }
}
