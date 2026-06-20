#include "wifi_manager.h"

#include "app_state.h"

#include "esp_check.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "esp_wifi.h"
#include "freertos/event_groups.h"
#include "sdkconfig.h"

#include <stdio.h>
#include <string.h>

#define WIFI_CONNECTED_BIT BIT0
#define WIFI_DISCONNECTED_BIT BIT1

static const char *TAG = "wifi";
static EventGroupHandle_t wifi_events;
static esp_netif_t *station_netif;

static void update_rssi(void) {
  wifi_ap_record_t access_point;
  if (esp_wifi_sta_get_ap_info(&access_point) != ESP_OK) {
    return;
  }

  esp_netif_ip_info_t ip_info;
  char ip_address[16] = "";
  if (esp_netif_get_ip_info(station_netif, &ip_info) == ESP_OK) {
    snprintf(ip_address, sizeof(ip_address), IPSTR, IP2STR(&ip_info.ip));
  }
  app_state_set_wifi(true, ip_address, access_point.rssi);
}

static void wifi_event_handler(void *argument, esp_event_base_t event_base,
                               int32_t event_id, void *event_data) {
  (void)argument;
  (void)event_data;

  if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
    esp_wifi_connect();
    return;
  }

  if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
    wifi_event_sta_disconnected_t *disconnected = event_data;
    xEventGroupClearBits(wifi_events, WIFI_CONNECTED_BIT);
    xEventGroupSetBits(wifi_events, WIFI_DISCONNECTED_BIT);
    app_state_set_wifi(false, NULL, 0);
    app_state_set_moonraker_online(false);
    ESP_LOGW(TAG, "disconnected reason=%u, reconnecting",
             disconnected ? disconnected->reason : 0);
    esp_wifi_connect();
    return;
  }

  if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
    ip_event_got_ip_t *got_ip = event_data;
    char ip_address[16];
    snprintf(ip_address, sizeof(ip_address), IPSTR,
             IP2STR(&got_ip->ip_info.ip));
    xEventGroupClearBits(wifi_events, WIFI_DISCONNECTED_BIT);
    xEventGroupSetBits(wifi_events, WIFI_CONNECTED_BIT);
    app_state_set_wifi(true, ip_address, 0);
    update_rssi();
    ESP_LOGI(TAG, "connected, ip=%s", ip_address);
  }
}

esp_err_t wifi_manager_start(void) {
  app_state_set_wifi_connecting();
  wifi_events = xEventGroupCreate();
  if (wifi_events == NULL) {
    return ESP_ERR_NO_MEM;
  }
  xEventGroupSetBits(wifi_events, WIFI_DISCONNECTED_BIT);

  ESP_RETURN_ON_ERROR(esp_netif_init(), TAG, "initialize netif");
  ESP_RETURN_ON_ERROR(esp_event_loop_create_default(), TAG,
                      "create event loop");
  station_netif = esp_netif_create_default_wifi_sta();
  if (station_netif == NULL) {
    return ESP_ERR_NO_MEM;
  }

  wifi_init_config_t init_config = WIFI_INIT_CONFIG_DEFAULT();
  ESP_RETURN_ON_ERROR(esp_wifi_init(&init_config), TAG, "initialize Wi-Fi");
  ESP_RETURN_ON_ERROR(esp_event_handler_register(WIFI_EVENT, ESP_EVENT_ANY_ID,
                                                 wifi_event_handler, NULL),
                      TAG, "register Wi-Fi handler");
  ESP_RETURN_ON_ERROR(esp_event_handler_register(IP_EVENT, IP_EVENT_STA_GOT_IP,
                                                 wifi_event_handler, NULL),
                      TAG, "register IP handler");

  wifi_config_t wifi_config = {0};
  strlcpy((char *)wifi_config.sta.ssid, CONFIG_SK1_WIFI_SSID,
          sizeof(wifi_config.sta.ssid));
  strlcpy((char *)wifi_config.sta.password, CONFIG_SK1_WIFI_PASSWORD,
          sizeof(wifi_config.sta.password));
  wifi_config.sta.threshold.authmode = WIFI_AUTH_WPA2_PSK;
  wifi_config.sta.sae_pwe_h2e = WPA3_SAE_PWE_BOTH;

  ESP_RETURN_ON_ERROR(esp_wifi_set_mode(WIFI_MODE_STA), TAG,
                      "set station mode");
  ESP_RETURN_ON_ERROR(esp_wifi_set_config(WIFI_IF_STA, &wifi_config), TAG,
                      "set station config");
  ESP_RETURN_ON_ERROR(esp_wifi_start(), TAG, "start Wi-Fi");
  return ESP_OK;
}

BaseType_t wifi_manager_wait_connected(TickType_t timeout) {
  EventBits_t bits = xEventGroupWaitBits(wifi_events, WIFI_CONNECTED_BIT,
                                         pdFALSE, pdTRUE, timeout);
  return (bits & WIFI_CONNECTED_BIT) != 0 ? pdTRUE : pdFALSE;
}

BaseType_t wifi_manager_wait_disconnected(TickType_t timeout) {
  EventBits_t bits = xEventGroupWaitBits(wifi_events, WIFI_DISCONNECTED_BIT,
                                         pdFALSE, pdTRUE, timeout);
  return (bits & WIFI_DISCONNECTED_BIT) != 0 ? pdTRUE : pdFALSE;
}
