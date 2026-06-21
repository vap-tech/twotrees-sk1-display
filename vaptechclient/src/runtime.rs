use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::app::event::AppEvent;
use crate::app::runner::AppRunner;
use crate::hmi::command::HmiCommand;

pub struct Runtime {
    runner: AppRunner,
    app_event_rx: mpsc::Receiver<AppEvent>,
    hmi_command_tx: mpsc::Sender<HmiCommand>,
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
        }
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(event) = self.app_event_rx.recv().await {
            self.runner.handle_event(event)?;

            for command in self.runner.drain_hmi_commands() {
                tracing::debug!(%command, "HMI command produced by render diff");

                self.hmi_command_tx
                    .send(command)
                    .await
                    .context("failed to queue HMI command")?;
            }

            for request in self.runner.drain_moonraker_requests() {
                tracing::debug!(?request, "moonraker request queued");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hmi::event::HmiEvent;

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
}
