#include "ui_pages_print.h"

#include "display_uart.h"
#include "thumbnail_worker.h"

#include <stdio.h>
#include <string.h>

static int last_elapsed_minute = -1;
static int last_remaining_minute = -1;

static void send_value(const char *component, int value) {
  char command[48];
  snprintf(command, sizeof(command), "%s.val=%d", component, value);
  display_send_async(command, DISPLAY_CMD_NORMAL);
}

static void send_text(const char *component, const char *text) {
  char escaped[FILE_ENTRY_NAME_MAX * 2];
  size_t output = 0;
  for (size_t index = 0; text[index] != '\0' && output + 2 < sizeof(escaped);
       ++index) {
    if (text[index] == '"' || text[index] == '\\') {
      escaped[output++] = '\\';
    }
    escaped[output++] = text[index];
  }
  escaped[output] = '\0';

  char command[sizeof(escaped) + 32];
  snprintf(command, sizeof(command), "%s.txt=\"%s\"", component, escaped);
  display_send_async(command, DISPLAY_CMD_NORMAL);
}

void ui_show_printing(ui_state_t *state) {
  char file_path[FILE_ENTRY_PATH_MAX];
  ui_state_lock(state);
  state->page = 2;
  char filename[FILE_ENTRY_NAME_MAX];
  snprintf(filename, sizeof(filename), "%s", state->selected_file_name);
  snprintf(file_path, sizeof(file_path), "%s", state->selected_file_path);
  ui_state_unlock(state);

  display_cancel_low_priority();
  last_elapsed_minute = -1;
  last_remaining_minute = -1;
  uint32_t generation = display_low_priority_generation();
  display_send_async("page 2", DISPLAY_CMD_HIGH);
  send_text("g0", filename);
  display_send_async("Print_Trun_1.cp0.close()", DISPLAY_CMD_NORMAL);
  display_send_async("vis cp0,0", DISPLAY_CMD_NORMAL);
  display_send_async("b5.picc=4", DISPLAY_CMD_NORMAL);
  display_send_async("b5.picc2=5", DISPLAY_CMD_NORMAL);
  ui_update_printing(state);
  if (file_path[0] != '\0') {
    thumbnail_request_print(file_path, generation);
  }
}

void ui_update_printing(ui_state_t *state) {
  int page;
  int nozzle_current;
  int nozzle_target;
  int bed_current;
  int bed_target;
  int progress;
  int elapsed_seconds;
  int estimated_seconds;
  int wifi_pic;
  int caselight_pic;
  int fan_pic;
  bool paused;

  ui_state_lock(state);
  page = state->page;
  nozzle_current = state->nozzle_current;
  nozzle_target = state->nozzle_target;
  bed_current = state->bed_current;
  bed_target = state->bed_target;
  progress = state->print_progress;
  elapsed_seconds = state->print_elapsed_seconds;
  estimated_seconds = state->print_estimated_seconds;
  wifi_pic = 67 + (state->wifi_signal_bars > 4 ? 4 : state->wifi_signal_bars);
  caselight_pic = state->caselight_on ? 3 : 2;
  fan_pic =
      (state->fan_values[0] || state->fan_values[1] || state->fan_values[2])
          ? 3
          : 2;
  paused = state->print_paused;
  ui_state_unlock(state);
  if (page != 2) {
    return;
  }

  char command[48];
  snprintf(command, sizeof(command), "Print_Trun_1.p0.pic=%d", wifi_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  send_value("n0", nozzle_current);
  send_value("n1", bed_current);
  send_value("n6", progress);
  int remaining_seconds = estimated_seconds - elapsed_seconds;
  if (remaining_seconds < 0) {
    remaining_seconds = 0;
  }
  int elapsed_minute = elapsed_seconds / 60;
  int remaining_minute = remaining_seconds / 60;
  if (elapsed_minute != last_elapsed_minute) {
    send_value("n4", elapsed_minute / 60);
    send_value("n5", elapsed_minute % 60);
    last_elapsed_minute = elapsed_minute;
  }
  if (remaining_minute != last_remaining_minute) {
    send_value("n7", remaining_minute / 60);
    send_value("n8", remaining_minute % 60);
    last_remaining_minute = remaining_minute;
  }
  snprintf(command, sizeof(command), "t8.txt=\"%d\"", nozzle_target);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "t9.txt=\"%d\"", bed_target);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b5.picc=%d", paused ? 5 : 4);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b5.picc2=%d", paused ? 4 : 5);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b6.picc=%d", caselight_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b6.picc2=%d", caselight_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b7.picc=%d", fan_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  snprintf(command, sizeof(command), "b7.picc2=%d", fan_pic);
  display_send_async(command, DISPLAY_CMD_NORMAL);
}

void ui_show_print_result(ui_state_t *state, bool completed) {
  char filename[FILE_ENTRY_NAME_MAX];
  char file_path[FILE_ENTRY_PATH_MAX];
  ui_state_lock(state);
  state->page = 77;
  snprintf(filename, sizeof(filename), "%s", state->selected_file_name);
  snprintf(file_path, sizeof(file_path), "%s", state->selected_file_path);
  ui_state_unlock(state);

  display_cancel_low_priority();
  uint32_t generation = display_low_priority_generation();
  display_send_async("page 77", DISPLAY_CMD_HIGH);
  display_send_async("print_done.tm0.en=0", DISPLAY_CMD_HIGH);
  display_send_async("print_done.cp0.close()", DISPLAY_CMD_NORMAL);
  display_send_async("vis print_done.cp0,0", DISPLAY_CMD_NORMAL);
  send_text("g0", filename);
  char command[48];
  snprintf(command, sizeof(command), "print_done_flag=%d", completed ? 1 : 0);
  display_send_async(command, DISPLAY_CMD_NORMAL);
  display_send_async("print_done.tm0.en=1", DISPLAY_CMD_NORMAL);
  if (file_path[0] != '\0') {
    thumbnail_request_result(file_path, generation);
  }
}
