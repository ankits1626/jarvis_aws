// IntelQueue - Request serialization and response routing for IntelProvider

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use super::provider::{CoPilotCycleResult, IntelProvider, TranscriptResult};

/// Request sent to the IntelQueue worker
pub struct IntelRequest {
    pub command: IntelCommand,
    pub reply_tx: oneshot::Sender<Result<IntelResponse, String>>,
}

/// Commands that can be submitted to the IntelQueue
#[derive(Debug)]
pub enum IntelCommand {
    Chat {
        messages: Vec<(String, String)>,
    },
    GenerateTranscript {
        audio_path: PathBuf,
    },
    CopilotAnalyze {
        audio_path: PathBuf,
        context: String,
    },
    GenerateTags {
        content: String,
    },
    Summarize {
        content: String,
    },
}

/// Responses returned from the IntelQueue worker
#[derive(Debug)]
pub enum IntelResponse {
    Chat(String),
    Transcript(TranscriptResult),
    CopilotAnalysis(CoPilotCycleResult),
    Tags(Vec<String>),
    Summary(String),
}

/// IntelQueue serializes all IntelProvider requests through a single mpsc channel.
/// Each caller gets its response back via a dedicated oneshot channel.
#[derive(Clone)]
pub struct IntelQueue {
    tx: mpsc::Sender<IntelRequest>,
}

impl IntelQueue {
    /// Create a new IntelQueue and spawn the worker task.
    ///
    /// The worker processes requests sequentially, calling the appropriate
    /// IntelProvider method for each command and routing the response back
    /// to the caller via the oneshot channel.
    ///
    /// # Arguments
    ///
    /// * `provider` - The IntelProvider to use for processing requests
    ///
    /// # Returns
    ///
    /// A new IntelQueue instance
    pub fn new(provider: Arc<dyn IntelProvider>) -> Self {
        let (tx, mut rx) = mpsc::channel::<IntelRequest>(32);

        // Spawn worker task
        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                let result = match req.command {
                    IntelCommand::Chat { messages } => {
                        provider.chat(&messages).await.map(IntelResponse::Chat)
                    }
                    IntelCommand::GenerateTranscript { audio_path } => {
                        provider
                            .generate_transcript(&audio_path)
                            .await
                            .map(IntelResponse::Transcript)
                    }
                    IntelCommand::CopilotAnalyze { audio_path, context } => {
                        provider
                            .copilot_analyze(&audio_path, &context)
                            .await
                            .map(IntelResponse::CopilotAnalysis)
                    }
                    IntelCommand::GenerateTags { content } => {
                        provider
                            .generate_tags(&content)
                            .await
                            .map(IntelResponse::Tags)
                    }
                    IntelCommand::Summarize { content } => {
                        provider.summarize(&content).await.map(IntelResponse::Summary)
                    }
                };

                // Send result back to caller (ignore send errors - caller may have dropped)
                let _ = req.reply_tx.send(result);
            }
        });

        IntelQueue { tx }
    }

    /// Submit a command to the queue and await the response.
    ///
    /// Creates a oneshot channel for the response, sends the request to the
    /// worker, and awaits the response.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Returns
    ///
    /// * `Ok(IntelResponse)` - The response from the provider
    /// * `Err(String)` - Error if the queue is closed or the worker dropped
    pub async fn submit(&self, command: IntelCommand) -> Result<IntelResponse, String> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(IntelRequest { command, reply_tx })
            .await
            .map_err(|_| "Queue closed".to_string())?;

        reply_rx
            .await
            .map_err(|_| "Worker dropped".to_string())?
    }
}
