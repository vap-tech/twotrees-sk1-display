#pragma once

#include "esp_err.h"
#include <stdbool.h>
#include <stddef.h>

esp_err_t moonraker_client_start(void);
esp_err_t moonraker_request_files(const char *path);
esp_err_t moonraker_start_print(const char *filename);
esp_err_t moonraker_pause_print(void);
esp_err_t moonraker_resume_print(void);
esp_err_t moonraker_cancel_print(void);
esp_err_t moonraker_set_caselight(bool enabled);
esp_err_t moonraker_set_fan_speed(size_t index, int percent);
esp_err_t moonraker_clear_print(void);
