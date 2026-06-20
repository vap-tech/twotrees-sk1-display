#include "app_state.h"

#include "esp_timer.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include <stdio.h>
#include <string.h>

static SemaphoreHandle_t state_mutex;
static app_state_snapshot_t state;
static bool print_result_acknowledged;
static const char *TAG = "app_state";

static const char *connection_state_name(app_connection_state_t value) {
  switch (value) {
  case APP_CONNECTION_BOOT:
    return "boot";
  case APP_CONNECTION_WIFI_CONNECTING:
    return "wifi_connecting";
  case APP_CONNECTION_WIFI_CONNECTED:
    return "wifi_connected";
  case APP_CONNECTION_MOONRAKER_CONNECTING:
    return "moonraker_connecting";
  case APP_CONNECTION_MOONRAKER_READY:
    return "moonraker_ready";
  case APP_CONNECTION_DISCONNECTED:
    return "disconnected";
  default:
    return "unknown";
  }
}

static const char *print_state_name(app_print_state_t value) {
  switch (value) {
  case APP_PRINT_IDLE:
    return "idle";
  case APP_PRINT_STARTING:
    return "starting";
  case APP_PRINT_PRINTING:
    return "printing";
  case APP_PRINT_PAUSING:
    return "pausing";
  case APP_PRINT_PAUSED:
    return "paused";
  case APP_PRINT_RESUMING:
    return "resuming";
  case APP_PRINT_CANCELLING:
    return "cancelling";
  case APP_PRINT_COMPLETE:
    return "complete";
  case APP_PRINT_CANCELLED:
    return "cancelled";
  case APP_PRINT_ERROR:
    return "error";
  default:
    return "unknown";
  }
}

static void transition_connection(app_connection_state_t next) {
  if (state.connection_state == next) {
    return;
  }
  ESP_LOGI(TAG, "connection %s -> %s",
           connection_state_name(state.connection_state),
           connection_state_name(next));
  state.connection_state = next;
}

static void transition_print(app_print_state_t next) {
  if (state.print_mode == next) {
    return;
  }
  ESP_LOGI(TAG, "print %s -> %s", print_state_name(state.print_mode),
           print_state_name(next));
  state.print_mode = next;
}

static void copy_text(char *destination, size_t size, const char *source) {
  snprintf(destination, size, "%s", source ? source : "");
}

static void lock_state(void) { xSemaphoreTake(state_mutex, portMAX_DELAY); }

static void finish_update(void) {
  state.updated_ms = esp_timer_get_time() / 1000;
  xSemaphoreGive(state_mutex);
}

void app_state_init(void) {
  memset(&state, 0, sizeof(state));
  state_mutex = xSemaphoreCreateMutex();
  state.connection_state = APP_CONNECTION_BOOT;
  state.print_mode = APP_PRINT_IDLE;
  copy_text(state.klippy_state, sizeof(state.klippy_state), "disconnected");
  copy_text(state.printer_state, sizeof(state.printer_state), "disconnected");
  copy_text(state.print_state, sizeof(state.print_state), "standby");
}

void app_state_get(app_state_snapshot_t *snapshot) {
  if (snapshot == NULL) {
    return;
  }
  lock_state();
  *snapshot = state;
  xSemaphoreGive(state_mutex);
}

void app_state_set_wifi_connecting(void) {
  lock_state();
  transition_connection(APP_CONNECTION_WIFI_CONNECTING);
  finish_update();
}

void app_state_set_wifi(bool connected, const char *ip_address, int rssi) {
  lock_state();
  state.wifi_connected = connected;
  state.wifi_rssi = rssi;
  copy_text(state.ip_address, sizeof(state.ip_address),
            connected ? ip_address : "");
  if (!connected) {
    transition_connection(APP_CONNECTION_DISCONNECTED);
  } else if (state.connection_state == APP_CONNECTION_BOOT ||
             state.connection_state == APP_CONNECTION_WIFI_CONNECTING ||
             state.connection_state == APP_CONNECTION_WIFI_CONNECTED) {
    transition_connection(APP_CONNECTION_WIFI_CONNECTED);
  }
  finish_update();
}

