#include "thumbnail_worker.h"

#include "display_uart.h"
#include "thumbnail_cache.h"
#include "wifi_manager.h"

#include "cJSON.h"
#include "esp_http_client.h"
#include "esp_log.h"
#include "freertos/queue.h"
#include "freertos/task.h"
#include "png.h"
#include "sdkconfig.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define PREVIEW_SIZE 155
#define THUMBNAIL_PATH_MAX 256
#define HTTP_BODY_MAX (128 * 1024)
#define COLPIC_PALETTE_SIZE 1024
#define COLPIC_CHUNK_SIZE 1024

typedef struct {
  enum {
    THUMBNAIL_TARGET_PREVIEW,
    THUMBNAIL_TARGET_PRINT,
    THUMBNAIL_TARGET_RESULT,
  } target;
  char file_path[THUMBNAIL_PATH_MAX];
  uint32_t generation;
} thumbnail_request_t;

typedef struct {
  uint8_t *data;
  size_t length;
  size_t capacity;
  bool overflow;
  uint32_t generation;
} http_body_t;

typedef struct {
  const uint8_t *data;
  size_t length;
  size_t offset;
} png_memory_reader_t;

typedef struct {
  uint16_t color;
  uint32_t count;
  uint16_t quantized;
} palette_entry_t;

static const char *TAG = "thumbnail";
static QueueHandle_t request_queue;

static bool request_current(const thumbnail_request_t *request) {
  return request->generation == display_low_priority_generation();
}

static void url_encode_path(const char *source, char *destination,
                            size_t size) {
  static const char hex[] = "0123456789ABCDEF";
  size_t output = 0;
  while (*source != '\0' && output + 1 < size) {
    unsigned char value = (unsigned char)*source++;
    bool safe = (value >= 'a' && value <= 'z') ||
                (value >= 'A' && value <= 'Z') ||
                (value >= '0' && value <= '9') || value == '-' ||
                value == '_' || value == '.' || value == '/';
    if (safe) {
      destination[output++] = (char)value;
    } else if (output + 3 < size) {
      destination[output++] = '%';
      destination[output++] = hex[value >> 4];
      destination[output++] = hex[value & 0x0f];
    } else {
      break;
    }
  }
  destination[output] = '\0';
}

static esp_err_t http_event_handler(esp_http_client_event_t *event) {
  http_body_t *body = event->user_data;
  if (event->event_id != HTTP_EVENT_ON_DATA || body == NULL ||
      event->data_len <= 0) {
    return ESP_OK;
  }
  if (body->generation != 0 &&
      body->generation != display_low_priority_generation()) {
    return ESP_ERR_INVALID_STATE;
  }

  size_t required = body->length + (size_t)event->data_len + 1;
  if (required > HTTP_BODY_MAX) {
    body->overflow = true;
    return ESP_FAIL;
  }
  if (required > body->capacity) {
    size_t capacity = body->capacity == 0 ? 4096 : body->capacity;
    while (capacity < required) {
      capacity *= 2;
    }
    if (capacity > HTTP_BODY_MAX) {
      capacity = HTTP_BODY_MAX;
    }
    uint8_t *data = realloc(body->data, capacity);
    if (data == NULL) {
      body->overflow = true;
      return ESP_ERR_NO_MEM;
    }
    body->data = data;
    body->capacity = capacity;
  }
  memcpy(body->data + body->length, event->data, event->data_len);
  body->length += event->data_len;
  body->data[body->length] = '\0';
  return ESP_OK;
}

