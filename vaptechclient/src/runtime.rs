use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::app::event::AppEvent;
use crate::app::runner::AppRunner;
use crate::app::state::Page;
use crate::hmi::command::HmiCommand;
use crate::thumbnail::cache::{ThumbnailCache, ThumbnailState};
use crate::thumbnail::{ThumbnailKey, ThumbnailRequest, ThumbnailResult, ThumbnailTarget};
use crate::ui::render_target::resolve_render_target;

/// Центральный async loop приложения.
///
/// Runtime не принимает решений сам: он доставляет AppEvent в AppRunner,
/// забирает сгенерированные эффекты и отправляет HMI-команды в UART service.
pub struct Runtime {
    runner: AppRunner,
    app_event_rx: mpsc::Receiver<AppEvent>,
    hmi_command_tx: mpsc::Sender<HmiCommand>,
    thumbnail_cache: ThumbnailCache,
    thumbnail_request_tx: Option<mpsc::Sender<ThumbnailRequest>>,
    thumbnail_result_rx: Option<mpsc::Receiver<ThumbnailResult>>,
}

impl Runtime {
    pub fn new(
        runner: AppRunner,
        app_event_rx: mpsc::Receiver<AppEvent>,
        hmi_command_tx: mpsc::Sender<HmiCommand>,
    ) -> Self {
        Self {
            runner,
            app_event_rx,
            hmi_command_tx,
            thumbnail_cache: ThumbnailCache::new(),
            thumbnail_request_tx: None,
            thumbnail_result_rx: None,
        }
    }

    pub fn with_thumbnails(
        runner: AppRunner,
        app_event_rx: mpsc::Receiver<AppEvent>,
        hmi_command_tx: mpsc::Sender<HmiCommand>,
        thumbnail_request_tx: mpsc::Sender<ThumbnailRequest>,
        thumbnail_result_rx: mpsc::Receiver<ThumbnailResult>,
    ) -> Self {
        Self {
            runner,
            app_event_rx,
            hmi_command_tx,
            thumbnail_cache: ThumbnailCache::new(),
            thumbnail_request_tx: Some(thumbnail_request_tx),
            thumbnail_result_rx: Some(thumbnail_result_rx),
        }
    }

    pub async fn run(self) -> Result<()> {
        if self.thumbnail_result_rx.is_some() {
            self.run_with_thumbnails().await
        } else {
            self.run_without_thumbnails().await
        }
    }

    async fn run_without_thumbnails(mut self) -> Result<()> {
        while let Some(event) = self.app_event_rx.recv().await {
            self.handle_app_event(event).await?;
        }

        Ok(())
    }

