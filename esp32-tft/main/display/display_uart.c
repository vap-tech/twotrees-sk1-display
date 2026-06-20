#include "display_uart.h"

#include "display_protocol.h"

#include "driver/uart.h"
#include "esp_check.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/queue.h"
#include "freertos/task.h"

#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#define DISPLAY_UART_PORT UART_NUM_1
#define DISPLAY_UART_TX_PIN 10
#define DISPLAY_UART_RX_PIN 11
#define DISPLAY_UART_BAUD 115200

#define DISPLAY_RX_BUFFER_SIZE 2048
#define DISPLAY_RX_FRAME_MAX 1536
#define DISPLAY_TX_PAYLOAD_MAX 1536
#define DISPLAY_TX_HIGH_QUEUE_DEPTH 32
#define DISPLAY_TX_NORMAL_QUEUE_DEPTH 128
#define DISPLAY_TX_LOW_QUEUE_DEPTH 64
#define DISPLAY_EVENT_QUEUE_DEPTH 32
#define DISPLAY_FRAME_END_LENGTH 3

typedef struct {
  uint8_t *payload;
  size_t length;
  uint32_t low_generation;
} display_tx_command_t;

static const char *TAG = "display_uart";
static const uint8_t FRAME_END[DISPLAY_FRAME_END_LENGTH] = {0xff, 0xff, 0xff};

static QueueHandle_t tx_high_queue;
static QueueHandle_t tx_normal_queue;
static QueueHandle_t tx_low_queue;
static QueueHandle_t event_queue;
static portMUX_TYPE state_lock = portMUX_INITIALIZER_UNLOCKED;
static uint32_t low_generation = 1;
static int64_t last_rx_ms;
static int64_t last_tx_ms;

static int64_t now_ms(void) { return esp_timer_get_time() / 1000; }

static QueueHandle_t queue_for_priority(display_cmd_priority_t priority) {
  switch (priority) {
  case DISPLAY_CMD_HIGH:
    return tx_high_queue;
  case DISPLAY_CMD_NORMAL:
    return tx_normal_queue;
  case DISPLAY_CMD_LOW:
    return tx_low_queue;
  default:
    return NULL;
  }
}

static void free_command(display_tx_command_t *command) {
  free(command->payload);
  memset(command, 0, sizeof(*command));
}

static bool receive_next_command(display_tx_command_t *command) {
  if (xQueueReceive(tx_high_queue, command, 0) == pdTRUE) {
    return true;
  }
  if (xQueueReceive(tx_normal_queue, command, 0) == pdTRUE) {
    return true;
  }
  if (xQueueReceive(tx_low_queue, command, 0) == pdTRUE) {
    return true;
  }
  return xQueueReceive(tx_high_queue, command, pdMS_TO_TICKS(20)) == pdTRUE;
}

static void uart_tx_task(void *argument) {
  (void)argument;
  display_tx_command_t command;

  while (true) {
    if (!receive_next_command(&command)) {
      continue;
    }

    uint32_t current_generation = display_low_priority_generation();
    if (command.low_generation != 0 &&
        command.low_generation != current_generation) {
      free_command(&command);
      continue;
    }

    int written =
        uart_write_bytes(DISPLAY_UART_PORT, command.payload, command.length);
    if (written == (int)command.length) {
      uart_write_bytes(DISPLAY_UART_PORT, FRAME_END, sizeof(FRAME_END));
      uart_wait_tx_done(DISPLAY_UART_PORT, pdMS_TO_TICKS(250));
      portENTER_CRITICAL(&state_lock);
      last_tx_ms = now_ms();
      portEXIT_CRITICAL(&state_lock);
      ESP_LOGD(TAG, "TX %.*s", (int)command.length, (char *)command.payload);
    } else {
      ESP_LOGE(TAG, "UART write failed: %d/%u", written,
               (unsigned)command.length);
    }
    free_command(&command);
  }
}

static bool frame_is_init_signal(const uint8_t *frame, size_t length) {
  if (length < 8) {
    return false;
  }
  for (size_t index = 0; index < length; ++index) {
    if (frame[index] != 0x91) {
      return false;
    }
  }
  return true;
}

static bool bytes_are_init_signal(const uint8_t *data, size_t length) {
  if (length == 0) {
    return false;
  }
  for (size_t index = 0; index < length; ++index) {
    if (data[index] != 0x91) {
      return false;
    }
  }
  return true;
}

