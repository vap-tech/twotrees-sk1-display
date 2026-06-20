#include "thumbnail_cache.h"

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include <stdio.h>
#include <string.h>

#define THUMBNAIL_METADATA_CACHE_SIZE 16
#define THUMBNAIL_CACHE_PATH_MAX 192

typedef struct {
  char file_path[THUMBNAIL_CACHE_PATH_MAX];
  thumbnail_info_t info;
  bool valid;
} thumbnail_cache_entry_t;

static SemaphoreHandle_t cache_lock;
static thumbnail_cache_entry_t entries[THUMBNAIL_METADATA_CACHE_SIZE];
static size_t next_entry;

void thumbnail_cache_init(void) { cache_lock = xSemaphoreCreateMutex(); }

void thumbnail_cache_put(const char *file_path, const thumbnail_info_t *info) {
  if (cache_lock == NULL || file_path == NULL || info == NULL) {
    return;
  }
  xSemaphoreTake(cache_lock, portMAX_DELAY);
  size_t target = next_entry;
  for (size_t index = 0; index < THUMBNAIL_METADATA_CACHE_SIZE; ++index) {
    if (entries[index].valid &&
        strcmp(entries[index].file_path, file_path) == 0) {
      target = index;
      break;
    }
  }
  snprintf(entries[target].file_path, sizeof(entries[target].file_path), "%s",
           file_path);
  entries[target].info = *info;
  entries[target].valid = true;
  if (target == next_entry) {
    next_entry = (next_entry + 1) % THUMBNAIL_METADATA_CACHE_SIZE;
  }
  xSemaphoreGive(cache_lock);
}

bool thumbnail_cache_get(const char *file_path, thumbnail_info_t *info) {
  if (cache_lock == NULL || file_path == NULL || info == NULL) {
    return false;
  }
  bool found = false;
  xSemaphoreTake(cache_lock, portMAX_DELAY);
  for (size_t index = 0; index < THUMBNAIL_METADATA_CACHE_SIZE; ++index) {
    if (entries[index].valid &&
        strcmp(entries[index].file_path, file_path) == 0) {
      *info = entries[index].info;
      found = true;
      break;
    }
  }
  xSemaphoreGive(cache_lock);
  return found;
}

