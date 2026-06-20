#include "files_cache.h"

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include <stdio.h>
#include <string.h>

static SemaphoreHandle_t cache_mutex;
static files_cache_snapshot_t cache;

static void copy_text(char *destination, size_t size, const char *source) {
  snprintf(destination, size, "%s", source ? source : "");
}

void files_cache_init(void) {
  memset(&cache, 0, sizeof(cache));
  cache_mutex = xSemaphoreCreateMutex();
}

void files_cache_get(files_cache_snapshot_t *snapshot) {
  if (snapshot == NULL) {
    return;
  }
  xSemaphoreTake(cache_mutex, portMAX_DELAY);
  *snapshot = cache;
  xSemaphoreGive(cache_mutex);
}

void files_cache_set_loading(const char *path) {
  xSemaphoreTake(cache_mutex, portMAX_DELAY);
  cache.loading = true;
  cache.valid = false;
  cache.count = 0;
  copy_text(cache.path, sizeof(cache.path), path);
  ++cache.generation;
  xSemaphoreGive(cache_mutex);
}

void files_cache_replace(const char *path, const file_entry_t *entries,
                         size_t count) {
  if (count > FILES_CACHE_MAX_ENTRIES) {
    count = FILES_CACHE_MAX_ENTRIES;
  }
  xSemaphoreTake(cache_mutex, portMAX_DELAY);
  memset(cache.entries, 0, sizeof(cache.entries));
  if (entries != NULL && count > 0) {
    memcpy(cache.entries, entries, count * sizeof(*entries));
  }
  cache.count = count;
  cache.loading = false;
  cache.valid = true;
  copy_text(cache.path, sizeof(cache.path), path);
  ++cache.generation;
  xSemaphoreGive(cache_mutex);
}

void files_cache_set_error(const char *path) {
  xSemaphoreTake(cache_mutex, portMAX_DELAY);
  cache.loading = false;
  cache.valid = false;
  cache.count = 0;
  copy_text(cache.path, sizeof(cache.path), path);
  ++cache.generation;
  xSemaphoreGive(cache_mutex);
}