void app_state_set_moonraker_online(bool online) {
  lock_state();
  state.moonraker_online = online;
  if (!online) {
    copy_text(state.klippy_state, sizeof(state.klippy_state), "disconnected");
    copy_text(state.printer_state, sizeof(state.printer_state), "disconnected");
    transition_connection(state.wifi_connected
                              ? APP_CONNECTION_MOONRAKER_CONNECTING
                              : APP_CONNECTION_DISCONNECTED);
  }
  finish_update();
}

void app_state_set_server_info(const char *version, const char *klippy_state) {
  lock_state();
  state.moonraker_online = true;
  copy_text(state.moonraker_version, sizeof(state.moonraker_version), version);
  copy_text(state.klippy_state, sizeof(state.klippy_state), klippy_state);
  transition_connection(strcmp(klippy_state, "ready") == 0
                            ? APP_CONNECTION_MOONRAKER_READY
                            : APP_CONNECTION_MOONRAKER_CONNECTING);
  finish_update();
}

void app_state_set_printer_info(const char *printer_state) {
  lock_state();
  copy_text(state.printer_state, sizeof(state.printer_state), printer_state);
  finish_update();
}

static void update_print_mode(const char *print_state) {
  bool terminal = strcmp(print_state, "complete") == 0 ||
                  strcmp(print_state, "cancelled") == 0 ||
                  strcmp(print_state, "error") == 0;
  if (!terminal) {
    print_result_acknowledged = false;
  } else if (print_result_acknowledged) {
    transition_print(APP_PRINT_IDLE);
    return;
  }

  if (strcmp(print_state, "printing") == 0) {
    if (state.print_mode != APP_PRINT_PAUSING &&
        state.print_mode != APP_PRINT_CANCELLING) {
      transition_print(APP_PRINT_PRINTING);
    }
  } else if (strcmp(print_state, "paused") == 0) {
    if (state.print_mode != APP_PRINT_RESUMING &&
        state.print_mode != APP_PRINT_CANCELLING) {
      transition_print(APP_PRINT_PAUSED);
    }
  } else if (strcmp(print_state, "complete") == 0) {
    transition_print(APP_PRINT_COMPLETE);
  } else if (strcmp(print_state, "cancelled") == 0) {
    transition_print(APP_PRINT_CANCELLED);
  } else if (strcmp(print_state, "error") == 0) {
    transition_print(APP_PRINT_ERROR);
  } else if (state.print_mode == APP_PRINT_CANCELLING) {
    transition_print(APP_PRINT_CANCELLED);
  } else if (state.print_mode != APP_PRINT_COMPLETE &&
             state.print_mode != APP_PRINT_CANCELLED) {
    transition_print(APP_PRINT_IDLE);
  }
}

void app_state_apply_printer_update(const app_printer_update_t *update) {
  if (update == NULL) {
    return;
  }
  lock_state();
  if (update->has_nozzle_current) {
    state.nozzle_current = update->nozzle_current;
  }
  if (update->has_nozzle_target) {
    state.nozzle_target = update->nozzle_target;
  }
  if (update->has_bed_current) {
    state.bed_current = update->bed_current;
  }
  if (update->has_bed_target) {
    state.bed_target = update->bed_target;
  }
  if (update->has_caselight) {
    if (state.caselight_on != update->caselight_on) {
      ESP_LOGI(TAG, "caselight %s",
               update->caselight_on ? "on" : "off");
    }
    state.caselight_on = update->caselight_on;
  }
  for (size_t index = 0; index < 3; ++index) {
    if (update->has_fan_values[index]) {
      state.fan_values[index] = update->fan_values[index];
    }
  }
  if (update->has_print_progress) {
    state.print_progress = update->print_progress;
  }
  if (update->has_print_elapsed_seconds) {
    state.print_elapsed_seconds = update->print_elapsed_seconds;
  }
  if (update->has_print_estimated_seconds) {
    state.print_estimated_seconds = update->print_estimated_seconds;
  }
  if (update->has_print_filename && update->print_filename != NULL &&
      update->print_filename[0] != '\0') {
    copy_text(state.print_filename, sizeof(state.print_filename),
              update->print_filename);
  }
  if (update->has_print_state && update->print_state != NULL) {
    copy_text(state.print_state, sizeof(state.print_state),
              update->print_state);
    update_print_mode(update->print_state);
  }
  finish_update();
}