static bool http_get(const char *path, http_body_t *body,
                     uint32_t generation) {
  body->generation = generation;
  char url[768];
  snprintf(url, sizeof(url), "http://%s:%d%s", CONFIG_SK1_MOONRAKER_HOST,
           CONFIG_SK1_MOONRAKER_PORT, path);
  esp_http_client_config_t config = {
      .url = url,
      .method = HTTP_METHOD_GET,
      .timeout_ms = 5000,
      .event_handler = http_event_handler,
      .user_data = body,
      .buffer_size = 2048,
  };
  esp_http_client_handle_t client = esp_http_client_init(&config);
  if (client == NULL) {
    return false;
  }
  esp_err_t result = esp_http_client_perform(client);
  int status = esp_http_client_get_status_code(client);
  esp_http_client_cleanup(client);
  if (result != ESP_OK || status != 200 || body->overflow) {
    ESP_LOGW(TAG, "GET %s failed: err=%s status=%d overflow=%d", path,
             esp_err_to_name(result), status, body->overflow);
    free(body->data);
    memset(body, 0, sizeof(*body));
    return false;
  }
  return true;
}

static bool load_thumbnail_info(const char *file_path,
                                thumbnail_info_t *thumbnail,
                                uint32_t generation) {
  char encoded[THUMBNAIL_PATH_MAX * 3];
  char request_path[sizeof(encoded) + 48];
  url_encode_path(file_path, encoded, sizeof(encoded));
  snprintf(request_path, sizeof(request_path),
           "/server/files/metadata?filename=%s", encoded);

  http_body_t body = {0};
  if (!http_get(request_path, &body, generation)) {
    return false;
  }
  cJSON *root = cJSON_Parse((char *)body.data);
  free(body.data);
  if (root == NULL) {
    return false;
  }

  cJSON *result = cJSON_GetObjectItemCaseSensitive(root, "result");
  cJSON *thumbnails = cJSON_GetObjectItemCaseSensitive(result, "thumbnails");
  cJSON *best = NULL;
  cJSON *item;
  cJSON_ArrayForEach(item, thumbnails) {
    cJSON *width = cJSON_GetObjectItemCaseSensitive(item, "width");
    if (cJSON_IsNumber(width) &&
        (best == NULL ||
         width->valueint >
             cJSON_GetObjectItemCaseSensitive(best, "width")->valueint)) {
      best = item;
    }
  }

  bool valid = false;
  if (best != NULL) {
    cJSON *path = cJSON_GetObjectItemCaseSensitive(best, "relative_path");
    cJSON *width = cJSON_GetObjectItemCaseSensitive(best, "width");
    cJSON *height = cJSON_GetObjectItemCaseSensitive(best, "height");
    cJSON *size = cJSON_GetObjectItemCaseSensitive(best, "size");
    if (cJSON_IsString(path) && cJSON_IsNumber(width) &&
        cJSON_IsNumber(height)) {
      snprintf(thumbnail->relative_path, sizeof(thumbnail->relative_path),
               "%s", path->valuestring);
      thumbnail->width = width->valueint;
      thumbnail->height = height->valueint;
      thumbnail->size = cJSON_IsNumber(size) ? (size_t)size->valuedouble : 0;
      valid = true;
    }
  }
  cJSON_Delete(root);
  return valid;
}

static void make_thumbnail_path(char *output, size_t size,
                                const char *file_path,
                                const char *relative_path) {
  const char *separator = strrchr(file_path, '/');
  if (separator == NULL) {
    snprintf(output, size, "%s", relative_path);
    return;
  }
  size_t directory_length = (size_t)(separator - file_path + 1);
  snprintf(output, size, "%.*s%s", (int)directory_length, file_path,
           relative_path);
}

static void png_read_memory(png_structp png, png_bytep output,
                            png_size_t length) {
  png_memory_reader_t *reader = png_get_io_ptr(png);
  if (reader == NULL || reader->offset + length > reader->length) {
    png_error(png, "PNG read overflow");
  }
  memcpy(output, reader->data + reader->offset, length);
  reader->offset += length;
}

static uint16_t quantize_rgb(uint8_t red, uint8_t green, uint8_t blue) {
  return ((uint16_t)(red >> 5) << 7) | ((uint16_t)(green >> 4) << 3) |
         (blue >> 5);
}