    async fn run_with_thumbnails(mut self) -> Result<()> {
        let mut thumbnail_result_rx = self
            .thumbnail_result_rx
            .take()
            .expect("thumbnail result rx exists");

        loop {
            tokio::select! {
                event = self.app_event_rx.recv() => {
                    let Some(event) = event else {
                        break;
                    };

                    self.handle_app_event(event).await?;
                }

                result = thumbnail_result_rx.recv() => {
                    let Some(result) = result else {
                        break;
                    };

                    self.handle_thumbnail_result(result).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_app_event(&mut self, event: AppEvent) -> Result<()> {
        self.runner.handle_event(event)?;
        self.drain_runner_outputs().await
    }

    async fn drain_runner_outputs(&mut self) -> Result<()> {
        // HMI-команды отправляются строго через очередь. Это сохраняет
        // последовательность команд и не дает renderer'у писать в UART напрямую.
        for command in self.runner.drain_hmi_commands() {
            self.send_hmi_command(command).await?;
        }

        for request in self.runner.drain_thumbnail_requests() {
            self.handle_thumbnail_request(request).await?;
        }

        // Пока MoonrakerRequest намеренно не подключены к websocket-клиенту:
        // на живой длинной печати это гарантирует read-only режим.
        for request in self.runner.drain_moonraker_requests() {
            tracing::debug!(?request, "moonraker request queued");
        }

        Ok(())
    }

    async fn handle_thumbnail_request(&mut self, request: ThumbnailRequest) -> Result<()> {
        let key = request.key().clone();

        match self.thumbnail_cache.get(&key) {
            Some(ThumbnailState::Ready(commands)) => {
                if self.thumbnail_is_current(&key) {
                    for command in commands.clone() {
                        self.send_hmi_command(command).await?;
                    }
                }
            }
            Some(ThumbnailState::Preparing) => {}
            Some(ThumbnailState::Failed(error)) => {
                tracing::debug!(
                    ?key,
                    error,
                    "thumbnail request skipped after previous failure"
                );
            }
            None => {
                let Some(request_tx) = &self.thumbnail_request_tx else {
                    tracing::debug!(?key, "thumbnail request dropped: worker is not configured");
                    return Ok(());
                };

                self.thumbnail_cache.mark_preparing(key.clone());
                request_tx
                    .send(request)
                    .await
                    .context("failed to queue thumbnail request")?;
            }
        }

        Ok(())
    }

    async fn handle_thumbnail_result(&mut self, result: ThumbnailResult) -> Result<()> {
        let key = result.key.clone();

        self.thumbnail_cache.apply_result(result);

        if !self.thumbnail_is_current(&key) {
            tracing::debug!(?key, "thumbnail ready but no longer current");
            return Ok(());
        }

        if let Some(ThumbnailState::Ready(commands)) = self.thumbnail_cache.get(&key) {
            for command in commands.clone() {
                self.send_hmi_command(command).await?;
            }
        }

        Ok(())
    }

    async fn send_hmi_command(&self, command: HmiCommand) -> Result<()> {
        tracing::debug!(%command, "HMI command produced by render diff");

        self.hmi_command_tx
            .send(command)
            .await
            .context("failed to queue HMI command")
    }

    fn thumbnail_is_current(&self, key: &ThumbnailKey) -> bool {
        match key.target {
            ThumbnailTarget::PrintPage => {
                resolve_render_target(&self.runner.state).is_print_view()
                    && self.runner.state.print.filename.as_deref() == Some(key.file_path.as_str())
            }
            ThumbnailTarget::FileSlot { slot } => {
                self.runner.state.hmi.current_screen == Page::Files
                    && self
                        .runner
                        .state
                        .files
                        .visible_file_at(slot)
                        .map(|file| file.path.as_str())
                        == Some(key.file_path.as_str())
            }
            ThumbnailTarget::PreviewPage => false,
            ThumbnailTarget::ResultPage => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::state::{FileSlot, Page};
    use crate::hmi::event::HmiEvent;
    use crate::moonraker::event::{MoonrakerEvent, PrinterStatus};
    use crate::thumbnail::ThumbnailKey;

    #[tokio::test]
    async fn hmi_startup_event_queues_home_page_command() {
        let (app_event_tx, app_event_rx) = mpsc::channel(4);
        let (hmi_command_tx, mut hmi_command_rx) = mpsc::channel(4);

        let runtime = Runtime::new(AppRunner::new(), app_event_rx, hmi_command_tx);
        let runtime_handle = tokio::spawn(runtime.run());

        app_event_tx
            .send(AppEvent::hmi(HmiEvent::Startup))
            .await
            .unwrap();
        drop(app_event_tx);

        assert_eq!(hmi_command_rx.recv().await, Some(HmiCommand::page(0)));

        runtime_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn ready_thumbnail_is_delivered_when_print_page_is_current() {
        let (app_event_tx, app_event_rx) = mpsc::channel(8);
        let (hmi_command_tx, mut hmi_command_rx) = mpsc::channel(32);
        let (thumbnail_request_tx, mut thumbnail_request_rx) = mpsc::channel(8);
        let (thumbnail_result_tx, thumbnail_result_rx) = mpsc::channel(8);

        let runtime = Runtime::with_thumbnails(
            AppRunner::new(),
            app_event_rx,
            hmi_command_tx,
            thumbnail_request_tx,
            thumbnail_result_rx,
        );
        let runtime_handle = tokio::spawn(runtime.run());

        app_event_tx
            .send(AppEvent::moonraker(MoonrakerEvent::printer_status(
                PrinterStatus::Printing,
            )))
            .await
            .unwrap();
        app_event_tx
            .send(AppEvent::moonraker(MoonrakerEvent::PrintProgress {
                filename: Some("cube.gcode".to_string()),
                progress_percent: Some(1),
                elapsed_seconds: Some(60),
                remaining_seconds: Some(3600),
            }))
            .await
            .unwrap();

        let request = thumbnail_request_rx.recv().await.unwrap();
        assert_eq!(request.key(), &ThumbnailKey::print("cube.gcode"));

        thumbnail_result_tx
            .send(crate::thumbnail::ThumbnailResult {
                key: request.key().clone(),
                result: Ok(vec![HmiCommand::raw("cp0.write(\"abc\")")]),
            })
            .await
            .unwrap();

        let mut observed_thumbnail = false;
        while let Some(command) = hmi_command_rx.recv().await {
            if command == HmiCommand::raw("cp0.write(\"abc\")") {
                observed_thumbnail = true;
                break;
            }
        }

        assert!(observed_thumbnail);

        drop(app_event_tx);
        drop(thumbnail_result_tx);
        runtime_handle.await.unwrap().unwrap();
    }

    #[test]
    fn file_slot_thumbnail_is_current_only_for_same_visible_file() {
        let (_app_event_tx, app_event_rx) = mpsc::channel(1);
        let (hmi_command_tx, _hmi_command_rx) = mpsc::channel(1);

        let mut runtime = Runtime::new(AppRunner::new(), app_event_rx, hmi_command_tx);
        runtime.runner.state.set_page(Page::Files);
        runtime
            .runner
            .state
            .files
            .set_visible_slot(0, Some(FileSlot::new("A.gcode", "A.gcode")));

        assert!(
            runtime.thumbnail_is_current(&ThumbnailKey::file_slot("A.gcode", 0)),
            "same file in same slot must be current"
        );

        runtime
            .runner
            .state
            .files
            .set_visible_slot(0, Some(FileSlot::new("D.gcode", "D.gcode")));

        assert!(
            !runtime.thumbnail_is_current(&ThumbnailKey::file_slot("A.gcode", 0)),
            "old file must not render into a reused slot"
        );
    }
}
