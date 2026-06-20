#pragma once

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include <stdbool.h>
#include <stdint.h>

#include "files_cache.h"
#include "app_state.h"

typedef struct {
  SemaphoreHandle_t lock;
  uint8_t page;
  int nozzle_current;
  int bed_current;
  int nozzle_target;
  int bed_target;
  int fan_values[3];
  bool caselight_on;
  uint8_t wifi_signal_bars;
  int print_progress;
  int print_elapsed_seconds;
  int print_estimated_seconds;
  bool print_paused;
  app_connection_state_t backend_connection_state;
  app_print_state_t backend_print_state;
  bool backend_state_initialized;
  uint32_t command_generation_handled;
  char files_path[FILE_ENTRY_PATH_MAX];
  size_t files_page;
  uint32_t files_generation_rendered;
  char selected_file_name[FILE_ENTRY_NAME_MAX];
  char selected_file_path[FILE_ENTRY_PATH_MAX];
} ui_state_t;

void ui_state_init(ui_state_t *state);
void ui_state_lock(ui_state_t *state);
void ui_state_unlock(ui_state_t *state);
