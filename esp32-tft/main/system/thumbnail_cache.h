#pragma once

#include "thumbnail_worker.h"

#include <stdbool.h>

void thumbnail_cache_init(void);
void thumbnail_cache_put(const char *file_path, const thumbnail_info_t *info);
bool thumbnail_cache_get(const char *file_path, thumbnail_info_t *info);

