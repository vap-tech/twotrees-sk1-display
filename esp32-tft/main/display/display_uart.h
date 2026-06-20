#pragma once

#include "display_events.h"

#include "esp_err.h"
#include "freertos/FreeRTOS.h"

#include <stddef.h>
#include <stdint.h>

typedef enum {
  DISPLAY_CMD_HIGH,
  DISPLAY_CMD_NORMAL,
  DISPLAY_CMD_LOW,
} display_cmd_priority_t;

esp_err_t display_uart_init(void);

esp_err_t display_send_async(const char *command,
                             display_cmd_priority_t priority);
esp_err_t display_send_bytes_async(const uint8_t *payload, size_t length,
                                   display_cmd_priority_t priority);

void display_cancel_low_priority(void);
uint32_t display_low_priority_generation(void);
esp_err_t display_send_low_if_current(const uint8_t *payload, size_t length,
                                      uint32_t generation);

BaseType_t display_receive_event(display_event_t *event, TickType_t timeout);
int64_t display_last_rx_ms(void);
int64_t display_last_tx_ms(void);
