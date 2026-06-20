#pragma once

#include "esp_err.h"
#include "freertos/FreeRTOS.h"

esp_err_t wifi_manager_start(void);
BaseType_t wifi_manager_wait_connected(TickType_t timeout);
BaseType_t wifi_manager_wait_disconnected(TickType_t timeout);
