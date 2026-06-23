use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::thumbnail::decoder::{decode_resize_rgba, extract_thumbnail_bytes_from_gcode_file};
use crate::thumbnail::resolver::{
    MoonrakerFileSource, ThumbnailResolverConfig, resolve_moonraker_file,
};
use crate::thumbnail::tjc_encoder::{commands_from_chunks, encode_image_to_tjc_chunks};
use crate::thumbnail::{ThumbnailKey, ThumbnailRequest, ThumbnailResult, ThumbnailSource};

pub struct ThumbnailWorker {
    request_rx: mpsc::Receiver<ThumbnailRequest>,
    result_tx: mpsc::Sender<ThumbnailResult>,
    resolver_config: Option<ThumbnailResolverConfig>,
}

impl ThumbnailWorker {
    pub fn new(
        request_rx: mpsc::Receiver<ThumbnailRequest>,
        result_tx: mpsc::Sender<ThumbnailResult>,
    ) -> Self {
        Self {
            request_rx,
            result_tx,
            resolver_config: None,
        }
    }

    pub fn with_resolver_config(mut self, resolver_config: ThumbnailResolverConfig) -> Self {
        self.resolver_config = Some(resolver_config);
        self
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(request) = self.request_rx.recv().await {
            let resolver_config = self.resolver_config.clone();
            let result =
                tokio::task::spawn_blocking(move || process_request(request, resolver_config))
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

fn process_request(
    request: ThumbnailRequest,
    resolver_config: Option<ThumbnailResolverConfig>,
) -> ThumbnailResult {
    match request {
        ThumbnailRequest::Prepare { key, source } => {
            let result = match source {
                ThumbnailSource::PreparedChunks(chunks) => {
                    Ok(commands_from_chunks(&key.target, &chunks))
                }
                ThumbnailSource::GcodeFile(path) => process_gcode_path(&key, &path),
                ThumbnailSource::MoonrakerFile {
                    path,
                    modified,
                    size,
                } => resolver_config
                    .as_ref()
                    .context("thumbnail Moonraker resolver is not configured")
                    .and_then(|config| {
                        resolve_moonraker_file(
                            config,
                            &MoonrakerFileSource {
                                path,
                                modified,
                                size,
                            },
                        )
                    })
                    .and_then(|path| process_gcode_path(&key, path.to_string_lossy().as_ref())),
            };

            ThumbnailResult {
                key,
                result: result.map_err(|error| format!("{error:#}")),
            }
        }
    }
}

fn process_gcode_path(
    key: &ThumbnailKey,
    path: &str,
) -> Result<Vec<crate::hmi::command::HmiCommand>> {
    extract_thumbnail_bytes_from_gcode_file(path)
        .and_then(|bytes| decode_resize_rgba(&bytes, key.width, key.height))
        .map(|rgba| encode_image_to_tjc_chunks(&rgba))
        .map(|chunks| commands_from_chunks(&key.target, &chunks))
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
                    file_modified: None,
                    file_size: None,
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
                    file_modified: None,
                    file_size: None,
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

    #[tokio::test]
    async fn moonraker_file_without_resolver_returns_error() {
        let (request_tx, request_rx) = mpsc::channel(1);
        let (result_tx, mut result_rx) = mpsc::channel(1);

        let worker = ThumbnailWorker::new(request_rx, result_tx);
        let worker_handle = tokio::spawn(worker.run());

        request_tx
            .send(ThumbnailRequest::Prepare {
                key: ThumbnailKey::print("cube.gcode"),
                source: ThumbnailSource::MoonrakerFile {
                    path: "cube.gcode".to_string(),
                    modified: Some(1),
                    size: Some(2),
                },
            })
            .await
            .unwrap();
        drop(request_tx);

        let result = result_rx.recv().await.unwrap();

        assert!(
            result
                .result
                .unwrap_err()
                .contains("Moonraker resolver is not configured")
        );

        worker_handle.await.unwrap().unwrap();
    }
}
