#include "app_watchdog.h"

#include "display_uart.h"

#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/task.h"

static const char *TAG = "watchdog";

static void watchdog_task(void *argument) {
  (void)argument;
  while (true) {
    int64_t now = esp_timer_get_time() / 1000;
    int64_t rx_idle = now - display_last_rx_ms();
    int64_t tx_idle = now - display_last_tx_ms();

    if (rx_idle > 60000) {
      ESP_LOGW(TAG, "display RX idle for %lld ms", rx_idle);
    }
    if (tx_idle > 60000) {
      ESP_LOGW(TAG, "display TX idle for %lld ms", tx_idle);
    }
    vTaskDelay(pdMS_TO_TICKS(10000));
  }
}

esp_err_t app_watchdog_start(void) {
  return xTaskCreate(watchdog_task, "app_watchdog", 3072, NULL, 11, NULL) ==
                 pdPASS
             ? ESP_OK
             : ESP_ERR_NO_MEM;
}
