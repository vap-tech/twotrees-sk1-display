#pragma once

#include <stddef.h>
#include <stdint.h>

#define DISPLAY_EVENT_TEXT_MAX 96

typedef enum {
  DISPLAY_EVENT_TOUCH,
  DISPLAY_EVENT_NUMERIC,
  DISPLAY_EVENT_TEXT,
  DISPLAY_EVENT_STATUS,
  DISPLAY_EVENT_INIT,
  DISPLAY_EVENT_UNKNOWN,
} display_event_type_t;

typedef struct {
  display_event_type_t type;
  uint8_t page;
  uint8_t component;
  uint16_t value;
  uint8_t status;
  size_t raw_length;
  char text[DISPLAY_EVENT_TEXT_MAX];
} display_event_t;
