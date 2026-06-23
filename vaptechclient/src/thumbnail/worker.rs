use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::thumbnail::decoder::{decode_resize_rgba, extract_thumbnail_bytes_from_gcode_file};
use crate::thumbnail::tjc_encoder::{commands_from_chunks, encode_image_to_tjc_chunks};
use crate::thumbnail::{ThumbnailRequest, ThumbnailResult, ThumbnailSource};

pub struct ThumbnailWorker {
    request_rx: mpsc::Receiver<ThumbnailRequest>,
    result_tx: mpsc::Sender<ThumbnailResult>,
}

impl ThumbnailWorker {
    pub fn new(
        request_rx: mpsc::Receiver<ThumbnailRequest>,
        result_tx: mpsc::Sender<ThumbnailResult>,
    ) -> Self {
        Self {
            request_rx,
            result_tx,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(request) = self.request_rx.recv().await {
            let result = tokio::task::spawn_blocking(move || process_request(request))
                .await
                .context("thumbnail worker task panicked")?;

            self.result_tx
                .send(result)
                .await
                .context("failed to publish thumbnail result")?;
        }

        Ok(())
    }
}

fn process_request(request: ThumbnailRequest) -> ThumbnailResult {
    match request {
        ThumbnailRequest::Prepare { key, source } => {
            let result = match source {
                ThumbnailSource::PreparedChunks(chunks) => {
                    Ok(commands_from_chunks(&key.target, &chunks))
                }
                ThumbnailSource::GcodeFile(path) => {
                    let result = extract_thumbnail_bytes_from_gcode_file(&path)
                        .and_then(|bytes| decode_resize_rgba(&bytes, key.width, key.height))
                        .map(|rgba| encode_image_to_tjc_chunks(&rgba))
                        .map(|chunks| commands_from_chunks(&key.target, &chunks));

                    result.map_err(|error| format!("{error:#}"))
                }
            };

            ThumbnailResult { key, result }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hmi::command::HmiCommand;
    use crate::thumbnail::{ThumbnailKey, ThumbnailTarget};

    #[tokio::test]
    async fn prepared_chunks_are_encoded_in_background() {
        let (request_tx, request_rx) = mpsc::channel(1);
        let (result_tx, mut result_rx) = mpsc::channel(1);

        let worker = ThumbnailWorker::new(request_rx, result_tx);
        let worker_handle = tokio::spawn(worker.run());

        request_tx
            .send(ThumbnailRequest::Prepare {
                key: ThumbnailKey {
                    file_path: "cube.gcode".to_string(),
                    target: ThumbnailTarget::PrintPage,
                    width: 155,
                    height: 155,
                    encoder_version: 1,
                },
                source: ThumbnailSource::PreparedChunks(vec!["abc".to_string()]),
            })
            .await
            .unwrap();
        drop(request_tx);

        let result = result_rx.recv().await.unwrap();
        let commands = result.result.unwrap();

        assert!(commands.contains(&HmiCommand::raw("cp0.write(\"abc\")")));

        worker_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn gcode_file_is_decoded_in_background() {
        let (request_tx, request_rx) = mpsc::channel(1);
        let (result_tx, mut result_rx) = mpsc::channel(1);

        let worker = ThumbnailWorker::new(request_rx, result_tx);
        let worker_handle = tokio::spawn(worker.run());
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/thumbnails/orca_thumbnail.gcode"
        );

        request_tx
            .send(ThumbnailRequest::Prepare {
                key: ThumbnailKey {
                    file_path: path.to_string(),
                    target: ThumbnailTarget::PrintPage,
                    width: 155,
                    height: 155,
                    encoder_version: 1,
                },
                source: ThumbnailSource::GcodeFile(path.to_string()),
            })
            .await
            .unwrap();
        drop(request_tx);

        let result = result_rx.recv().await.unwrap();
        let commands = result.result.unwrap();

        assert!(
            commands
                .iter()
                .any(|command| command == &HmiCommand::raw("Print_Trun_1.cp0.close()"))
        );
        assert!(
            commands
                .iter()
                .any(|command| command.to_string().starts_with("cp0.write(\"0`0009"))
        );

        worker_handle.await.unwrap().unwrap();
    }
}
