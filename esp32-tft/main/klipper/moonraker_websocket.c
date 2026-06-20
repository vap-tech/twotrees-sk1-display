#include "moonraker_websocket.h"

#include "app_state.h"
#include "wifi_manager.h"

#include "cJSON.h"
#include "esp_log.h"
#include "esp_websocket_client.h"
#include "freertos/task.h"
#include "sdkconfig.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define WS_MESSAGE_MAX 16384

static const char *TAG = "moonraker_ws";
static esp_websocket_client_handle_t client;
static char *message_buffer;
static size_t message_length;

static bool json_number_value(cJSON *object, const char *name, double *value) {
  cJSON *item = cJSON_GetObjectItemCaseSensitive(object, name);
  if (!cJSON_IsNumber(item)) {
    return false;
  }
  *value = item->valuedouble;
  return true;
}

static void apply_status(cJSON *status) {
  if (!cJSON_IsObject(status)) {
    return;
  }

  app_printer_update_t update = {0};
  double value;
  cJSON *extruder = cJSON_GetObjectItemCaseSensitive(status, "extruder");
  if (cJSON_IsObject(extruder)) {
    if (json_number_value(extruder, "temperature", &value)) {
      update.has_nozzle_current = true;
      update.nozzle_current = (int)lround(value);
    }
    if (json_number_value(extruder, "target", &value)) {
      update.has_nozzle_target = true;
      update.nozzle_target = (int)lround(value);
    }
  }

  cJSON *heater_bed = cJSON_GetObjectItemCaseSensitive(status, "heater_bed");
  if (cJSON_IsObject(heater_bed)) {
    if (json_number_value(heater_bed, "temperature", &value)) {
      update.has_bed_current = true;
      update.bed_current = (int)lround(value);
    }
    if (json_number_value(heater_bed, "target", &value)) {
      update.has_bed_target = true;
      update.bed_target = (int)lround(value);
    }
  }

  cJSON *caselight =
      cJSON_GetObjectItemCaseSensitive(status, "output_pin caselight");
  if (cJSON_IsObject(caselight) &&
      json_number_value(caselight, "value", &value)) {
    update.has_caselight = true;
    update.caselight_on = value > 0.5;
    ESP_LOGI(TAG, "caselight update=%s",
             update.caselight_on ? "on" : "off");
  }

  static const char *fan_names[3] = {
      "fan", "fan_generic Side_fan", "fan_generic Filter_fan"};
  for (size_t index = 0; index < 3; ++index) {
    cJSON *fan = cJSON_GetObjectItemCaseSensitive(status, fan_names[index]);
    if (cJSON_IsObject(fan) && json_number_value(fan, "speed", &value)) {
      update.has_fan_values[index] = true;
      update.fan_values[index] = (int)lround(value * 100.0);
    }
  }

  cJSON *print_stats = cJSON_GetObjectItemCaseSensitive(status, "print_stats");
  if (cJSON_IsObject(print_stats)) {
    cJSON *state = cJSON_GetObjectItemCaseSensitive(print_stats, "state");
    if (cJSON_IsString(state)) {
      update.has_print_state = true;
      update.print_state = state->valuestring;
    }
    cJSON *filename =
        cJSON_GetObjectItemCaseSensitive(print_stats, "filename");
    if (cJSON_IsString(filename)) {
      update.has_print_filename = true;
      update.print_filename = filename->valuestring;
    }
    if (json_number_value(print_stats, "print_duration", &value)) {
      update.has_print_elapsed_seconds = true;
      update.print_elapsed_seconds = (int)lround(value);
    }
  }

  cJSON *virtual_sdcard =
      cJSON_GetObjectItemCaseSensitive(status, "virtual_sdcard");
  if (cJSON_IsObject(virtual_sdcard) &&
      json_number_value(virtual_sdcard, "progress", &value)) {
    update.has_print_progress = true;
    update.print_progress = (int)lround(value * 100.0);
  }

  app_state_apply_printer_update(&update);
}

static void process_message(const char *data) {
  cJSON *root = cJSON_Parse(data);
  if (root == NULL) {
    ESP_LOGW(TAG, "invalid JSON message");
    return;
  }

  cJSON *method = cJSON_GetObjectItemCaseSensitive(root, "method");
  if (cJSON_IsString(method) &&
      strcmp(method->valuestring, "notify_status_update") == 0) {
    cJSON *params = cJSON_GetObjectItemCaseSensitive(root, "params");
    apply_status(cJSON_GetArrayItem(params, 0));
  } else {
    cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
    apply_status(cJSON_GetObjectItemCaseSensitive(result, "status"));
  }
  cJSON_Delete(root);
}

