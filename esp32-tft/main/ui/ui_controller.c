#include "ui_controller.h"

#include "app_state.h"
#include "display_events.h"
#include "display_uart.h"
#include "files_cache.h"
#include "moonraker_client.h"
#include "ui_pages_files.h"
#include "ui_pages_home.h"
#include "ui_pages_print.h"
#include "ui_state.h"

#include "esp_log.h"
#include "freertos/task.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *TAG = "ui";
static ui_state_t state;
static bool init_running;
static int64_t last_init_ms;

static void show_page(uint8_t page) {
  char command[24];
  display_cancel_low_priority();
  snprintf(command, sizeof(command), "page %u", page);
  display_send_async(command, DISPLAY_CMD_HIGH);
  ui_state_lock(&state);
  state.page = page;
  ui_state_unlock(&state);
}

static void init_sequence_task(void *argument) {
  (void)argument;
  display_cancel_low_priority();
  show_page(45);
  vTaskDelay(pdMS_TO_TICKS(1000));
  ui_show_home(&state);
  init_running = false;
  vTaskDelete(NULL);
}

static void request_init_sequence(void) {
  if (init_running) {
    return;
  }
  init_running = true;
  if (xTaskCreate(init_sequence_task, "ui_init", 3072, NULL, 6, NULL) !=
      pdPASS) {
    init_running = false;
    ESP_LOGE(TAG, "failed to create init task");
  }
}

static bool navigation_blocked(uint8_t page) {
  switch (page) {
  case 27:
  case 45:
  case 51:
  case 56:
  case 62:
  case 67:
  case 68:
  case 73:
  case 74:
  case 77:
    return true;
  default:
    return false;
  }
}

static bool print_screen_active(app_print_state_t print_mode) {
  return print_mode == APP_PRINT_STARTING ||
         print_mode == APP_PRINT_PRINTING ||
         print_mode == APP_PRINT_PAUSING ||
         print_mode == APP_PRINT_PAUSED ||
         print_mode == APP_PRINT_RESUMING;
}

static void handle_navigation(uint8_t component) {
  switch (component) {
  case 0: {
    app_state_snapshot_t snapshot;
    app_state_get(&snapshot);
    if (print_screen_active(snapshot.print_mode)) {
      ui_show_printing(&state);
    } else {
      ui_show_home(&state);
    }
    break;
  }
  case 1:
    show_page(3);
    break;
  case 2:
    ui_state_lock(&state);
    snprintf(state.files_path, sizeof(state.files_path), "gcodes");
    state.files_page = 0;
    ui_state_unlock(&state);
    ui_show_files_loading(&state);
    moonraker_request_files("gcodes");
    break;
  case 3:
    show_page(11);
    break;
  case 4:
    show_page(21);
    break;
  }
}

static void render_current_files(void) {
  files_cache_snapshot_t *snapshot = malloc(sizeof(*snapshot));
  if (snapshot == NULL) {
    ESP_LOGE(TAG, "unable to allocate files snapshot");
    return;
  }
  files_cache_get(snapshot);
  ui_render_files(&state, snapshot);
  free(snapshot);
}

static void request_directory(const char *path) {
  ui_state_lock(&state);
  snprintf(state.files_path, sizeof(state.files_path), "%s", path);
  state.files_page = 0;
  ui_state_unlock(&state);
  ui_show_files_loading(&state);
  moonraker_request_files(path);
}

static void handle_file_slot(uint8_t component) {
  files_cache_snapshot_t *snapshot = malloc(sizeof(*snapshot));
  if (snapshot == NULL) {
    return;
  }
  files_cache_get(snapshot);

  ui_state_lock(&state);
  size_t entry_index = state.files_page * 3 + (component - 8);
  bool current_path = strcmp(state.files_path, snapshot->path) == 0;
  ui_state_unlock(&state);

  if (!snapshot->valid || !current_path || entry_index >= snapshot->count) {
    free(snapshot);
    return;
  }

  file_entry_t entry = snapshot->entries[entry_index];
  free(snapshot);

  if (entry.kind == FILE_KIND_DIR) {
    char directory[256];
    snprintf(directory, sizeof(directory), "gcodes/%s", entry.path);
    request_directory(directory);
    return;
  }

  ui_state_lock(&state);
  snprintf(state.selected_file_name, sizeof(state.selected_file_name), "%s",
           entry.name);
  snprintf(state.selected_file_path, sizeof(state.selected_file_path), "%s",
           entry.path);
  ui_state_unlock(&state);
  ui_show_file_preview(&state);
}

