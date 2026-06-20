#include "ui_pages_files.h"

#include "display_uart.h"
#include "thumbnail_worker.h"

#include <stdio.h>
#include <string.h>

typedef struct {
  const char *name;
  const char *image;
  const char *button;
  const char *time;
  const char *number_a;
  const char *number_b;
  const char *hidden[6];
} file_slot_components_t;

static const file_slot_components_t SLOTS[3] = {
    {
        .name = "t12",
        .image = "q0",
        .button = "b12",
        .time = "t2",
        .number_a = "n0",
        .number_b = "n1",
        .hidden = {"t3", "t0", "t1", "t2", "n0", "n1"},
    },
    {
        .name = "t13",
        .image = "q1",
        .button = "b13",
        .time = "t6",
        .number_a = "n2",
        .number_b = "n3",
        .hidden = {"t7", "t4", "t5", "t6", "n2", "n3"},
    },
    {
        .name = "t14",
        .image = "q2",
        .button = "b14",
        .time = "t10",
        .number_a = "n4",
        .number_b = "n5",
        .hidden = {"t11", "t8", "t9", "t10", "n4", "n5"},
    },
};

static void send_command(const char *format, const char *component, int value) {
  char command[96];
  snprintf(command, sizeof(command), format, component, value);
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

static void shorten_name(const char *name, char *output, size_t size) {
  const size_t limit = 22;
  if (strlen(name) <= limit) {
    snprintf(output, size, "%s", name);
    return;
  }
  snprintf(output, size, "%.*s...", (int)(limit - 3), name);
}

static void clear_slot(size_t slot_index) {
  const file_slot_components_t *slot = &SLOTS[slot_index];
  for (size_t index = 0; index < 6; ++index) {
    send_command("vis %s,%d", slot->hidden[index], 0);
  }
  send_text(slot->name, "");
  send_command("vis %s,%d", slot->name, 0);
  send_command("%s.picc=%d", slot->image, 100);
  send_command("%s.picc2=%d", slot->image, 100);
  send_command("%s.picc=%d", slot->button, 100);
}

static void render_entry(size_t slot_index, const file_entry_t *entry) {
  const file_slot_components_t *slot = &SLOTS[slot_index];
  clear_slot(slot_index);

  char visible_name[32];
  shorten_name(entry->name, visible_name, sizeof(visible_name));
  send_text(slot->name, visible_name);
  send_command("vis %s,%d", slot->name, 1);

  if (entry->kind == FILE_KIND_DIR) {
    send_command("%s.picc=%d", slot->image, 99);
    send_command("%s.picc2=%d", slot->image, 100);
    return;
  }

  send_command("%s.picc=%d", slot->image, 98);
  send_command("%s.picc2=%d", slot->image, 99);
  send_command("%s.picc=%d", slot->button, 18);
  send_command("vis %s,%d", slot->time, 1);
  send_command("vis %s,%d", slot->number_a, 1);
  send_command("vis %s,%d", slot->number_b, 1);
  send_text(slot->time, "0.000");
  send_command("%s.val=%d", slot->number_a, 0);
  send_command("%s.val=%d", slot->number_b, 0);
}

void ui_show_files_loading(ui_state_t *state) {
  display_cancel_low_priority();
  display_send_async("page 7", DISPLAY_CMD_HIGH);
  display_send_async("Local_Files.cp0.close()", DISPLAY_CMD_NORMAL);
  display_send_async("Local_Files.cp1.close()", DISPLAY_CMD_NORMAL);
  display_send_async("Local_Files.cp2.close()", DISPLAY_CMD_NORMAL);
  display_send_async("vis cp0,0", DISPLAY_CMD_NORMAL);
  display_send_async("vis cp1,0", DISPLAY_CMD_NORMAL);
  display_send_async("vis cp2,0", DISPLAY_CMD_NORMAL);
  for (size_t index = 0; index < 3; ++index) {
    clear_slot(index);
  }
  send_text("t15", "Loading...");

  ui_state_lock(state);
  state->page = 7;
  ui_state_unlock(state);
}

void ui_render_files(ui_state_t *state,
                     const files_cache_snapshot_t *snapshot) {
  size_t page;
  ui_state_lock(state);
  page = state->files_page;
  state->files_generation_rendered = snapshot->generation;
  ui_state_unlock(state);

  send_text("t15", snapshot->valid ? "" : "Load failed");
  for (size_t slot = 0; slot < 3; ++slot) {
    size_t entry_index = page * 3 + slot;
    if (snapshot->valid && entry_index < snapshot->count) {
      render_entry(slot, &snapshot->entries[entry_index]);
    } else {
      clear_slot(slot);
    }
  }
}

void ui_show_file_preview(ui_state_t *state) {
  char filename[FILE_ENTRY_NAME_MAX];
  char file_path[FILE_ENTRY_PATH_MAX];
  ui_state_lock(state);
  state->page = 9;
  snprintf(filename, sizeof(filename), "%s", state->selected_file_name);
  snprintf(file_path, sizeof(file_path), "%s", state->selected_file_path);
  ui_state_unlock(state);

  display_cancel_low_priority();
  uint32_t generation = display_low_priority_generation();
  display_send_async("page 9", DISPLAY_CMD_HIGH);
  send_text("g0", filename);
  display_send_async("n4.val=0", DISPLAY_CMD_NORMAL);
  display_send_async("n5.val=0", DISPLAY_CMD_NORMAL);
  display_send_async("t2.txt=\"0\"", DISPLAY_CMD_NORMAL);
  display_send_async("preview.cp0.close()", DISPLAY_CMD_NORMAL);
  display_send_async("vis cp0,0", DISPLAY_CMD_NORMAL);
  thumbnail_request_preview(file_path, generation);
}
