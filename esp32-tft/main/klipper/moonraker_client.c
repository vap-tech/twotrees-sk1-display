#include "moonraker_client.h"

#include "app_state.h"
#include "files_cache.h"
#include "wifi_manager.h"

#include "cJSON.h"
#include "esp_http_client.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/queue.h"
#include "freertos/task.h"
#include "sdkconfig.h"

#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define HTTP_RESPONSE_MAX 32768
#define POLL_INTERVAL_MS 2000
#define FILE_REQUEST_PATH_MAX 256

typedef struct {
  char path[FILE_REQUEST_PATH_MAX];
} file_request_t;

typedef struct {
  enum {
    PRINT_COMMAND_START,
    PRINT_COMMAND_PAUSE,
    PRINT_COMMAND_RESUME,
    PRINT_COMMAND_CANCEL,
  } type;
  char filename[FILE_REQUEST_PATH_MAX];
  uint32_t generation;
} print_command_request_t;

typedef struct {
  char data[HTTP_RESPONSE_MAX];
  size_t length;
  bool overflow;
} http_response_t;

typedef struct {
  size_t index;
  int percent;
} fan_command_request_t;

static const char *TAG = "moonraker";
static QueueHandle_t file_request_queue;
static QueueHandle_t print_command_queue;
static QueueHandle_t caselight_command_queue;
static QueueHandle_t clear_print_queue;
static QueueHandle_t fan_command_queue;
static char estimated_time_filename[APP_STATE_FILE_MAX];
static int estimated_time_seconds;

static void url_encode_path(const char *source, char *destination,
                            size_t size) {
  static const char hex[] = "0123456789ABCDEF";
  size_t output = 0;
  while (*source != '\0' && output + 1 < size) {
    unsigned char value = (unsigned char)*source++;
    bool safe = (value >= 'a' && value <= 'z') ||
                (value >= 'A' && value <= 'Z') ||
                (value >= '0' && value <= '9') || value == '-' ||
                value == '_' || value == '.' || value == '/';
    if (safe) {
      destination[output++] = (char)value;
    } else if (output + 3 < size) {
      destination[output++] = '%';
      destination[output++] = hex[value >> 4];
      destination[output++] = hex[value & 0x0f];
    } else {
      break;
    }
  }
  destination[output] = '\0';
}

static esp_err_t http_event_handler(esp_http_client_event_t *event) {
  http_response_t *response = event->user_data;
  if (event->event_id != HTTP_EVENT_ON_DATA || response == NULL) {
    return ESP_OK;
  }

  size_t available = sizeof(response->data) - response->length - 1;
  size_t copy_length = event->data_len;
  if (copy_length > available) {
    copy_length = available;
    response->overflow = true;
  }
  memcpy(response->data + response->length, event->data, copy_length);
  response->length += copy_length;
  response->data[response->length] = '\0';
  return ESP_OK;
}

static cJSON *moonraker_request(const char *path,
                               esp_http_client_method_t method) {
  char url[256];
  snprintf(url, sizeof(url), "http://%s:%d%s", CONFIG_SK1_MOONRAKER_HOST,
           CONFIG_SK1_MOONRAKER_PORT, path);

  http_response_t *response = calloc(1, sizeof(*response));
  if (response == NULL) {
    return NULL;
  }
  esp_http_client_config_t config = {
      .url = url,
      .method = method,
      .timeout_ms = 1500,
      .event_handler = http_event_handler,
      .user_data = response,
      .buffer_size = 1024,
  };
  esp_http_client_handle_t client = esp_http_client_init(&config);
  if (client == NULL) {
    free(response);
    return NULL;
  }

  esp_err_t result = esp_http_client_perform(client);
  int status_code = esp_http_client_get_status_code(client);
  esp_http_client_cleanup(client);

  if (result != ESP_OK || status_code != 200 || response->overflow) {
    ESP_LOGW(TAG, "GET %s failed: err=%s status=%d overflow=%d", path,
             esp_err_to_name(result), status_code, response->overflow);
    free(response);
    return NULL;
  }
  cJSON *root = cJSON_Parse(response->data);
  free(response);
  return root;
}

