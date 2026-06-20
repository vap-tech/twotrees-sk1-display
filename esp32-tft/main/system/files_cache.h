#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define FILES_CACHE_MAX_ENTRIES 64
#define FILE_ENTRY_NAME_MAX 96
#define FILE_ENTRY_PATH_MAX 192

typedef enum {
  FILE_KIND_FILE,
  FILE_KIND_DIR,
} file_kind_t;

typedef struct {
  char name[FILE_ENTRY_NAME_MAX];
  char path[FILE_ENTRY_PATH_MAX];
  file_kind_t kind;
  int64_t modified;
  size_t size;
} file_entry_t;

typedef struct {
  file_entry_t entries[FILES_CACHE_MAX_ENTRIES];
  size_t count;
  char path[FILE_ENTRY_PATH_MAX];
  bool valid;
  bool loading;
  uint32_t generation;
} files_cache_snapshot_t;

void files_cache_init(void);
void files_cache_get(files_cache_snapshot_t *snapshot);
void files_cache_set_loading(const char *path);
void files_cache_replace(const char *path, const file_entry_t *entries,
                         size_t count);
void files_cache_set_error(const char *path);