static void handle_files_navigation(uint8_t component) {
  if (component >= 8 && component <= 10) {
    handle_file_slot(component);
    return;
  }

  files_cache_snapshot_t *snapshot = malloc(sizeof(*snapshot));
  if (snapshot == NULL) {
    return;
  }
  files_cache_get(snapshot);

  if (component == 11 || component == 12) {
    ui_state_lock(&state);
    size_t max_page = snapshot->count == 0 ? 0 : (snapshot->count - 1) / 3;
    if (component == 11 && state.files_page > 0) {
      --state.files_page;
    } else if (component == 12 && state.files_page < max_page) {
      ++state.files_page;
    }
    ui_state_unlock(&state);
    ui_render_files(&state, snapshot);
    free(snapshot);
    return;
  }
  free(snapshot);

  if (component == 13) {
    char parent[FILE_ENTRY_PATH_MAX];
    ui_state_lock(&state);
    snprintf(parent, sizeof(parent), "%s", state.files_path);
    ui_state_unlock(&state);

    char *separator = strrchr(parent, '/');
    if (separator != NULL) {
      *separator = '\0';
      request_directory(parent);
    }
  }
}

static void handle_touch(uint8_t page, uint8_t component) {
  ESP_LOGI(TAG, "touch page=%u component=%u", page, component);

  if (!navigation_blocked(page) && component <= 4) {
    handle_navigation(component);
    return;
  }

  if ((page == 0 && component == 5) ||
      (page == 2 && component == 6)) {
    app_state_snapshot_t snapshot;
    app_state_get(&snapshot);
    if (moonraker_set_caselight(!snapshot.caselight_on) != ESP_OK) {
      ESP_LOGE(TAG, "unable to queue caselight command");
    }
    return;
  }

  if ((page == 0 && component == 6) ||
      (page == 2 && component == 7)) {
    ui_show_fans(&state);
    return;
  }
  if ((page == 0 || page == 2) && component == 9) {
    show_page(18);
    return;
  }
  if ((page == 0 || page == 2) && component == 10) {
    show_page(68);
    return;
  }
  if (page == 3 && component == 21) {
    show_page(4);
    return;
  }
  if ((page == 3 && component == 22) || (page == 4 && component == 11)) {
    ui_show_fans(&state);
    return;
  }
  if (page == 6 && component == 5) {
    show_page(3);
    return;
  }
  if (page == 7 && component >= 8 && component <= 13) {
    handle_files_navigation(component);
    return;
  }
  if (page == 9 && component == 6) {
    show_page(7);
    render_current_files();
    return;
  }
  if (page == 9 && component == 5) {
    show_page(3);
    return;
  }
  if (page == 9 && component == 7) {
    app_state_snapshot_t snapshot;
    app_state_get(&snapshot);
    if (snapshot.command_status == APP_COMMAND_PENDING &&
        strcmp(snapshot.command_name, "print_start") == 0) {
      ESP_LOGW(TAG, "print start already pending");
      return;
    }
    char file_path[FILE_ENTRY_PATH_MAX];
    ui_state_lock(&state);
    snprintf(file_path, sizeof(file_path), "%s", state.selected_file_path);
    ui_state_unlock(&state);
    if (moonraker_start_print(file_path) != ESP_OK) {
      ESP_LOGE(TAG, "unable to queue print start for %s", file_path);
    }
    return;
  }
  if (page == 2 && component == 5) {
    app_state_snapshot_t snapshot;
    app_state_get(&snapshot);
    if (snapshot.print_mode == APP_PRINT_PAUSED) {
      show_page(74);
      if (moonraker_resume_print() != ESP_OK) {
        ui_show_printing(&state);
      }
    } else if (snapshot.print_mode == APP_PRINT_PRINTING) {
      show_page(27);
    }
    return;
  }
  if (page == 27 && component == 0) {
    if (moonraker_pause_print() != ESP_OK) {
      ui_show_printing(&state);
    }
    return;
  }
  if (page == 27 && component == 1) {
    show_page(73);
    if (moonraker_cancel_print() != ESP_OK) {
      ui_show_printing(&state);
    }
    return;
  }
  if (page == 27 && component == 2) {
    ui_show_printing(&state);
    return;
  }
  if (page == 74 && component == 0) {
    return;
  }
  if (page == 77 && component == 5) {
    char file_path[FILE_ENTRY_PATH_MAX];
    ui_state_lock(&state);
    snprintf(file_path, sizeof(file_path), "%s", state.selected_file_path);
    ui_state_unlock(&state);
    app_state_acknowledge_print_result();
    moonraker_start_print(file_path);
    return;
  }
  if (page == 77 && component == 6) {
    app_state_acknowledge_print_result();
    ui_show_home(&state);
    if (moonraker_clear_print() != ESP_OK) {
      ESP_LOGE(TAG, "unable to queue completed print reset");
    }
    return;
  }

  ESP_LOGW(TAG, "unsupported touch page=%u component=%u", page, component);
}