static cJSON *moonraker_get(const char *path) {
  return moonraker_request(path, HTTP_METHOD_GET);
}

static const char *json_string(cJSON *object, const char *name) {
  cJSON *item = cJSON_GetObjectItemCaseSensitive(object, name);
  return cJSON_IsString(item) ? item->valuestring : "";
}

static double json_number(cJSON *object, const char *name) {
  cJSON *item = cJSON_GetObjectItemCaseSensitive(object, name);
  return cJSON_IsNumber(item) ? item->valuedouble : 0.0;
}

static int poll_server_info(void) {
  cJSON *root = moonraker_get("/server/info");
  if (root == NULL) {
    return -1;
  }
  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  if (!cJSON_IsObject(result)) {
    cJSON_Delete(root);
    return -1;
  }

  const char *version = json_string(result, "moonraker_version");
  const char *klippy_state = json_string(result, "klippy_state");
  app_state_set_server_info(version, klippy_state);
  ESP_LOGI(TAG, "server=%s klippy=%s", version, klippy_state);
  int ready = strcmp(klippy_state, "ready") == 0 ? 1 : 0;
  cJSON_Delete(root);
  return ready;
}

static void poll_printer_info(void) {
  cJSON *root = moonraker_get("/printer/info");
  if (root == NULL) {
    return;
  }
  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  if (cJSON_IsObject(result)) {
    const char *printer_state = json_string(result, "state");
    app_state_set_printer_info(printer_state);
    ESP_LOGI(TAG, "printer=%s", printer_state);
  }
  cJSON_Delete(root);
}