static void publish_event(const uint8_t *payload, size_t length) {
  display_event_t event;
  if (!display_protocol_decode(payload, length, &event)) {
    return;
  }
  if (xQueueSend(event_queue, &event, 0) != pdTRUE) {
    ESP_LOGW(TAG, "event queue full, dropping type=%d", event.type);
  }
}

static void publish_init_event(size_t repeat_count) {
  display_event_t event = {
      .type = DISPLAY_EVENT_INIT,
      .raw_length = repeat_count,
  };
  if (xQueueSend(event_queue, &event, 0) != pdTRUE) {
    ESP_LOGW(TAG, "event queue full, dropping display init");
  }
}

static void consume_complete_frames(uint8_t *buffer, size_t *length) {
  size_t frame_start = 0;
  size_t index = 0;

  while (index + 2 < *length) {
    if (buffer[index] == 0xff && buffer[index + 1] == 0xff &&
        buffer[index + 2] == 0xff) {
      publish_event(buffer + frame_start, index - frame_start);
      index += DISPLAY_FRAME_END_LENGTH;
      frame_start = index;
    } else {
      ++index;
    }
  }

  if (frame_start > 0) {
    memmove(buffer, buffer + frame_start, *length - frame_start);
    *length -= frame_start;
  }
}

static void uart_rx_task(void *argument) {
  (void)argument;
  uint8_t chunk[256];
  uint8_t frame_buffer[DISPLAY_RX_FRAME_MAX];
  size_t frame_length = 0;
  size_t init_repeat_count = 0;
  int64_t partial_since_ms = 0;

  while (true) {
    int received = uart_read_bytes(DISPLAY_UART_PORT, chunk, sizeof(chunk),
                                   pdMS_TO_TICKS(50));
    if (received > 0) {
      portENTER_CRITICAL(&state_lock);
      last_rx_ms = now_ms();
      portEXIT_CRITICAL(&state_lock);

      if (frame_length == 0 && bytes_are_init_signal(chunk, received)) {
        init_repeat_count += received;
        partial_since_ms = now_ms();
        continue;
      }

      if (init_repeat_count > 0) {
        publish_init_event(init_repeat_count);
        init_repeat_count = 0;
      }

      if (frame_length + (size_t)received > sizeof(frame_buffer)) {
        ESP_LOGW(TAG, "RX frame overflow, dropping %u bytes",
                 (unsigned)frame_length);
        frame_length = 0;
      }
      memcpy(frame_buffer + frame_length, chunk, received);
      frame_length += received;
      partial_since_ms = now_ms();
      consume_complete_frames(frame_buffer, &frame_length);
      continue;
    }

    if (init_repeat_count > 0 && partial_since_ms > 0 &&
        now_ms() - partial_since_ms >= 300) {
      publish_init_event(init_repeat_count);
      init_repeat_count = 0;
      partial_since_ms = 0;
      continue;
    }

    if (frame_length > 0 && partial_since_ms > 0 &&
        now_ms() - partial_since_ms >= 300) {
      if (frame_is_init_signal(frame_buffer, frame_length)) {
        publish_init_event(frame_length);
      } else {
        ESP_LOG_BUFFER_HEX_LEVEL(TAG, frame_buffer, frame_length, ESP_LOG_WARN);
      }
      frame_length = 0;
      partial_since_ms = 0;
    }
  }
}