static void handle_numeric(uint8_t page, uint8_t component, uint16_t value) {
  ESP_LOGI(TAG, "numeric page=%u component=%u value=%u", page, component,
           value);
  if (page == 6 && component <= 2) {
    ui_state_lock(&state);
    state.fan_values[component] = value > 100 ? 100 : value;
    int fan_value = state.fan_values[component];
    ui_state_unlock(&state);

    char command[32];
    snprintf(command, sizeof(command), "h%u.val=%d", component, fan_value);
    display_send_async(command, DISPLAY_CMD_HIGH);
    snprintf(command, sizeof(command), "n%u.val=%d", component, fan_value);
    display_send_async(command, DISPLAY_CMD_HIGH);
    if (moonraker_set_fan_speed(component, fan_value) != ESP_OK) {
      ESP_LOGE(TAG, "unable to queue fan %u command", component);
    }
  }
}

static void ui_event_task(void *argument) {
  (void)argument;
  display_event_t event;

  vTaskDelay(pdMS_TO_TICKS(1000));
  request_init_sequence();

  while (true) {
    if (display_receive_event(&event, portMAX_DELAY) != pdTRUE) {
      continue;
    }
    switch (event.type) {
    case DISPLAY_EVENT_TOUCH:
      handle_touch(event.page, event.component);
      break;
    case DISPLAY_EVENT_NUMERIC:
      handle_numeric(event.page, event.component, event.value);
      break;
    case DISPLAY_EVENT_TEXT:
      ESP_LOGI(TAG, "text: %s", event.text);
      break;
    case DISPLAY_EVENT_STATUS:
      ESP_LOGW(TAG, "display status 0x%02x", event.status);
      break;
    case DISPLAY_EVENT_INIT: {
      int64_t now = display_last_rx_ms();
      ESP_LOGI(TAG, "display init signal repeat=%u",
               (unsigned)event.raw_length);
      if (now - last_init_ms >= 5000) {
        last_init_ms = now;
        request_init_sequence();
      }
      break;
    }
    case DISPLAY_EVENT_UNKNOWN:
      ESP_LOGW(TAG, "unknown display frame length=%u",
               (unsigned)event.raw_length);
      break;
    }
  }
}