static void receive_data(const esp_websocket_event_data_t *data) {
  if (data->op_code != 0x1 && data->op_code != 0x0) {
    return;
  }
  if (data->payload_offset == 0) {
    message_length = 0;
  }
  if (data->payload_len <= 0 || data->payload_len >= WS_MESSAGE_MAX ||
      data->payload_offset < 0 || data->data_len < 0 ||
      (size_t)data->payload_offset + (size_t)data->data_len >=
          WS_MESSAGE_MAX) {
    message_length = 0;
    ESP_LOGW(TAG, "discarding oversized websocket message");
    return;
  }
  memcpy(message_buffer + data->payload_offset, data->data_ptr, data->data_len);
  message_length = data->payload_offset + data->data_len;
  if (message_length >= (size_t)data->payload_len && data->fin) {
    message_buffer[message_length] = '\0';
    process_message(message_buffer);
    message_length = 0;
  }
}

static void websocket_event(void *handler_args, esp_event_base_t base,
                            int32_t event_id, void *event_data) {
  (void)handler_args;
  (void)base;
  esp_websocket_event_data_t *data = event_data;
  switch (event_id) {
  case WEBSOCKET_EVENT_CONNECTED: {
    static const char subscription[] =
        "{\"jsonrpc\":\"2.0\",\"method\":\"printer.objects.subscribe\","
        "\"params\":{\"objects\":{\"extruder\":null,\"heater_bed\":null,"
        "\"print_stats\":null,\"virtual_sdcard\":null,"
        "\"output_pin caselight\":null,\"fan\":null,"
        "\"fan_generic Side_fan\":null,"
        "\"fan_generic Filter_fan\":null}},\"id\":1}";
    ESP_LOGI(TAG, "connected");
    int sent = esp_websocket_client_send_text(
        client, subscription, strlen(subscription), pdMS_TO_TICKS(1000));
    if (sent < 0) {
      ESP_LOGW(TAG, "subscription send failed");
    }
    break;
  }
  case WEBSOCKET_EVENT_DISCONNECTED:
    ESP_LOGW(TAG, "disconnected");
    message_length = 0;
    break;
  case WEBSOCKET_EVENT_DATA:
    receive_data(data);
    break;
  case WEBSOCKET_EVENT_ERROR:
    ESP_LOGW(TAG, "connection error");
    break;
  default:
    break;
  }
}

static void websocket_lifecycle_task(void *argument) {
  (void)argument;
  while (true) {
    if (wifi_manager_wait_connected(portMAX_DELAY) != pdTRUE) {
      continue;
    }
    ESP_LOGI(TAG, "Wi-Fi ready, starting client");
    esp_err_t result = esp_websocket_client_start(client);
    if (result != ESP_OK && result != ESP_ERR_INVALID_STATE) {
      ESP_LOGW(TAG, "start failed: %s", esp_err_to_name(result));
      vTaskDelay(pdMS_TO_TICKS(1000));
      continue;
    }

    wifi_manager_wait_disconnected(portMAX_DELAY);
    ESP_LOGI(TAG, "Wi-Fi lost, stopping client");
    result = esp_websocket_client_stop(client);
    if (result != ESP_OK && result != ESP_ERR_INVALID_STATE) {
      ESP_LOGW(TAG, "stop failed: %s", esp_err_to_name(result));
    }
    message_length = 0;
  }
}

esp_err_t moonraker_websocket_start(void) {
  char uri[96];
  snprintf(uri, sizeof(uri), "ws://%s:%d/websocket",
           CONFIG_SK1_MOONRAKER_HOST, CONFIG_SK1_MOONRAKER_PORT);
  message_buffer = malloc(WS_MESSAGE_MAX);
  if (message_buffer == NULL) {
    return ESP_ERR_NO_MEM;
  }

  esp_websocket_client_config_t config = {
      .uri = uri,
      .task_prio = 7,
      .task_name = "moonraker_ws",
      .task_stack = 6144,
      .buffer_size = 2048,
      .reconnect_timeout_ms = 2000,
      .network_timeout_ms = 3000,
      .ping_interval_sec = 10,
  };
  client = esp_websocket_client_init(&config);
  if (client == NULL) {
    free(message_buffer);
    message_buffer = NULL;
    return ESP_ERR_NO_MEM;
  }
  esp_err_t result = esp_websocket_register_events(
      client, WEBSOCKET_EVENT_ANY, websocket_event, NULL);
  if (result != ESP_OK) {
    esp_websocket_client_destroy(client);
    client = NULL;
    free(message_buffer);
    message_buffer = NULL;
    return result;
  }
  if (xTaskCreate(websocket_lifecycle_task, "moonraker_ws_lifecycle", 3072,
                  NULL, 7, NULL) != pdPASS) {
    esp_websocket_client_destroy(client);
    client = NULL;
    free(message_buffer);
    message_buffer = NULL;
    return ESP_ERR_NO_MEM;
  }
  return ESP_OK;
}
