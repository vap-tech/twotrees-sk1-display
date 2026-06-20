#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define APP_STATE_TEXT_MAX 32
#define APP_STATE_COMMAND_TEXT_MAX 96
#define APP_STATE_FILE_MAX 192

typedef enum {
  APP_COMMAND_IDLE,
  APP_COMMAND_PENDING,
  APP_COMMAND_SUCCEEDED,
  APP_COMMAND_FAILED,
} app_command_status_t;

typedef enum {
  APP_CONNECTION_BOOT,
  APP_CONNECTION_WIFI_CONNECTING,
  APP_CONNECTION_WIFI_CONNECTED,
  APP_CONNECTION_MOONRAKER_CONNECTING,
  APP_CONNECTION_MOONRAKER_READY,
  APP_CONNECTION_DISCONNECTED,
} app_connection_state_t;

typedef enum {
  APP_PRINT_IDLE,
  APP_PRINT_STARTING,
  APP_PRINT_PRINTING,
  APP_PRINT_PAUSING,
  APP_PRINT_PAUSED,
  APP_PRINT_RESUMING,
  APP_PRINT_CANCELLING,
  APP_PRINT_COMPLETE,
  APP_PRINT_CANCELLED,
  APP_PRINT_ERROR,
} app_print_state_t;

typedef struct {
  app_connection_state_t connection_state;
  app_print_state_t print_mode;
  bool wifi_connected;
  char ip_address[16];
  int wifi_rssi;
  bool moonraker_online;
  char moonraker_version[APP_STATE_TEXT_MAX];
  char klippy_state[APP_STATE_TEXT_MAX];
  char printer_state[APP_STATE_TEXT_MAX];
  char print_state[APP_STATE_TEXT_MAX];
  char print_filename[APP_STATE_FILE_MAX];
  int nozzle_current;
  int nozzle_target;
  int bed_current;
  int bed_target;
  bool caselight_on;
  int fan_values[3];
  int print_progress;
  int print_elapsed_seconds;
  int print_estimated_seconds;
  uint32_t command_generation;
  app_command_status_t command_status;
  char command_name[APP_STATE_TEXT_MAX];
  char command_message[APP_STATE_COMMAND_TEXT_MAX];
  int64_t updated_ms;
} app_state_snapshot_t;

typedef struct {
  bool has_nozzle_current;
  int nozzle_current;
  bool has_nozzle_target;
  int nozzle_target;
  bool has_bed_current;
  int bed_current;
  bool has_bed_target;
  int bed_target;
  bool has_caselight;
  bool caselight_on;
  bool has_fan_values[3];
  int fan_values[3];
  bool has_print_state;
  const char *print_state;
  bool has_print_filename;
  const char *print_filename;
  bool has_print_progress;
  int print_progress;
  bool has_print_elapsed_seconds;
  int print_elapsed_seconds;
  bool has_print_estimated_seconds;
  int print_estimated_seconds;
} app_printer_update_t;

void app_state_init(void);
void app_state_get(app_state_snapshot_t *snapshot);

void app_state_set_wifi_connecting(void);
void app_state_set_wifi(bool connected, const char *ip_address, int rssi);
void app_state_set_moonraker_online(bool online);
void app_state_set_server_info(const char *version, const char *klippy_state);
void app_state_set_printer_info(const char *printer_state);
void app_state_apply_printer_update(const app_printer_update_t *update);
void app_state_set_printer_status(int nozzle_current, int nozzle_target,
                                  int bed_current, int bed_target,
                                  const char *print_state,
                                  const char *print_filename,
                                  int print_progress);
uint32_t app_state_command_begin(const char *name);
void app_state_command_finish(uint32_t generation, bool succeeded,
                              const char *message);
void app_state_acknowledge_print_result(void);
void app_state_set_fan_value(size_t index, int value);
