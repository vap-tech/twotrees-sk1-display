#pragma once

#include "files_cache.h"
#include "ui_state.h"

void ui_show_files_loading(ui_state_t *state);
void ui_render_files(ui_state_t *state, const files_cache_snapshot_t *snapshot);
void ui_show_file_preview(ui_state_t *state);