static uint16_t quantized_to_rgb565(uint16_t value) {
  uint16_t red3 = (value >> 7) & 0x07;
  uint16_t green4 = (value >> 3) & 0x0f;
  uint16_t blue3 = value & 0x07;
  uint16_t red5 = (red3 << 2) | (red3 >> 1);
  uint16_t green6 = (green4 << 2) | (green4 >> 2);
  uint16_t blue5 = (blue3 << 2) | (blue3 >> 1);
  return (red5 << 11) | (green6 << 5) | blue5;
}

static uint16_t *decode_png(const uint8_t *data, size_t length, int *width,
                            int *height, uint32_t generation) {
  png_structp png =
      png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
  png_infop info = png != NULL ? png_create_info_struct(png) : NULL;
  if (png == NULL || info == NULL) {
    png_destroy_read_struct(&png, &info, NULL);
    return NULL;
  }
  if (setjmp(png_jmpbuf(png))) {
    png_destroy_read_struct(&png, &info, NULL);
    return NULL;
  }

  png_memory_reader_t reader = {.data = data, .length = length};
  png_set_read_fn(png, &reader, png_read_memory);
  png_read_info(png, info);
  png_uint_32 source_width = png_get_image_width(png, info);
  png_uint_32 source_height = png_get_image_height(png, info);
  int color_type = png_get_color_type(png, info);
  int bit_depth = png_get_bit_depth(png, info);

  if (bit_depth == 16) {
    png_set_strip_16(png);
  }
  if (color_type == PNG_COLOR_TYPE_PALETTE) {
    png_set_palette_to_rgb(png);
  }
  if (color_type == PNG_COLOR_TYPE_GRAY && bit_depth < 8) {
    png_set_expand_gray_1_2_4_to_8(png);
  }
  if (png_get_valid(png, info, PNG_INFO_tRNS)) {
    png_set_tRNS_to_alpha(png);
  }
  if (color_type == PNG_COLOR_TYPE_GRAY ||
      color_type == PNG_COLOR_TYPE_GRAY_ALPHA) {
    png_set_gray_to_rgb(png);
  }
  if (!(color_type & PNG_COLOR_MASK_ALPHA) &&
      !png_get_valid(png, info, PNG_INFO_tRNS)) {
    png_set_filler(png, 0xff, PNG_FILLER_AFTER);
  }
  png_read_update_info(png, info);

  if (source_width == 0 || source_height == 0) {
    png_destroy_read_struct(&png, &info, NULL);
    return NULL;
  }
  int target_width = PREVIEW_SIZE;
  int target_height = PREVIEW_SIZE;
  if (source_width > source_height) {
    target_height = (int)(source_height * PREVIEW_SIZE / source_width);
  } else if (source_height > source_width) {
    target_width = (int)(source_width * PREVIEW_SIZE / source_height);
  }
  if (target_width < 1) {
    target_width = 1;
  }
  if (target_height < 1) {
    target_height = 1;
  }

  uint16_t *pixels =
      malloc((size_t)target_width * target_height * sizeof(*pixels));
  png_bytep row = malloc(png_get_rowbytes(png, info));
  if (pixels == NULL || row == NULL) {
    free(pixels);
    free(row);
    png_destroy_read_struct(&png, &info, NULL);
    return NULL;
  }

  int next_target_y = 0;
  for (png_uint_32 source_y = 0; source_y < source_height; ++source_y) {
    if (generation != display_low_priority_generation()) {
      free(pixels);
      free(row);
      png_destroy_read_struct(&png, &info, NULL);
      return NULL;
    }
    png_read_row(png, row, NULL);
    while (next_target_y < target_height &&
           (png_uint_32)(next_target_y * source_height / target_height) ==
               source_y) {
      for (int target_x = 0; target_x < target_width; ++target_x) {
        png_uint_32 source_x =
            (png_uint_32)(target_x * source_width / target_width);
        const uint8_t *pixel = row + source_x * 4;
        uint8_t alpha = pixel[3];
        uint8_t red = (uint8_t)(pixel[0] * alpha / 255);
        uint8_t green = (uint8_t)(pixel[1] * alpha / 255);
        uint8_t blue = (uint8_t)(pixel[2] * alpha / 255);
        pixels[next_target_y * target_width + target_x] =
            quantize_rgb(red, green, blue);
      }
      ++next_target_y;
    }
  }
  png_read_end(png, NULL);
  free(row);
  png_destroy_read_struct(&png, &info, NULL);
  *width = target_width;
  *height = target_height;
  return pixels;
}