static void state_push_task(void *argument) {
  (void)argument;
  files_cache_snapshot_t *files_snapshot = malloc(sizeof(*files_snapshot));
  if (files_snapshot == NULL) {
    ESP_LOGE(TAG, "unable to allocate files cache monitor");
    vTaskDelete(NULL);
  }

  while (true) {
    app_state_snapshot_t snapshot;
    app_state_get(&snapshot);

    int wifi_bars = 0;
    if (snapshot.wifi_connected) {
      if (snapshot.wifi_rssi >= -55) {
        wifi_bars = 4;
      } else if (snapshot.wifi_rssi >= -67) {
        wifi_bars = 3;
      } else if (snapshot.wifi_rssi >= -75) {
        wifi_bars = 2;
      } else {
        wifi_bars = 1;
      }
    }

    ui_state_lock(&state);
    state.nozzle_current = snapshot.nozzle_current;
    state.nozzle_target = snapshot.nozzle_target;
    state.bed_current = snapshot.bed_current;
    state.bed_target = snapshot.bed_target;
    state.caselight_on = snapshot.caselight_on;
    for (size_t index = 0; index < 3; ++index) {
      state.fan_values[index] = snapshot.fan_values[index];
    }
    state.wifi_signal_bars = wifi_bars;
    state.print_progress = snapshot.print_progress;
    state.print_elapsed_seconds = snapshot.print_elapsed_seconds;
    state.print_estimated_seconds = snapshot.print_estimated_seconds;
    state.print_paused = snapshot.print_mode == APP_PRINT_PAUSED ||
                         snapshot.print_mode == APP_PRINT_RESUMING;
    if (snapshot.print_filename[0] != '\0') {
      const char *basename = strrchr(snapshot.print_filename, '/');
      basename = basename == NULL ? snapshot.print_filename : basename + 1;
      snprintf(state.selected_file_name, sizeof(state.selected_file_name),
               "%.*s", (int)sizeof(state.selected_file_name) - 1, basename);
      snprintf(state.selected_file_path, sizeof(state.selected_file_path),
               "%s", snapshot.print_filename);
    }
    app_connection_state_t previous_connection =
        state.backend_connection_state;
    app_print_state_t previous_print = state.backend_print_state;
    bool backend_was_initialized = state.backend_state_initialized;
    state.backend_connection_state = snapshot.connection_state;
    state.backend_print_state = snapshot.print_mode;
    state.backend_state_initialized = true;
    uint8_t current_page = state.page;
    bool new_command_result =
        snapshot.command_generation != 0 &&
        snapshot.command_generation != state.command_generation_handled &&
        snapshot.command_status != APP_COMMAND_PENDING;
    if (new_command_result) {
      state.command_generation_handled = snapshot.command_generation;
    }
    ui_state_unlock(&state);

    ui_update_home_temperatures(&state);
    ui_update_fans(&state);
    ui_update_printing(&state);

    if (new_command_result) {
      bool succeeded = snapshot.command_status == APP_COMMAND_SUCCEEDED;
      if (strcmp(snapshot.command_name, "print_start") == 0 && succeeded) {
        ui_show_printing(&state);
      } else if (strcmp(snapshot.command_name, "print_pause") == 0 &&
                 succeeded) {
        ui_show_printing(&state);
      } else if (strcmp(snapshot.command_name, "print_resume") == 0 &&
                 succeeded) {
        ui_show_printing(&state);
      } else if (!succeeded) {
        ESP_LOGE(TAG, "%s failed: %s", snapshot.command_name,
                 snapshot.command_message);
        ui_show_printing(&state);
      }
    }

    bool print_became_cancelled =
        snapshot.print_mode == APP_PRINT_CANCELLED &&
        (!backend_was_initialized ||
         previous_print != APP_PRINT_CANCELLED);
    bool print_became_complete =
        snapshot.print_mode == APP_PRINT_COMPLETE &&
        (!backend_was_initialized || previous_print != APP_PRINT_COMPLETE);
    bool external_print_stopped =
        snapshot.print_mode == APP_PRINT_IDLE && backend_was_initialized &&
        (previous_print == APP_PRINT_PRINTING ||
         previous_print == APP_PRINT_PAUSED ||
         previous_print == APP_PRINT_PAUSING ||
         previous_print == APP_PRINT_RESUMING);
    if (print_became_cancelled) {
      ui_show_print_result(&state, false);
    } else if (print_became_complete) {
      ui_show_print_result(&state, true);
    } else if (external_print_stopped) {
      ESP_LOGI(TAG, "external print stop, returning home");
      ui_show_home(&state);
    } else {
      bool connection_recovered =
          snapshot.connection_state == APP_CONNECTION_MOONRAKER_READY &&
          (!backend_was_initialized ||
           previous_connection != APP_CONNECTION_MOONRAKER_READY);
      bool active_print_discovered =
          (snapshot.print_mode == APP_PRINT_PRINTING ||
           snapshot.print_mode == APP_PRINT_PAUSED) &&
          (!backend_was_initialized ||
           (previous_print != APP_PRINT_PRINTING &&
            previous_print != APP_PRINT_PAUSED));
      bool should_recover_print_page =
          (connection_recovered || active_print_discovered) &&
          (snapshot.print_mode == APP_PRINT_PRINTING ||
           snapshot.print_mode == APP_PRINT_PAUSED) &&
          current_page != 2;
      if (should_recover_print_page) {
        ESP_LOGI(TAG, "recovering active print page");
        ui_show_printing(&state);
      }
    }

    files_cache_get(files_snapshot);
    ui_state_lock(&state);
    bool files_visible = state.page == 7;
    bool path_matches = strcmp(state.files_path, files_snapshot->path) == 0;
    bool generation_changed =
        state.files_generation_rendered != files_snapshot->generation;
    ui_state_unlock(&state);
    if (files_visible && path_matches && generation_changed) {
      ui_render_files(&state, files_snapshot);
    }

    vTaskDelay(pdMS_TO_TICKS(1000));
  }
}

esp_err_t ui_controller_start(void) {
  ui_state_init(&state);
  if (state.lock == NULL) {
    return ESP_ERR_NO_MEM;
  }
  if (xTaskCreate(ui_event_task, "ui_events", 6144, NULL, 9, NULL) != pdPASS) {
    return ESP_ERR_NO_MEM;
  }
  if (xTaskCreate(state_push_task, "ui_state", 3072, NULL, 5, NULL) != pdPASS) {
    return ESP_ERR_NO_MEM;
  }
  return ESP_OK;
}