static void poll_printer_status(void) {
  cJSON *root = moonraker_get(
      "/printer/objects/"
      "query?webhooks&extruder&heater_bed&print_stats&virtual_sdcard&"
      "output_pin%20caselight&fan&fan_generic%20Side_fan&"
      "fan_generic%20Filter_fan");
  if (root == NULL) {
    return;
  }

  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  cJSON *status = cJSON_GetObjectItemCaseSensitive(result, "status");
  cJSON *extruder = cJSON_GetObjectItemCaseSensitive(status, "extruder");
  cJSON *heater_bed = cJSON_GetObjectItemCaseSensitive(status, "heater_bed");
  cJSON *print_stats = cJSON_GetObjectItemCaseSensitive(status, "print_stats");
  cJSON *virtual_sdcard =
      cJSON_GetObjectItemCaseSensitive(status, "virtual_sdcard");
  cJSON *caselight =
      cJSON_GetObjectItemCaseSensitive(status, "output_pin caselight");
  cJSON *fans[3] = {
      cJSON_GetObjectItemCaseSensitive(status, "fan"),
      cJSON_GetObjectItemCaseSensitive(status, "fan_generic Side_fan"),
      cJSON_GetObjectItemCaseSensitive(status, "fan_generic Filter_fan"),
  };

  if (cJSON_IsObject(extruder) && cJSON_IsObject(heater_bed)) {
    int nozzle_current = (int)lround(json_number(extruder, "temperature"));
    int nozzle_target = (int)lround(json_number(extruder, "target"));
    int bed_current = (int)lround(json_number(heater_bed, "temperature"));
    int bed_target = (int)lround(json_number(heater_bed, "target"));
    int progress = (int)lround(json_number(virtual_sdcard, "progress") * 100.0);
    const char *print_state = json_string(print_stats, "state");
    const char *print_filename = json_string(print_stats, "filename");
    int print_elapsed_seconds =
        (int)lround(json_number(print_stats, "print_duration"));

    if (print_filename[0] != '\0' &&
        strcmp(estimated_time_filename, print_filename) != 0) {
      estimated_time_filename[0] = '\0';
      estimated_time_seconds = 0;
      char encoded_filename[APP_STATE_FILE_MAX * 3];
      char metadata_path[sizeof(encoded_filename) + 48];
      url_encode_path(print_filename, encoded_filename,
                      sizeof(encoded_filename));
      snprintf(metadata_path, sizeof(metadata_path),
               "/server/files/metadata?filename=%s", encoded_filename);
      cJSON *metadata_root = moonraker_get(metadata_path);
      if (metadata_root != NULL) {
        cJSON *metadata =
            cJSON_GetObjectItemCaseSensitive(metadata_root, "result");
        if (cJSON_IsObject(metadata)) {
          snprintf(estimated_time_filename, sizeof(estimated_time_filename),
                   "%s", print_filename);
          estimated_time_seconds =
              (int)lround(json_number(metadata, "estimated_time"));
          ESP_LOGI(TAG, "estimated print time=%ds file=%s",
                   estimated_time_seconds, print_filename);
        }
        cJSON_Delete(metadata_root);
      }
    }
    app_printer_update_t update = {
        .has_nozzle_current = true,
        .nozzle_current = nozzle_current,
        .has_nozzle_target = true,
        .nozzle_target = nozzle_target,
        .has_bed_current = true,
        .bed_current = bed_current,
        .has_bed_target = true,
        .bed_target = bed_target,
        .has_caselight = cJSON_IsObject(caselight),
        .caselight_on = json_number(caselight, "value") > 0.5,
        .has_print_state = cJSON_IsObject(print_stats),
        .print_state = print_state,
        .has_print_filename = print_filename[0] != '\0',
        .print_filename = print_filename,
        .has_print_progress = cJSON_IsObject(virtual_sdcard),
        .print_progress = progress,
        .has_print_elapsed_seconds = cJSON_IsObject(print_stats),
        .print_elapsed_seconds = print_elapsed_seconds,
        .has_print_estimated_seconds =
            print_filename[0] != '\0' && estimated_time_seconds > 0,
        .print_estimated_seconds = estimated_time_seconds,
    };
    for (size_t index = 0; index < 3; ++index) {
      update.has_fan_values[index] = cJSON_IsObject(fans[index]);
      update.fan_values[index] =
          (int)lround(json_number(fans[index], "speed") * 100.0);
    }
    app_state_apply_printer_update(&update);
    ESP_LOGI(TAG, "status nozzle=%d/%d bed=%d/%d print=%s %d%%", nozzle_current,
             nozzle_target, bed_current, bed_target, print_state, progress);
  }
  cJSON_Delete(root);
}

static void process_fan_command(void) {
  fan_command_request_t request;
  if (xQueueReceive(fan_command_queue, &request, 0) != pdTRUE) {
    return;
  }

  char path[160];
  if (request.index == 0) {
    int pwm = (int)lround(request.percent * 255.0 / 100.0);
    snprintf(path, sizeof(path),
             "/printer/gcode/script?script=M106%%20S%d", pwm);
  } else {
    const char *name = request.index == 1 ? "Side_fan" : "Filter_fan";
    snprintf(path, sizeof(path),
             "/printer/gcode/script?script=SET_FAN_SPEED%%20FAN%%3D%s"
             "%%20SPEED%%3D%.2f",
             name, request.percent / 100.0);
  }

  cJSON *root = moonraker_request(path, HTTP_METHOD_POST);
  if (root == NULL) {
    ESP_LOGW(TAG, "fan %u command failed", (unsigned)request.index);
    return;
  }
  ESP_LOGI(TAG, "fan %u set to %d%%", (unsigned)request.index,
           request.percent);
  cJSON_Delete(root);
}