void app_state_set_printer_status(int nozzle_current, int nozzle_target,
                                  int bed_current, int bed_target,
                                  const char *print_state,
                                  const char *print_filename,
                                  int print_progress) {
  app_printer_update_t update = {
      .has_nozzle_current = true,
      .nozzle_current = nozzle_current,
      .has_nozzle_target = true,
      .nozzle_target = nozzle_target,
      .has_bed_current = true,
      .bed_current = bed_current,
      .has_bed_target = true,
      .bed_target = bed_target,
      .has_print_state = true,
      .print_state = print_state,
      .has_print_filename = print_filename != NULL &&
                            print_filename[0] != '\0',
      .print_filename = print_filename,
      .has_print_progress = true,
      .print_progress = print_progress,
  };
  app_state_apply_printer_update(&update);
}

uint32_t app_state_command_begin(const char *name) {
  lock_state();
  ++state.command_generation;
  if (state.command_generation == 0) {
    ++state.command_generation;
  }
  state.command_status = APP_COMMAND_PENDING;
  copy_text(state.command_name, sizeof(state.command_name), name);
  state.command_message[0] = '\0';
  if (strcmp(name, "print_start") == 0) {
    print_result_acknowledged = false;
    transition_print(APP_PRINT_STARTING);
  } else if (strcmp(name, "print_pause") == 0) {
    transition_print(APP_PRINT_PAUSING);
  } else if (strcmp(name, "print_resume") == 0) {
    transition_print(APP_PRINT_RESUMING);
  } else if (strcmp(name, "print_cancel") == 0) {
    transition_print(APP_PRINT_CANCELLING);
  }
  uint32_t generation = state.command_generation;
  finish_update();
  return generation;
}

void app_state_command_finish(uint32_t generation, bool succeeded,
                              const char *message) {
  lock_state();
  if (state.command_generation == generation) {
    state.command_status =
        succeeded ? APP_COMMAND_SUCCEEDED : APP_COMMAND_FAILED;
    copy_text(state.command_message, sizeof(state.command_message), message);
    if (!succeeded) {
      if (strcmp(state.command_name, "print_start") == 0) {
        transition_print(APP_PRINT_IDLE);
      } else if (strcmp(state.command_name, "print_pause") == 0 ||
                 strcmp(state.command_name, "print_cancel") == 0) {
        transition_print(APP_PRINT_PRINTING);
      } else if (strcmp(state.command_name, "print_resume") == 0) {
        transition_print(APP_PRINT_PAUSED);
      }
    }
  }
  finish_update();
}

void app_state_acknowledge_print_result(void) {
  lock_state();
  print_result_acknowledged = true;
  if (state.print_mode == APP_PRINT_COMPLETE ||
      state.print_mode == APP_PRINT_CANCELLED ||
      state.print_mode == APP_PRINT_ERROR) {
    transition_print(APP_PRINT_IDLE);
    state.print_filename[0] = '\0';
    state.print_progress = 0;
    state.print_elapsed_seconds = 0;
    state.print_estimated_seconds = 0;
  }
  finish_update();
}

void app_state_set_fan_value(size_t index, int value) {
  if (index >= 3) {
    return;
  }
  lock_state();
  state.fan_values[index] = value < 0 ? 0 : value > 100 ? 100 : value;
  finish_update();
}