esp_err_t display_uart_init(void) {
  tx_high_queue =
      xQueueCreate(DISPLAY_TX_HIGH_QUEUE_DEPTH, sizeof(display_tx_command_t));
  tx_normal_queue =
      xQueueCreate(DISPLAY_TX_NORMAL_QUEUE_DEPTH, sizeof(display_tx_command_t));
  tx_low_queue =
      xQueueCreate(DISPLAY_TX_LOW_QUEUE_DEPTH, sizeof(display_tx_command_t));
  event_queue =
      xQueueCreate(DISPLAY_EVENT_QUEUE_DEPTH, sizeof(display_event_t));
  if (!tx_high_queue || !tx_normal_queue || !tx_low_queue || !event_queue) {
    return ESP_ERR_NO_MEM;
  }

  uart_config_t config = {
      .baud_rate = DISPLAY_UART_BAUD,
      .data_bits = UART_DATA_8_BITS,
      .parity = UART_PARITY_DISABLE,
      .stop_bits = UART_STOP_BITS_1,
      .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
      .source_clk = UART_SCLK_DEFAULT,
  };

  ESP_RETURN_ON_ERROR(uart_driver_install(DISPLAY_UART_PORT,
                                          DISPLAY_RX_BUFFER_SIZE, 0, 0, NULL,
                                          0),
                      TAG, "install UART");
  ESP_RETURN_ON_ERROR(uart_param_config(DISPLAY_UART_PORT, &config), TAG,
                      "configure UART");
  ESP_RETURN_ON_ERROR(uart_set_pin(DISPLAY_UART_PORT, DISPLAY_UART_TX_PIN,
                                   DISPLAY_UART_RX_PIN, UART_PIN_NO_CHANGE,
                                   UART_PIN_NO_CHANGE),
                      TAG, "set UART pins");

  last_rx_ms = now_ms();
  last_tx_ms = now_ms();
  xTaskCreate(uart_rx_task, "display_rx", 4096, NULL, 12, NULL);
  xTaskCreate(uart_tx_task, "display_tx", 4096, NULL, 10, NULL);
  return ESP_OK;
}

static esp_err_t enqueue_command(const uint8_t *payload, size_t length,
                                 display_cmd_priority_t priority,
                                 uint32_t generation) {
  if (payload == NULL || length == 0 || length > DISPLAY_TX_PAYLOAD_MAX) {
    return ESP_ERR_INVALID_ARG;
  }

  QueueHandle_t queue = queue_for_priority(priority);
  if (queue == NULL) {
    return ESP_ERR_INVALID_ARG;
  }

  display_tx_command_t command = {
      .payload = malloc(length),
      .length = length,
      .low_generation = priority == DISPLAY_CMD_LOW ? generation : 0,
  };
  if (command.payload == NULL) {
    return ESP_ERR_NO_MEM;
  }
  memcpy(command.payload, payload, length);

  if (xQueueSend(queue, &command, 0) != pdTRUE) {
    free_command(&command);
    return ESP_ERR_NO_MEM;
  }
  return ESP_OK;
}

esp_err_t display_send_bytes_async(const uint8_t *payload, size_t length,
                                   display_cmd_priority_t priority) {
  uint32_t generation =
      priority == DISPLAY_CMD_LOW ? display_low_priority_generation() : 0;
  return enqueue_command(payload, length, priority, generation);
}

esp_err_t display_send_async(const char *command,
                             display_cmd_priority_t priority) {
  if (command == NULL) {
    return ESP_ERR_INVALID_ARG;
  }
  return display_send_bytes_async((const uint8_t *)command, strlen(command),
                                  priority);
}

void display_cancel_low_priority(void) {
  display_tx_command_t command;
  portENTER_CRITICAL(&state_lock);
  ++low_generation;
  if (low_generation == 0) {
    ++low_generation;
  }
  portEXIT_CRITICAL(&state_lock);

  while (xQueueReceive(tx_low_queue, &command, 0) == pdTRUE) {
    free_command(&command);
  }
}

uint32_t display_low_priority_generation(void) {
  uint32_t generation;
  portENTER_CRITICAL(&state_lock);
  generation = low_generation;
  portEXIT_CRITICAL(&state_lock);
  return generation;
}

esp_err_t display_send_low_if_current(const uint8_t *payload, size_t length,
                                      uint32_t generation) {
  if (generation != display_low_priority_generation()) {
    return ESP_ERR_INVALID_STATE;
  }
  return enqueue_command(payload, length, DISPLAY_CMD_LOW, generation);
}

BaseType_t display_receive_event(display_event_t *event, TickType_t timeout) {
  return xQueueReceive(event_queue, event, timeout);
}

int64_t display_last_rx_ms(void) {
  int64_t value;
  portENTER_CRITICAL(&state_lock);
  value = last_rx_ms;
  portEXIT_CRITICAL(&state_lock);
  return value;
}

int64_t display_last_tx_ms(void) {
  int64_t value;
  portENTER_CRITICAL(&state_lock);
  value = last_tx_ms;
  portEXIT_CRITICAL(&state_lock);
  return value;
}