static void process_caselight_command(void) {
  bool enabled;
  if (xQueueReceive(caselight_command_queue, &enabled, 0) != pdTRUE) {
    return;
  }

  const char *path =
      enabled
          ? "/printer/gcode/script?script=SET_PIN%20PIN%3Dcaselight%20VALUE%3D1"
          : "/printer/gcode/script?script=SET_PIN%20PIN%3Dcaselight%20VALUE%3D0";
  cJSON *root = moonraker_request(path, HTTP_METHOD_POST);
  if (root == NULL) {
    ESP_LOGW(TAG, "caselight command failed");
    return;
  }
  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  ESP_LOGI(TAG, "caselight %s: %s", enabled ? "on" : "off",
           cJSON_IsString(result) ? result->valuestring : "unexpected response");
  cJSON_Delete(root);
}

static void process_clear_print(void) {
  bool requested;
  if (xQueueReceive(clear_print_queue, &requested, 0) != pdTRUE ||
      !requested) {
    return;
  }

  cJSON *root = moonraker_request(
      "/printer/gcode/script?script=SDCARD_RESET_FILE", HTTP_METHOD_POST);
  if (root == NULL) {
    ESP_LOGW(TAG, "clear completed print failed");
    return;
  }
  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  ESP_LOGI(TAG, "clear completed print: %s",
           cJSON_IsString(result) ? result->valuestring
                                  : "unexpected response");
  cJSON_Delete(root);
}

static void make_entry_path(char *destination, size_t size,
                            const char *directory, const char *name) {
  if (strcmp(directory, "gcodes") == 0) {
    snprintf(destination, size, "%s", name);
  } else {
    snprintf(destination, size, "%s/%s", directory + strlen("gcodes/"), name);
  }
}

static void append_directory_entries(file_entry_t *entries, size_t *count,
                                     const char *path, cJSON *directories) {
  cJSON *item;
  cJSON_ArrayForEach(item, directories) {
    if (*count >= FILES_CACHE_MAX_ENTRIES) {
      return;
    }
    const char *name = json_string(item, "dirname");
    if (name[0] == '\0' || name[0] == '.') {
      continue;
    }
    file_entry_t *entry = &entries[(*count)++];
    snprintf(entry->name, sizeof(entry->name), "%s", name);
    make_entry_path(entry->path, sizeof(entry->path), path, name);
    entry->kind = FILE_KIND_DIR;
    entry->modified = (int64_t)json_number(item, "modified");
    entry->size = (size_t)json_number(item, "size");
  }
}

static void append_file_entries(file_entry_t *entries, size_t *count,
                                const char *path, cJSON *files) {
  cJSON *item;
  cJSON_ArrayForEach(item, files) {
    if (*count >= FILES_CACHE_MAX_ENTRIES) {
      return;
    }
    const char *name = json_string(item, "filename");
    if (name[0] == '\0' || name[0] == '.') {
      continue;
    }
    file_entry_t *entry = &entries[(*count)++];
    snprintf(entry->name, sizeof(entry->name), "%s", name);
    make_entry_path(entry->path, sizeof(entry->path), path, name);
    entry->kind = FILE_KIND_FILE;
    entry->modified = (int64_t)json_number(item, "modified");
    entry->size = (size_t)json_number(item, "size");
  }
}

static void refresh_files(const char *path) {
  char encoded_path[FILE_REQUEST_PATH_MAX * 3];
  char request_path[sizeof(encoded_path) + 64];
  url_encode_path(path, encoded_path, sizeof(encoded_path));
  snprintf(request_path, sizeof(request_path),
           "/server/files/directory?path=%s", encoded_path);

  files_cache_set_loading(path);
  cJSON *root = moonraker_get(request_path);
  if (root == NULL) {
    files_cache_set_error(path);
    return;
  }

  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  cJSON *directories = cJSON_GetObjectItemCaseSensitive(result, "dirs");
  cJSON *files = cJSON_GetObjectItemCaseSensitive(result, "files");
  if (!cJSON_IsObject(result) || !cJSON_IsArray(directories) ||
      !cJSON_IsArray(files)) {
    files_cache_set_error(path);
    cJSON_Delete(root);
    return;
  }

  file_entry_t *entries = calloc(FILES_CACHE_MAX_ENTRIES, sizeof(*entries));
  if (entries == NULL) {
    files_cache_set_error(path);
    cJSON_Delete(root);
    return;
  }
  size_t count = 0;
  append_directory_entries(entries, &count, path, directories);
  append_file_entries(entries, &count, path, files);
  files_cache_replace(path, entries, count);
  ESP_LOGI(TAG, "files path=%s count=%u", path, (unsigned)count);
  free(entries);
  cJSON_Delete(root);
}

