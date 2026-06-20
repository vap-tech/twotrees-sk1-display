#include "ui_state.h"

#include <stdio.h>
#include <string.h>

void ui_state_init(ui_state_t *state) {
  memset(state, 0, sizeof(*state));
  state->lock = xSemaphoreCreateMutex();
  state->nozzle_current = 25;
  state->bed_current = 30;
  snprintf(state->files_path, sizeof(state->files_path), "gcodes");
}

void ui_state_lock(ui_state_t *state) {
  xSemaphoreTake(state->lock, portMAX_DELAY);
}

void ui_state_unlock(ui_state_t *state) { xSemaphoreGive(state->lock); }
