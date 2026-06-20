#include "display_protocol.h"

#include <string.h>

bool display_protocol_decode(const uint8_t *payload, size_t length,
                             display_event_t *event) {
  if (payload == NULL || event == NULL || length == 0) {
    return false;
  }

  memset(event, 0, sizeof(*event));
  event->raw_length = length;

  if (payload[0] == 0x65 && length >= 3) {
    event->type = DISPLAY_EVENT_TOUCH;
    event->page = payload[1];
    event->component = payload[2];
    return true;
  }

  if (payload[0] == 0x71 && length >= 5) {
    event->type = DISPLAY_EVENT_NUMERIC;
    event->page = payload[1];
    event->component = payload[2];
    event->value = payload[3] | ((uint16_t)payload[4] << 8);
    return true;
  }

  if (payload[0] == 0x70) {
    event->type = DISPLAY_EVENT_TEXT;
    size_t text_length = length - 1;
    if (text_length >= sizeof(event->text)) {
      text_length = sizeof(event->text) - 1;
    }
    memcpy(event->text, payload + 1, text_length);
    event->text[text_length] = '\0';
    return true;
  }

  if (length == 1 && (payload[0] == 0x1a || payload[0] == 0x1c)) {
    event->type = DISPLAY_EVENT_STATUS;
    event->status = payload[0];
    return true;
  }

  event->type = DISPLAY_EVENT_UNKNOWN;
  return true;
}
