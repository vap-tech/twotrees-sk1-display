#include "ui_pages_home.h"

#include "display_uart.h"

#include <stdio.h>

static int last_fan_values[3] = {-1, -1, -1};

static void send_value(const char *component, int value) {
  char command[48];
  snprintf(command, sizeof(command), "%s.val=%d", component, value);
  display_send_async(command, DISPLAY_CMD_NORMAL);
}

void ui_show_home(ui_state_t *state) {
  int nozzle_current;
  int bed_current;
  int nozzle_target;
  int bed_target;
  int caselight_pic;
  int fan_pic;
  int wifi_pic;

  ui_state_lock(state);
  state->page = 0;
  nozzle_current = state->nozzle_current;
  bed_current = state->bed_current;
  nozzle_target = state->nozzle_target;
  bed_target = state->bed_target;
  caselight_pic = state->caselight_on ? 3 : 2;
  fan_pic =
      (state->fan_values[0] || state->fan_values[1] || state->fan_values[2])
          ? 3
          : 2;
  wifi_pic = 67 + (state->wifi_signal_bars > 4 ? 4 : state->wifi_signal_bars);
  ui_state_unlock(state);

  display_cancel_low_priority();
  display_send_async("page 0", DISPLAY_CMD_HIGH);

  char command[48];
  snprintf(command, sizeof(command), "Start.p0.pic=%d", wifi_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  send_value("n0", nozzle_current);
  send_value("n1", bed_current);
  send_value("n4", nozzle_target);
  send_value("n5", bed_target);
  snprintf(command, sizeof(command), "b6.picc=%d", fan_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b6.picc2=%d", fan_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b5.picc=%d", caselight_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b5.picc2=%d", caselight_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
}

void ui_update_home_temperatures(ui_state_t *state) {
  int page;
  int nozzle_current;
  int bed_current;
  int nozzle_target;
  int bed_target;
  int caselight_pic;
  int wifi_pic;
  int fan_pic;

  ui_state_lock(state);
  page = state->page;
  nozzle_current = state->nozzle_current;
  bed_current = state->bed_current;
  nozzle_target = state->nozzle_target;
  bed_target = state->bed_target;
  caselight_pic = state->caselight_on ? 3 : 2;
  wifi_pic = 67 + (state->wifi_signal_bars > 4 ? 4 : state->wifi_signal_bars);
  fan_pic =
      (state->fan_values[0] || state->fan_values[1] || state->fan_values[2])
          ? 3
          : 2;
  ui_state_unlock(state);

  if (page != 0) {
    return;
  }
  send_value("n0", nozzle_current);
  send_value("n1", bed_current);
  send_value("n4", nozzle_target);
  send_value("n5", bed_target);

  char command[48];
  snprintf(command, sizeof(command), "Start.p0.pic=%d", wifi_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b5.picc=%d", caselight_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b5.picc2=%d", caselight_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b6.picc=%d", fan_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b6.picc2=%d", fan_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
}

void ui_show_fans(ui_state_t *state) {
  ui_state_lock(state);
  state->page = 6;
  ui_state_unlock(state);

  display_cancel_low_priority();
  display_send_async("page 6", DISPLAY_CMD_HIGH);
  for (size_t index = 0; index < 3; ++index) {
    last_fan_values[index] = -1;
  }
  ui_update_fans(state);
}

void ui_update_fans(ui_state_t *state) {
  int page;
  int values[3];
  ui_state_lock(state);
  page = state->page;
  for (size_t index = 0; index < 3; ++index) {
    values[index] = state->fan_values[index];
  }
  ui_state_unlock(state);
  if (page != 6) {
    return;
  }

  char command[32];
  for (size_t index = 0; index < 3; ++index) {
    if (values[index] == last_fan_values[index]) {
      continue;
    }
    snprintf(command, sizeof(command), "h%u.val=%d", (unsigned)index,
             values[index]);
    display_send_async(command, DISPLAY_CMD_NORMAL);
    snprintf(command, sizeof(command), "n%u.val=%d", (unsigned)index,
             values[index]);
    display_send_async(command, DISPLAY_CMD_NORMAL);
    last_fan_values[index] = values[index];
  }
}