static int palette_compare(const void *left, const void *right) {
  const palette_entry_t *a = left;
  const palette_entry_t *b = right;
  return a->count < b->count ? 1 : a->count > b->count ? -1 : 0;
}

static void write_u32_le(uint8_t *output, uint32_t value) {
  output[0] = value;
  output[1] = value >> 8;
  output[2] = value >> 16;
  output[3] = value >> 24;
}

static size_t encode_colpic(const uint16_t *pixels, int width, int height,
                            char **encoded_output, uint32_t generation) {
  uint32_t *histogram =
      calloc(COLPIC_PALETTE_SIZE, sizeof(*histogram));
  palette_entry_t *palette =
      malloc(COLPIC_PALETTE_SIZE * sizeof(*palette));
  uint16_t *palette_index =
      calloc(COLPIC_PALETTE_SIZE, sizeof(*palette_index));
  if (histogram == NULL || palette == NULL || palette_index == NULL) {
    free(histogram);
    free(palette);
    free(palette_index);
    return 0;
  }
  size_t pixel_count = (size_t)width * height;
  for (size_t index = 0; index < pixel_count; ++index) {
    if ((index & 0x3ff) == 0 &&
        generation != display_low_priority_generation()) {
      free(histogram);
      free(palette);
      free(palette_index);
      return 0;
    }
    ++histogram[pixels[index]];
  }

  uint16_t palette_count = 0;
  for (uint16_t value = 0; value < COLPIC_PALETTE_SIZE; ++value) {
    if (histogram[value] == 0) {
      continue;
    }
    palette[palette_count++] = (palette_entry_t){
        .color = quantized_to_rgb565(value),
        .count = histogram[value],
        .quantized = value,
    };
  }
  qsort(palette, palette_count, sizeof(*palette), palette_compare);
  for (uint16_t index = 0; index < palette_count; ++index) {
    palette_index[palette[index].quantized] = index;
  }

  size_t raw_capacity = 32 + palette_count * 2 + pixel_count * 2 + 3;
  uint8_t *raw = calloc(1, raw_capacity);
  if (raw == NULL) {
    free(histogram);
    free(palette);
    free(palette_index);
    return 0;
  }
  raw[0] = 3;
  write_u32_le(raw + 4, width);
  write_u32_le(raw + 8, height);
  write_u32_le(raw + 12, 98419516);
  write_u32_le(raw + 16, palette_count * 2);
  for (uint16_t index = 0; index < palette_count; ++index) {
    raw[32 + index * 2] = palette[index].color;
    raw[33 + index * 2] = palette[index].color >> 8;
  }

  size_t output = 32 + palette_count * 2;
  uint16_t last_segment = 0;
  for (size_t index = 0; index < pixel_count;) {
    if ((index & 0x3ff) == 0 &&
        generation != display_low_priority_generation()) {
      free(raw);
      free(histogram);
      free(palette);
      free(palette_index);
      return 0;
    }
    uint16_t color = pixels[index];
    size_t run = 1;
    while (index + run < pixel_count && pixels[index + run] == color &&
           run < 255) {
      ++run;
    }
    uint16_t mapped = palette_index[color];
    uint16_t segment = (mapped >> 5) & 0x1f;
    uint8_t entry = mapped & 0x1f;
    if (segment != last_segment) {
      raw[output++] = (7 << 5) | segment;
      last_segment = segment;
    }
    if (run <= 6) {
      raw[output++] = ((uint8_t)run << 5) | entry;
    } else {
      raw[output++] = entry;
      raw[output++] = run;
    }
    index += run;
  }
  write_u32_le(raw + 20, output - 32 - palette_count * 2);

  while (output % 3 != 0) {
    raw[output++] = 0;
  }
  size_t encoded_length = output * 4 / 3;
  char *encoded = malloc(encoded_length + 1);
  if (encoded == NULL) {
    free(raw);
    free(histogram);
    free(palette);
    free(palette_index);
    return 0;
  }
  for (size_t input = 0, target = 0; input < output; input += 3) {
    uint8_t values[4] = {
        raw[input] >> 2,
        ((raw[input] & 0x03) << 4) | (raw[input + 1] >> 4),
        ((raw[input + 1] & 0x0f) << 2) | (raw[input + 2] >> 6),
        raw[input + 2] & 0x3f,
    };
    for (size_t part = 0; part < 4; ++part) {
      uint8_t character = values[part] + 48;
      encoded[target++] = character == '\\' ? '~' : (char)character;
    }
  }
  encoded[encoded_length] = '\0';
  free(raw);
  free(histogram);
  free(palette);
  free(palette_index);
  *encoded_output = encoded;
  return encoded_length;
}