static void process_file_request(void) {
  file_request_t request;
  if (xQueueReceive(file_request_queue, &request, 0) == pdTRUE) {
    refresh_files(request.path);
  }
}

static const char *print_command_name(int type) {
  switch (type) {
  case PRINT_COMMAND_START:
    return "print_start";
  case PRINT_COMMAND_PAUSE:
    return "print_pause";
  case PRINT_COMMAND_RESUME:
    return "print_resume";
  case PRINT_COMMAND_CANCEL:
    return "print_cancel";
  default:
    return "print_unknown";
  }
}

static void process_print_command(void) {
  print_command_request_t request;
  if (xQueueReceive(print_command_queue, &request, 0) != pdTRUE) {
    return;
  }

  char request_path[FILE_REQUEST_PATH_MAX * 3 + 48];
  if (request.type == PRINT_COMMAND_START) {
    char encoded_filename[FILE_REQUEST_PATH_MAX * 3];
    url_encode_path(request.filename, encoded_filename,
                    sizeof(encoded_filename));
    snprintf(request_path, sizeof(request_path),
             "/printer/print/start?filename=%s", encoded_filename);
  } else {
    const char *action = request.type == PRINT_COMMAND_PAUSE
                             ? "pause"
                             : request.type == PRINT_COMMAND_RESUME ? "resume"
                                                                    : "cancel";
    snprintf(request_path, sizeof(request_path), "/printer/print/%s", action);
  }

  cJSON *root = moonraker_request(request_path, HTTP_METHOD_POST);
  if (root == NULL) {
    app_state_command_finish(request.generation, false,
                             "Moonraker rejected print command");
    return;
  }
  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  bool succeeded = cJSON_IsString(result) &&
                   strcmp(result->valuestring, "ok") == 0;
  app_state_command_finish(request.generation, succeeded,
                           succeeded ? "ok" : "Print command failed");
  ESP_LOGI(TAG, "%s%s%s: %s", print_command_name(request.type),
           request.filename[0] ? " " : "", request.filename,
           succeeded ? "ok" : "failed");
  cJSON_Delete(root);
}

static void moonraker_task(void *argument) {
  (void)argument;
  while (true) {
    if (wifi_manager_wait_connected(pdMS_TO_TICKS(5000)) != pdTRUE) {
      app_state_set_moonraker_online(false);
      continue;
    }

    int server_state = poll_server_info();
    if (server_state < 0) {
      app_state_set_moonraker_online(false);
      vTaskDelay(pdMS_TO_TICKS(POLL_INTERVAL_MS));
      continue;
    }
    if (server_state == 0) {
      vTaskDelay(pdMS_TO_TICKS(POLL_INTERVAL_MS));
      continue;
    }

    process_file_request();
    process_print_command();
    process_fan_command();
    process_caselight_command();
    process_clear_print();
    poll_printer_info();
    poll_printer_status();
    process_clear_print();
    process_caselight_command();
    process_fan_command();
    process_print_command();
    process_file_request();
    vTaskDelay(pdMS_TO_TICKS(POLL_INTERVAL_MS));
  }
}

