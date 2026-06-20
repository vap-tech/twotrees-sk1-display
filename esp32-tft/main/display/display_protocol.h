#pragma once

#include "display_events.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

bool display_protocol_decode(const uint8_t *payload, size_t length,
                             display_event_t *event);