static bool send_preview(const char *encoded, size_t length,
                         const thumbnail_request_t *request) {
  if (!request_current(request)) {
    return false;
  }
  const char *close_command;
  const char *visible_command;
  const char *write_prefix;
  if (request->target == THUMBNAIL_TARGET_PRINT) {
    close_command = "Print_Trun_1.cp0.close()";
    visible_command = "vis cp0,1";
    write_prefix = "cp0.write(\"";
  } else if (request->target == THUMBNAIL_TARGET_RESULT) {
    close_command = "print_done.cp0.close()";
    visible_command = "vis print_done.cp0,1";
    write_prefix = "print_done.cp0.write(\"";
  } else {
    close_command = "preview.cp0.close()";
    visible_command = "vis cp0,1";
    write_prefix = "preview.cp0.write(\"";
  }
  display_send_async(close_command, DISPLAY_CMD_NORMAL);
  display_send_async(visible_command, DISPLAY_CMD_NORMAL);

  for (size_t offset = 0; offset < length; offset += COLPIC_CHUNK_SIZE) {
    if (!request_current(request)) {
      return false;
    }
    size_t chunk = length - offset;
    if (chunk > COLPIC_CHUNK_SIZE) {
      chunk = COLPIC_CHUNK_SIZE;
    }
    size_t command_length = strlen(write_prefix) + strlen("\")") + chunk;
    char *command = malloc(command_length + 1);
    if (command == NULL) {
      return false;
    }
    int prefix =
        snprintf(command, command_length + 1, "%s", write_prefix);
    memcpy(command + prefix, encoded + offset, chunk);
    memcpy(command + prefix + chunk, "\")", 3);
    esp_err_t result;
    do {
      result = display_send_low_if_current(
          (const uint8_t *)command, prefix + chunk + 2, request->generation);
      if (result == ESP_ERR_NO_MEM && request_current(request)) {
        vTaskDelay(pdMS_TO_TICKS(5));
      }
    } while (result == ESP_ERR_NO_MEM && request_current(request));
    free(command);
    if (result != ESP_OK) {
      return false;
    }
    vTaskDelay(pdMS_TO_TICKS(1));
  }
  return true;
}

