#include "app_state.h"
#include "app_watchdog.h"
#include "display_uart.h"
#include "files_cache.h"
#include "moonraker_client.h"
#include "moonraker_websocket.h"
#include "thumbnail_worker.h"
#include "thumbnail_cache.h"
#include "ui_controller.h"
#include "wifi_manager.h"

#include "esp_err.h"
#include "esp_log.h"
#include "nvs_flash.h"

static const char *TAG = "app";

void app_main(void) {
  esp_err_t nvs_result = nvs_flash_init();
  if (nvs_result == ESP_ERR_NVS_NO_FREE_PAGES ||
      nvs_result == ESP_ERR_NVS_NEW_VERSION_FOUND) {
    ESP_ERROR_CHECK(nvs_flash_erase());
    nvs_result = nvs_flash_init();
  }
  ESP_ERROR_CHECK(nvs_result);

  app_state_init();
  files_cache_init();
  thumbnail_cache_init();
  ESP_ERROR_CHECK(display_uart_init());
  ESP_ERROR_CHECK(ui_controller_start());
  ESP_ERROR_CHECK(app_watchdog_start());
  ESP_ERROR_CHECK(wifi_manager_start());
  ESP_ERROR_CHECK(moonraker_client_start());
  ESP_ERROR_CHECK(moonraker_websocket_start());
  ESP_ERROR_CHECK(thumbnail_worker_start());
  ESP_LOGI(TAG, "SK1 display client started");
}