esp_err_t moonraker_client_start(void) {
  file_request_queue = xQueueCreate(1, sizeof(file_request_t));
  print_command_queue = xQueueCreate(1, sizeof(print_command_request_t));
  caselight_command_queue = xQueueCreate(1, sizeof(bool));
  clear_print_queue = xQueueCreate(1, sizeof(bool));
  fan_command_queue = xQueueCreate(3, sizeof(fan_command_request_t));
  if (file_request_queue == NULL || print_command_queue == NULL ||
      caselight_command_queue == NULL || clear_print_queue == NULL ||
      fan_command_queue == NULL) {
    return ESP_ERR_NO_MEM;
  }
  return xTaskCreate(moonraker_task, "moonraker", 8192, NULL, 7, NULL) == pdPASS
             ? ESP_OK
             : ESP_ERR_NO_MEM;
}

esp_err_t moonraker_start_print(const char *filename) {
  if (filename == NULL || filename[0] == '\0' ||
      print_command_queue == NULL) {
    return ESP_ERR_INVALID_ARG;
  }
  print_command_request_t request = {.type = PRINT_COMMAND_START};
  snprintf(request.filename, sizeof(request.filename), "%s", filename);
  request.generation = app_state_command_begin("print_start");
  if (xQueueOverwrite(print_command_queue, &request) != pdTRUE) {
    app_state_command_finish(request.generation, false,
                             "Print command queue unavailable");
    return ESP_FAIL;
  }
  return ESP_OK;
}

static esp_err_t queue_print_command(int type) {
  if (print_command_queue == NULL) {
    return ESP_ERR_INVALID_STATE;
  }
  print_command_request_t request = {.type = type};
  const char *name = print_command_name(type);
  request.generation = app_state_command_begin(name);
  if (xQueueOverwrite(print_command_queue, &request) != pdTRUE) {
    app_state_command_finish(request.generation, false,
                             "Print command queue unavailable");
    return ESP_FAIL;
  }
  return ESP_OK;
}

esp_err_t moonraker_pause_print(void) {
  return queue_print_command(PRINT_COMMAND_PAUSE);
}

esp_err_t moonraker_resume_print(void) {
  return queue_print_command(PRINT_COMMAND_RESUME);
}

esp_err_t moonraker_cancel_print(void) {
  return queue_print_command(PRINT_COMMAND_CANCEL);
}

esp_err_t moonraker_set_fan_speed(size_t index, int percent) {
  if (fan_command_queue == NULL || index >= 3) {
    return ESP_ERR_INVALID_ARG;
  }
  fan_command_request_t request = {
      .index = index,
      .percent = percent < 0 ? 0 : percent > 100 ? 100 : percent,
  };
  app_state_set_fan_value(index, request.percent);
  return xQueueSend(fan_command_queue, &request, 0) == pdTRUE ? ESP_OK
                                                              : ESP_FAIL;
}

esp_err_t moonraker_set_caselight(bool enabled) {
  if (caselight_command_queue == NULL) {
    return ESP_ERR_INVALID_STATE;
  }
  return xQueueOverwrite(caselight_command_queue, &enabled) == pdTRUE
             ? ESP_OK
             : ESP_FAIL;
}

esp_err_t moonraker_clear_print(void) {
  if (clear_print_queue == NULL) {
    return ESP_ERR_INVALID_STATE;
  }
  bool requested = true;
  return xQueueOverwrite(clear_print_queue, &requested) == pdTRUE
             ? ESP_OK
             : ESP_FAIL;
}

esp_err_t moonraker_request_files(const char *path) {
  if (path == NULL || path[0] == '\0') {
    return ESP_ERR_INVALID_ARG;
  }
  file_request_t request = {0};
  snprintf(request.path, sizeof(request.path), "%s", path);
  files_cache_set_loading(request.path);
  return xQueueOverwrite(file_request_queue, &request) == pdTRUE ? ESP_OK
                                                                 : ESP_FAIL;
}