static void process_request(const thumbnail_request_t *request) {
  if (!request_current(request)) {
    return;
  }
  thumbnail_info_t info = {0};
  if (!load_thumbnail_info(request->file_path, &info, request->generation)) {
    ESP_LOGW(TAG, "no thumbnail metadata for %s", request->file_path);
    return;
  }
  thumbnail_cache_put(request->file_path, &info);
  ESP_LOGI(TAG, "metadata %s %ux%u size=%u", info.relative_path, info.width,
           info.height, (unsigned)info.size);

  char thumbnail_path[THUMBNAIL_PATH_MAX];
  make_thumbnail_path(thumbnail_path, sizeof(thumbnail_path),
                      request->file_path, info.relative_path);
  char encoded_path[THUMBNAIL_PATH_MAX * 3];
  char request_path[sizeof(encoded_path) + 32];
  url_encode_path(thumbnail_path, encoded_path, sizeof(encoded_path));
  snprintf(request_path, sizeof(request_path), "/server/files/gcodes/%s",
           encoded_path);

  http_body_t body = {0};
  if (!request_current(request) ||
      !http_get(request_path, &body, request->generation)) {
    return;
  }
  int width = 0;
  int height = 0;
  uint16_t *pixels = decode_png(body.data, body.length, &width, &height,
                                request->generation);
  free(body.data);
  if (pixels == NULL || !request_current(request)) {
    free(pixels);
    ESP_LOGW(TAG, "PNG decode failed or cancelled");
    return;
  }

  char *encoded = NULL;
  size_t encoded_length =
      encode_colpic(pixels, width, height, &encoded, request->generation);
  free(pixels);
  if (encoded_length == 0 || !request_current(request)) {
    free(encoded);
    ESP_LOGW(TAG, "ColPic encode failed or cancelled");
    return;
  }
  ESP_LOGI(TAG, "encoded %dx%d to %u bytes", width, height,
           (unsigned)encoded_length);
  bool sent = send_preview(encoded, encoded_length, request);
  free(encoded);
  ESP_LOGI(TAG, "preview %s", sent ? "queued" : "cancelled");
}

static void thumbnail_worker_task(void *argument) {
  (void)argument;
  thumbnail_request_t request;
  while (true) {
    if (xQueueReceive(request_queue, &request, portMAX_DELAY) != pdTRUE) {
      continue;
    }
    if (wifi_manager_wait_connected(pdMS_TO_TICKS(5000)) == pdTRUE) {
      process_request(&request);
    }
  }
}

esp_err_t thumbnail_worker_start(void) {
  request_queue = xQueueCreate(1, sizeof(thumbnail_request_t));
  if (request_queue == NULL) {
    return ESP_ERR_NO_MEM;
  }
  return xTaskCreate(thumbnail_worker_task, "thumbnail", 10240, NULL, 3,
                     NULL) == pdPASS
             ? ESP_OK
             : ESP_ERR_NO_MEM;
}

esp_err_t thumbnail_request_preview(const char *file_path,
                                    uint32_t display_generation) {
  if (file_path == NULL || file_path[0] == '\0' || request_queue == NULL) {
    return ESP_ERR_INVALID_ARG;
  }
  thumbnail_request_t request = {
      .target = THUMBNAIL_TARGET_PREVIEW,
      .generation = display_generation,
  };
  snprintf(request.file_path, sizeof(request.file_path), "%s", file_path);
  return xQueueOverwrite(request_queue, &request) == pdTRUE ? ESP_OK
                                                            : ESP_FAIL;
}

esp_err_t thumbnail_request_print(const char *file_path,
                                  uint32_t display_generation) {
  if (file_path == NULL || file_path[0] == '\0' || request_queue == NULL) {
    return ESP_ERR_INVALID_ARG;
  }
  thumbnail_request_t request = {
      .target = THUMBNAIL_TARGET_PRINT,
      .generation = display_generation,
  };
  snprintf(request.file_path, sizeof(request.file_path), "%s", file_path);
  return xQueueOverwrite(request_queue, &request) == pdTRUE ? ESP_OK
                                                            : ESP_FAIL;
}

esp_err_t thumbnail_request_result(const char *file_path,
                                   uint32_t display_generation) {
  if (file_path == NULL || file_path[0] == '\0' || request_queue == NULL) {
    return ESP_ERR_INVALID_ARG;
  }
  thumbnail_request_t request = {
      .target = THUMBNAIL_TARGET_RESULT,
      .generation = display_generation,
  };
  snprintf(request.file_path, sizeof(request.file_path), "%s", file_path);
  return xQueueOverwrite(request_queue, &request) == pdTRUE ? ESP_OK
                                                            : ESP_FAIL;
}
