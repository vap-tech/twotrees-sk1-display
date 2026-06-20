#pragma once

#include "esp_err.h"

#include <stddef.h>
#include <stdint.h>

typedef struct {
  char relative_path[192];
  uint16_t width;
  uint16_t height;
  size_t size;
} thumbnail_info_t;

esp_err_t thumbnail_worker_start(void);
esp_err_t thumbnail_request_preview(const char *file_path,
                                    uint32_t display_generation);
esp_err_t thumbnail_request_print(const char *file_path,
                                  uint32_t display_generation);
esp_err_t thumbnail_request_result(const char *file_path,
                                   uint32_t display_generation);
