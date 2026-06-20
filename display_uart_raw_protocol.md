# Display UART raw protocol manual

Источник: `tools/display_uart_demo_host.py`.

Каждая команда отправляется как ASCII-строка плюс терминатор `ff ff ff`.
Шаблоны в угловых скобках, например `<value>`, означают runtime-подстановку из кода; hex для них показывает байты самого плейсхолдера, а не конкретного значения.

| Hex raw frame | ASCII template | Описание |
|---|---|---|
| `62 31 35 2e 70 69 63 63 32 3d 38 33 ff ff ff` | `b15.picc2=83` | Установить картинку компонента для второго состояния/нажатия. Контекст: `complete_z_tilt:1260`. |
| `62 31 36 2e 70 69 63 63 32 3d 38 33 ff ff ff` | `b16.picc2=83` | Установить картинку компонента для второго состояния/нажатия. Контекст: `complete_z_tilt:1261`. |
| `62 31 37 2e 70 69 63 63 32 3d 38 33 ff ff ff` | `b17.picc2=83` | Установить картинку компонента для второго состояния/нажатия. Контекст: `complete_z_tilt:1262`. |
| `68 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `h<component>.val=<value>` | Установить numeric value компонента. Контекст: `handle_numeric_event:1812`. |
| `6e 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n<component>.val=<value>` | Установить numeric value компонента. Контекст: `handle_numeric_event:1813`. |
| `70 61 67 65 20 31 31 ff ff ff` | `page 11` | Переключение на page 11: System верхняя страница. Контекст: `handle_touch_event:1348`. |
| `70 61 67 65 20 32 31 ff ff ff` | `page 21` | Переключение на page 21: FAQ. Контекст: `handle_touch_event:1350`. |
| `70 61 67 65 20 37 34 ff ff ff` | `page 74` | Переключение на page 74: Continue dialog. Контекст: `handle_touch_event:1361`. |
| `70 61 67 65 20 32 37 ff ff ff` | `page 27` | Переключение на page 27: Pause/Stop dialog. Контекст: `handle_touch_event:1366`. |
| `70 61 67 65 20 36 38 ff ff ff` | `page 68` | Переключение на page 68: Emergency dialog. Контекст: `handle_touch_event:1383`. |
| `70 61 67 65 20 31 ff ff ff` | `page 1` | Переключение на page 1: клавиатура ввода температуры. Контекст: `handle_touch_event:1562`. |
| `70 61 67 65 20 31 34 ff ff ff` | `page 14` | Переключение на page 14: System нижняя страница. Контекст: `handle_touch_event:1689`. |
| `70 61 67 65 20 31 39 ff ff ff` | `page 19` | Переключение на page 19: Export diary. Контекст: `handle_touch_event:1695`. |
| `70 61 67 65 20 31 35 ff ff ff` | `page 15` | Переключение на page 15: About. Контекст: `handle_touch_event:1698`. |
| `70 61 67 65 20 32 30 ff ff ff` | `page 20` | Переключение на page 20: Factory reset. Контекст: `handle_touch_event:1712`. |
| `70 61 67 65 20 35 32 ff ff ff` | `page 52` | Переключение на page 52: Online manual. Контекст: `handle_touch_event:1729`. |
| `70 61 67 65 20 35 33 ff ff ff` | `page 53` | Переключение на page 53: Contact. Контекст: `handle_touch_event:1732`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `<component>.val=<value>` | Установить numeric value компонента. Контекст: `ramp_temperature_to_target:585`. |
| `70 61 67 65 20 33 36 ff ff ff` | `page 36` | Переключение на page 36: Bed mesh. Контекст: `run_demo_bed_mesh:1109`. |
| `71 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 38 30 ff ff ff` | `q<component>.picc=80` | Установить картинку/иконку компонента. Контекст: `run_demo_bed_mesh:1137`. |
| `70 61 67 65 20 35 31 ff ff ff` | `page 51` | Переключение на page 51: Homing overlay. Контекст: `run_demo_homing:998`. |
| `70 61 67 65 20 36 37 ff ff ff` | `page 67` | Переключение на page 67: Rebooting. Контекст: `run_demo_reboot:981`. |
| `70 61 67 65 20 33 37 ff ff ff` | `page 37` | Переключение на page 37: Shaper. Контекст: `run_demo_shaper:1156`. |
| `71 34 2e 70 69 63 63 3d 31 30 39 ff ff ff` | `q4.picc=109` | Установить картинку/иконку компонента. Контекст: `run_demo_shaper:1161`. |
| `71 34 2e 70 69 63 63 3d 31 31 30 ff ff ff` | `q4.picc=110` | Установить картинку/иконку компонента. Контекст: `run_demo_shaper:1165`. |
| `71 34 2e 70 69 63 63 3d 31 31 31 ff ff ff` | `q4.picc=111` | Установить картинку/иконку компонента. Контекст: `run_demo_shaper:1166`. |
| `70 61 67 65 20 33 34 ff ff ff` | `page 34` | Переключение на page 34: Triangle/Z tilt auto. Контекст: `run_demo_triangle:1072`. |
| `71 30 2e 70 69 63 63 3d 37 39 ff ff ff` | `q0.picc=79` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1073`. |
| `71 31 2e 70 69 63 63 3d 37 38 ff ff ff` | `q1.picc=78` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1074`. |
| `71 32 2e 70 69 63 63 3d 37 38 ff ff ff` | `q2.picc=78` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1075`. |
| `71 33 2e 70 69 63 63 3d 37 38 ff ff ff` | `q3.picc=78` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1076`. |
| `68 65 61 74 5f 63 6f 6d 70 6c 65 74 65 3d 31 ff ff ff` | `heat_complete=1` | Установить внутренний флаг страницы/процесса HMI. Контекст: `run_demo_triangle:1080`. |
| `71 31 2e 70 69 63 63 3d 37 39 ff ff ff` | `q1.picc=79` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1080`. |
| `68 6f 6d 65 5f 63 6f 6d 70 6c 65 74 65 3d 31 ff ff ff` | `home_complete=1` | Установить внутренний флаг страницы/процесса HMI. Контекст: `run_demo_triangle:1081`. |
| `71 32 2e 70 69 63 63 3d 37 39 ff ff ff` | `q2.picc=79` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1081`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 38 32 ff ff ff` | `<component>.picc=82` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1092`. |
| `6c 65 76 65 6c 5f 63 6f 6d 70 6c 65 74 65 3d 31 ff ff ff` | `level_complete=1` | Установить внутренний флаг страницы/процесса HMI. Контекст: `run_demo_triangle:1095`. |
| `71 33 2e 70 69 63 63 3d 37 39 ff ff ff` | `q3.picc=79` | Установить картинку/иконку компонента. Контекст: `run_demo_triangle:1095`. |
| `76 69 73 20 74 34 2c 31 ff ff ff` | `vis t4,1` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `run_demo_triangle:1095`. |
| `70 61 67 65 20 33 35 ff ff ff` | `page 35` | Переключение на page 35: Probe/Z tilt manual. Контекст: `run_demo_z_tilt:1189`. |
| `62 31 32 2e 70 69 63 63 3d 38 33 ff ff ff` | `b12.picc=83` | Установить картинку/иконку компонента. Контекст: `run_demo_z_tilt:1194`. |
| `62 31 33 2e 70 69 63 63 3d 38 33 ff ff ff` | `b13.picc=83` | Установить картинку/иконку компонента. Контекст: `run_demo_z_tilt:1195`. |
| `62 31 34 2e 70 69 63 63 3d 38 34 ff ff ff` | `b14.picc=84` | Установить картинку/иконку компонента. Контекст: `run_demo_z_tilt:1196`. |
| `62 31 35 2e 70 69 63 63 32 3d 38 34 ff ff ff` | `b15.picc2=84` | Установить картинку компонента для второго состояния/нажатия. Контекст: `run_demo_z_tilt:1201`. |
| `62 31 36 2e 70 69 63 63 32 3d 38 34 ff ff ff` | `b16.picc2=84` | Установить картинку компонента для второго состояния/нажатия. Контекст: `run_demo_z_tilt:1201`. |
| `62 31 37 2e 70 69 63 63 32 3d 38 34 ff ff ff` | `b17.picc2=84` | Установить картинку компонента для второго состояния/нажатия. Контекст: `run_demo_z_tilt:1201`. |
| `70 61 67 65 20 34 35 ff ff ff` | `page 45` | Переключение на page 45: Starting overlay. Контекст: `run_init_sequence:1292`. |
| `74 32 2e 61 70 68 3d 30 ff ff ff` | `t2.aph=0` | Установить alpha/прозрачность компонента. Контекст: `run_load_unload_process:606`. |
| `74 33 2e 61 70 68 3d 30 ff ff ff` | `t3.aph=0` | Установить alpha/прозрачность компонента. Контекст: `run_load_unload_process:607`. |
| `71 31 2e 70 69 63 63 3d 31 34 ff ff ff` | `q1.picc=14` | Установить картинку/иконку компонента. Контекст: `run_load_unload_process:608`. |
| `71 32 2e 70 69 63 63 3d 31 34 ff ff ff` | `q2.picc=14` | Установить картинку/иконку компонента. Контекст: `run_load_unload_process:609`. |
| `71 30 2e 70 69 63 63 3d 31 35 ff ff ff` | `q0.picc=15` | Установить картинку/иконку компонента. Контекст: `run_load_unload_process:610`. |
| `74 31 2e 61 70 68 3d 31 30 30 ff ff ff` | `t1.aph=100` | Установить alpha/прозрачность компонента. Контекст: `run_load_unload_process:611`. |
| `71 31 2e 70 69 63 63 3d 31 35 ff ff ff` | `q1.picc=15` | Установить картинку/иконку компонента. Контекст: `run_load_unload_process:636`. |
| `74 32 2e 61 70 68 3d 31 30 30 ff ff ff` | `t2.aph=100` | Установить alpha/прозрачность компонента. Контекст: `run_load_unload_process:637`. |
| `76 69 73 20 74 32 2c 31 ff ff ff` | `vis t2,1` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `run_load_unload_process:638`. |
| `71 32 2e 70 69 63 63 3d 31 35 ff ff ff` | `q2.picc=15` | Установить картинку/иконку компонента. Контекст: `run_load_unload_process:652`. |
| `74 33 2e 61 70 68 3d 31 30 30 ff ff ff` | `t3.aph=100` | Установить alpha/прозрачность компонента. Контекст: `run_load_unload_process:653`. |
| `76 69 73 20 74 33 2c 31 ff ff ff` | `vis t3,1` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `run_load_unload_process:654`. |
| `6f 62 69 63 6f 2e 71 72 30 2e 74 78 74 3d 22 22 ff ff ff` | `obico.qr0.txt=""` | Установить текст компонента. Контекст: `run_obico_page:841`. |
| `6f 62 69 63 6f 2e 71 72 30 2e 61 70 68 3d 30 ff ff ff` | `obico.qr0.aph=0` | Установить alpha/прозрачность компонента. Контекст: `run_obico_page:842`. |
| `6f 62 69 63 6f 2e 71 30 2e 70 69 63 63 3d 31 33 32 ff ff ff` | `obico.q0.picc=132` | Установить картинку/иконку компонента. Контекст: `run_obico_page:843`. |
| `6f 62 69 63 6f 2e 71 31 2e 70 69 63 63 3d 31 33 32 ff ff ff` | `obico.q1.picc=132` | Установить картинку/иконку компонента. Контекст: `run_obico_page:844`. |
| `70 61 67 65 20 37 38 ff ff ff` | `page 78` | Переключение на page 78: Obico/link. Контекст: `run_obico_page:845`. |
| `6f 62 69 63 6f 2e 71 30 2e 70 69 63 63 3d 31 33 31 ff ff ff` | `obico.q0.picc=131` | Установить картинку/иконку компонента. Контекст: `run_obico_page:856`. |
| `6f 62 69 63 6f 2e 71 72 30 2e 61 70 68 3d 31 32 37 ff ff ff` | `obico.qr0.aph=127` | Установить alpha/прозрачность компонента. Контекст: `run_obico_page:857`. |
| `6f 62 69 63 6f 2e 71 72 30 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `obico.qr0.txt="<text>"` | Установить текст компонента. Контекст: `run_obico_page:858`. |
| `70 61 67 65 20 37 33 ff ff ff` | `page 73` | Переключение на page 73: Machine stopping. Контекст: `run_stop_print:819`. |
| `70 61 67 65 20 33 33 ff ff ff` | `page 33` | Переключение на page 33: Calibration main. Контекст: `send_calibration_page:923`. |
| `71 3c 69 6e 64 65 78 3e 2e 70 69 63 63 3d 31 31 36 ff ff ff` | `q<index>.picc=116` | Установить картинку/иконку компонента. Контекст: `send_calibration_page:926`. |
| `74 3c 69 6e 64 65 78 3e 2e 74 78 74 3d 22 30 2e 30 30 22 ff ff ff` | `t<index>.txt="0.00"` | Установить текст компонента. Контекст: `send_calibration_page:927`. |
| `71 3c 69 6e 64 65 78 3e 2e 70 69 63 63 3d 38 30 ff ff ff` | `q<index>.picc=80` | Установить картинку/иконку компонента. Контекст: `send_calibration_page:939`. |
| `74 3c 69 6e 64 65 78 3e 2e 74 78 74 3d 22 3c 76 61 6c 75 65 3e 22 ff ff ff` | `t<index>.txt="<value>"` | Установить текст компонента. Контекст: `send_calibration_page:940`. |
| `74 31 2e 74 78 74 3d 22 3c 7a 5f 6d 61 78 3e 22 ff ff ff` | `t1.txt="<z_max>"` | Установить текст компонента. Контекст: `send_calibration_page:944`. |
| `74 32 2e 74 78 74 3d 22 3c 7a 5f 6d 69 6e 3e 22 ff ff ff` | `t2.txt="<z_min>"` | Установить текст компонента. Контекст: `send_calibration_page:945`. |
| `74 33 2e 74 78 74 3d 22 3c 7a 5f 6f 66 66 73 65 74 3e 22 ff ff ff` | `t3.txt="<z_offset>"` | Установить текст компонента. Контекст: `send_calibration_page:946`. |
| `74 34 2e 74 78 74 3d 22 3c 73 68 61 70 65 72 5f 66 72 65 71 5f 78 3e 22 ff ff ff` | `t4.txt="<shaper_freq_x>"` | Установить текст компонента. Контекст: `send_calibration_page:947`. |
| `74 35 2e 74 78 74 3d 22 3c 73 68 61 70 65 72 5f 66 72 65 71 5f 79 3e 22 ff ff ff` | `t5.txt="<shaper_freq_y>"` | Установить текст компонента. Контекст: `send_calibration_page:948`. |
| `74 30 2e 74 78 74 3d 22 3c 74 69 6c 74 5f 74 6f 6c 65 72 61 6e 63 65 3e 22 ff ff ff` | `t0.txt="<tilt_tolerance>"` | Установить текст компонента. Контекст: `send_calibration_page:949`. |
| `66 69 6c 61 6d 65 6e 74 2e 62 39 2e 70 69 63 63 3d 32 38 ff ff ff` | `filament.b9.picc=28` | Установить картинку/иконку компонента. Контекст: `send_calibration_page:954`. |
| `66 69 6c 61 6d 65 6e 74 2e 62 39 2e 70 69 63 63 32 3d 32 38 ff ff ff` | `filament.b9.picc2=28` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_calibration_page:955`. |
| `70 61 67 65 20 36 ff ff ff` | `page 6` | Переключение на page 6: Fan page. Контекст: `send_fan_page:271`. |
| `70 61 67 65 20 39 ff ff ff` | `page 9` | Переключение на page 9: file preview. Контекст: `send_file_preview_page:718`. |
| `67 30 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `g0.txt="<text>"` | Установить текст компонента. Контекст: `send_file_preview_page:719`. |
| `6e 34 2e 76 61 6c 3d 30 30 ff ff ff` | `n4.val=00` | Установить numeric value компонента. Контекст: `send_file_preview_page:720`. |
| `6e 35 2e 76 61 6c 3d 30 30 ff ff ff` | `n5.val=00` | Установить numeric value компонента. Контекст: `send_file_preview_page:721`. |
| `74 32 2e 74 78 74 3d 22 30 22 ff ff ff` | `t2.txt="0"` | Установить текст компонента. Контекст: `send_file_preview_page:722`. |
| `70 72 65 76 69 65 77 2e 63 70 30 2e 63 6c 6f 73 65 28 29 ff ff ff` | `preview.cp0.close()` | Закрыть canvas/preview stream перед новой отрисовкой. Контекст: `send_file_preview_page:723`. |
| `70 61 67 65 20 3c 70 61 67 65 3e ff ff ff` | `page <page>` | Переключение на page <page>: Files/USB: список файлов, папки, слоты, превью.. Контекст: `send_files_page:392`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 63 6c 6f 73 65 28 29 ff ff ff` | `<component>.close()` | Закрыть canvas/preview stream перед новой отрисовкой. Контекст: `send_files_page:393`. |
| `76 69 73 20 63 70 30 2c 30 ff ff ff` | `vis cp0,0` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_files_page:396`. |
| `76 69 73 20 63 70 31 2c 30 ff ff ff` | `vis cp1,0` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_files_page:397`. |
| `76 69 73 20 63 70 32 2c 30 ff ff ff` | `vis cp2,0` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_files_page:398`. |
| `76 69 73 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2c 30 ff ff ff` | `vis <component>,0` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_files_page:439`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 22 ff ff ff` | `<component>.txt=""` | Установить текст компонента. Контекст: `send_files_page:444`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 31 30 30 ff ff ff` | `<component>.picc=100` | Установить картинку/иконку компонента. Контекст: `send_files_page:446`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 32 3d 31 30 30 ff ff ff` | `<component>.picc2=100` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_files_page:447`. |
| `74 31 35 2e 74 78 74 3d 22 22 ff ff ff` | `t15.txt=""` | Установить текст компонента. Контекст: `send_files_page:449`. |
| `76 69 73 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2c 31 ff ff ff` | `vis <component>,1` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_files_page:453`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `<component>.txt="<text>"` | Установить текст компонента. Контекст: `send_files_page:454`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 39 39 ff ff ff` | `<component>.picc=99` | Установить картинку/иконку компонента. Контекст: `send_files_page:455`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 39 38 ff ff ff` | `<component>.picc=98` | Установить картинку/иконку компонента. Контекст: `send_files_page:465`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 32 3d 39 39 ff ff ff` | `<component>.picc2=99` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_files_page:466`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 31 38 ff ff ff` | `<component>.picc=18` | Установить картинку/иконку компонента. Контекст: `send_files_page:467`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 30 2e 30 30 30 22 ff ff ff` | `<component>.txt="0.000"` | Установить текст компонента. Контекст: `send_files_page:471`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 76 61 6c 3d 30 ff ff ff` | `<component>.val=0` | Установить numeric value компонента. Контекст: `send_files_page:472`. |
| `76 69 73 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2c 3c 31 5f 69 66 5f 65 6e 74 72 79 5f 68 61 73 5f 70 72 65 76 69 65 77 5f 65 3e ff ff ff` | `vis <component>,<1_if_entry_has_preview_e>` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_files_page:474`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 77 72 69 74 65 28 22 3c 63 68 75 6e 6b 3e 22 29 ff ff ff` | `<component>.write("<chunk>")` | Записать chunk изображения/preview в canvas компонент. Контекст: `send_files_page:488`. |
| `70 61 67 65 20 31 30 ff ff ff` | `page 10` | Переключение на page 10: history. Контекст: `send_history_page:876`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 30 ff ff ff` | `<component>.picc=0` | Установить картинку/иконку компонента. Контекст: `send_history_page:893`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 69 63 63 3d 3c 32 34 5f 69 66 5f 65 6e 74 72 79 5f 73 74 61 74 75 73 5f 63 6f 6d 70 6c 3e ff ff ff` | `<component>.picc=<24_if_entry_status_compl>` | Установить картинку/иконку компонента. Контекст: `send_history_page:900`. |
| `74 31 39 2e 74 78 74 3d 22 22 ff ff ff` | `t19.txt=""` | Установить текст компонента. Контекст: `send_history_page:906`. |
| `70 61 67 65 20 30 ff ff ff` | `page 0` | Переключение на page 0: Home/ожидание. Контекст: `send_home_state:242`. |
| `53 74 61 72 74 2e 70 30 2e 70 69 63 3d 3c 70 69 63 3e ff ff ff` | `Start.p0.pic=<pic>` | Установить картинку/иконку компонента. Контекст: `send_home_state:243`. |
| `6e 30 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n0.val=<value>` | Установить numeric value компонента. Контекст: `send_home_state:244`. |
| `6e 31 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n1.val=<value>` | Установить numeric value компонента. Контекст: `send_home_state:245`. |
| `6e 34 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n4.val=<value>` | Установить numeric value компонента. Контекст: `send_home_state:246`. |
| `6e 35 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n5.val=<value>` | Установить numeric value компонента. Контекст: `send_home_state:247`. |
| `62 36 2e 70 69 63 63 3d 3c 70 69 63 3e ff ff ff` | `b6.picc=<pic>` | Установить картинку/иконку компонента. Контекст: `send_home_state:248`. |
| `62 36 2e 70 69 63 63 32 3d 3c 70 69 63 3e ff ff ff` | `b6.picc2=<pic>` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_home_state:249`. |
| `62 35 2e 70 69 63 63 3d 3c 70 69 63 3e ff ff ff` | `b5.picc=<pic>` | Установить картинку/иконку компонента. Контекст: `send_home_state:250`. |
| `62 35 2e 70 69 63 63 32 3d 3c 70 69 63 3e ff ff ff` | `b5.picc2=<pic>` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_home_state:251`. |
| `70 61 67 65 20 34 ff ff ff` | `page 4` | Переключение на page 4: Load/Unload настройка. Контекст: `send_load_unload_page:290`. |
| `70 61 67 65 20 33 ff ff ff` | `page 3` | Переключение на page 3: Move/Temp. Контекст: `send_move_temp_page:317`. |
| `76 69 73 20 74 32 2c 30 ff ff ff` | `vis t2,0` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_move_temp_page:318`. |
| `62 37 2e 70 69 63 63 3d 3c 70 69 63 3e ff ff ff` | `b7.picc=<pic>` | Установить картинку/иконку компонента. Контекст: `send_move_temp_page:321`. |
| `6e 33 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n3.val=<value>` | Установить numeric value компонента. Контекст: `send_move_temp_page:322`. |
| `6e 32 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n2.val=<value>` | Установить numeric value компонента. Контекст: `send_move_temp_page:324`. |
| `70 61 67 65 20 31 38 ff ff ff` | `page 18` | Переключение на page 18: Network. Контекст: `send_network_page:345`. |
| `70 61 67 65 20 36 32 ff ff ff` | `page 62` | Переключение на page 62: Searching Wi-Fi. Контекст: `send_network_page:345`. |
| `74 35 2e 74 78 74 3d 22 49 50 3a 3c 69 70 5f 61 64 64 72 65 73 73 3e 22 ff ff ff` | `t5.txt="IP:<ip_address>"` | Установить текст компонента. Контекст: `send_network_page:353`. |
| `4e 65 74 77 6f 72 6b 2e 62 37 2e 70 69 63 63 32 3d 34 31 ff ff ff` | `Network.b7.picc2=41` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_network_page:354`. |
| `4e 65 74 77 6f 72 6b 2e 70 30 2e 70 69 63 3d 37 31 ff ff ff` | `Network.p0.pic=71` | Установить картинку/иконку компонента. Контекст: `send_network_page:355`. |
| `62 37 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `b7.txt="<text>"` | Установить текст компонента. Контекст: `send_network_page:356`. |
| `70 34 2e 70 69 63 3d 34 32 ff ff ff` | `p4.pic=42` | Установить картинку/иконку компонента. Контекст: `send_network_page:357`. |
| `4e 65 74 77 6f 72 6b 2e 62 38 2e 70 69 63 63 32 3d 34 31 ff ff ff` | `Network.b8.picc2=41` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_network_page:358`. |
| `4e 65 74 77 6f 72 6b 2e 70 31 2e 70 69 63 3d 36 38 ff ff ff` | `Network.p1.pic=68` | Установить картинку/иконку компонента. Контекст: `send_network_page:359`. |
| `62 38 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `b8.txt="<text>"` | Установить текст компонента. Контекст: `send_network_page:360`. |
| `4e 65 74 77 6f 72 6b 2e 62 39 2e 70 69 63 63 32 3d 34 31 ff ff ff` | `Network.b9.picc2=41` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_network_page:361`. |
| `4e 65 74 77 6f 72 6b 2e 70 32 2e 70 69 63 3d 36 38 ff ff ff` | `Network.p2.pic=68` | Установить картинку/иконку компонента. Контекст: `send_network_page:362`. |
| `62 39 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `b9.txt="<text>"` | Установить текст компонента. Контекст: `send_network_page:363`. |
| `4e 65 74 77 6f 72 6b 2e 62 31 30 2e 70 69 63 63 32 3d 34 31 ff ff ff` | `Network.b10.picc2=41` | Установить картинку компонента для второго состояния/нажатия. Контекст: `send_network_page:364`. |
| `4e 65 74 77 6f 72 6b 2e 70 33 2e 70 69 63 3d 36 38 ff ff ff` | `Network.p3.pic=68` | Установить картинку/иконку компонента. Контекст: `send_network_page:365`. |
| `62 31 30 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `b10.txt="<text>"` | Установить текст компонента. Контекст: `send_network_page:366`. |
| `70 61 67 65 20 32 ff ff ff` | `page 2` | Переключение на page 2: активная печать. Контекст: `send_print_page:748`. |
| `50 72 69 6e 74 5f 54 72 75 6e 5f 31 2e 70 30 2e 70 69 63 3d 3c 70 69 63 3e ff ff ff` | `Print_Trun_1.p0.pic=<pic>` | Установить картинку/иконку компонента. Контекст: `send_print_page:749`. |
| `6e 34 2e 76 61 6c 3d 30 ff ff ff` | `n4.val=0` | Установить numeric value компонента. Контекст: `send_print_page:751`. |
| `6e 35 2e 76 61 6c 3d 30 ff ff ff` | `n5.val=0` | Установить numeric value компонента. Контекст: `send_print_page:752`. |
| `6e 37 2e 76 61 6c 3d 30 ff ff ff` | `n7.val=0` | Установить numeric value компонента. Контекст: `send_print_page:753`. |
| `6e 38 2e 76 61 6c 3d 30 ff ff ff` | `n8.val=0` | Установить numeric value компонента. Контекст: `send_print_page:754`. |
| `6e 36 2e 76 61 6c 3d 3c 76 61 6c 75 65 3e ff ff ff` | `n6.val=<value>` | Установить numeric value компонента. Контекст: `send_print_page:755`. |
| `74 38 2e 74 78 74 3d 22 3c 76 61 6c 75 65 3e 22 ff ff ff` | `t8.txt="<value>"` | Установить текст компонента. Контекст: `send_print_page:761`. |
| `74 39 2e 74 78 74 3d 22 3c 76 61 6c 75 65 3e 22 ff ff ff` | `t9.txt="<value>"` | Установить текст компонента. Контекст: `send_print_page:762`. |
| `70 61 67 65 20 37 37 ff ff ff` | `page 77` | Переключение на page 77: Print result. Контекст: `send_print_result_page:798`. |
| `70 72 69 6e 74 5f 64 6f 6e 65 2e 63 70 30 2e 63 6c 6f 73 65 28 29 ff ff ff` | `print_done.cp0.close()` | Закрыть canvas/preview stream перед новой отрисовкой. Контекст: `send_print_result_page:799`. |
| `76 69 73 20 70 72 69 6e 74 5f 64 6f 6e 65 2e 63 70 30 2c 30 ff ff ff` | `vis print_done.cp0,0` | Показать/скрыть компонент: 1 = visible, 0 = hidden. Контекст: `send_print_result_page:800`. |
| `70 72 69 6e 74 5f 64 6f 6e 65 5f 66 6c 61 67 3d 3c 31 5f 69 66 5f 63 6f 6d 70 6c 65 74 65 64 5f 65 6c 73 65 5f 30 3e ff ff ff` | `print_done_flag=<1_if_completed_else_0>` | Установить внутренний флаг страницы/процесса HMI. Контекст: `send_print_result_page:802`. |
| `70 72 69 6e 74 5f 64 6f 6e 65 2e 74 6d 30 2e 65 6e 3d 31 ff ff ff` | `print_done.tm0.en=1` | Экран результата печати. Контекст: `send_print_result_page:803`. |
| `74 32 2e 74 78 74 3d 22 3c 64 61 74 65 74 69 6d 65 3e 5c 6e 22 ff ff ff` | `t2.txt="<datetime>\n"` | Установить текст компонента. Контекст: `send_print_result_page:804`. |
| `74 34 2e 74 78 74 3d 22 30 6d 30 32 73 22 ff ff ff` | `t4.txt="0m02s"` | Установить текст компонента. Контекст: `send_print_result_page:805`. |
| `67 65 74 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 3c 61 74 74 72 3e ff ff ff` | `get <component>.<attr>` | Запрос значения/атрибута компонента у дисплея. Контекст: `send_usb_attr_probe:2034`. |
| `67 65 74 20 3c 70 72 65 66 69 78 3e 3c 69 6e 64 65 78 3e 2e 74 78 74 ff ff ff` | `get <prefix><index>.txt` | Запрос значения/атрибута компонента у дисплея. Контекст: `send_usb_get_text_probe:2011`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 78 3d 3c 78 3e ff ff ff` | `<component>.x=<x>` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2079`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 79 3d 3c 79 3e ff ff ff` | `<component>.y=<y>` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2080`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 77 3d 31 31 35 ff ff ff` | `<component>.w=115` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2081`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 68 3d 32 32 ff ff ff` | `<component>.h=22` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2082`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 66 6f 6e 74 3d 30 ff ff ff` | `<component>.font=0` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2083`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 63 6f 3d 36 35 35 33 35 ff ff ff` | `<component>.pco=65535` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2084`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 62 63 6f 3d 30 ff ff ff` | `<component>.bco=0` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_move_probe:2085`. |
| `67 65 74 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 78 ff ff ff` | `get <component>.x` | Запрос значения/атрибута компонента у дисплея. Контекст: `send_usb_label_move_probe:2089`. |
| `67 65 74 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 79 ff ff ff` | `get <component>.y` | Запрос значения/атрибута компонента у дисплея. Контекст: `send_usb_label_move_probe:2090`. |
| `67 65 74 20 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 ff ff ff` | `get <component>.txt` | Запрос значения/атрибута компонента у дисплея. Контекст: `send_usb_label_move_probe:2091`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 66 6f 6e 74 3d 3c 66 6f 6e 74 3e ff ff ff` | `<component>.font=<font>` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_style_probe:2052`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 70 63 6f 3d 3c 70 63 6f 3e ff ff ff` | `<component>.pco=<pco>` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_style_probe:2053`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 62 63 6f 3d 3c 62 63 6f 3e ff ff ff` | `<component>.bco=<bco>` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_style_probe:2054`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 73 74 61 3d 31 ff ff ff` | `<component>.sta=1` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_style_probe:2055`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 78 63 65 6e 3d 31 ff ff ff` | `<component>.xcen=1` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_style_probe:2056`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 79 63 65 6e 3d 31 ff ff ff` | `<component>.ycen=1` | Настройка геометрии или стиля компонента. Контекст: `send_usb_label_style_probe:2057`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 3c 63 6f 6d 70 6f 6e 65 6e 74 3e 22 ff ff ff` | `<component>.txt="<component>"` | Установить текст компонента. Контекст: `send_usb_row_probe:1973`. |
| `70 61 67 65 20 35 34 ff ff ff` | `page 54` | Переключение на page 54: USB files. Контекст: `send_usb_text_probe:1944`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 3c 74 65 78 74 3e 74 3c 69 6e 64 65 78 3e 22 ff ff ff` | `<component>.txt="<text>t<index>"` | Установить текст компонента. Контекст: `send_usb_text_probe:1947`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 3c 74 65 78 74 3e 62 3c 69 6e 64 65 78 3e 22 ff ff ff` | `<component>.txt="<text>b<index>"` | Установить текст компонента. Контекст: `send_usb_text_probe:1950`. |
| `3c 63 6f 6d 70 6f 6e 65 6e 74 3e 2e 74 78 74 3d 22 3c 74 65 78 74 3e 67 3c 69 6e 64 65 78 3e 22 ff ff ff` | `<component>.txt="<text>g<index>"` | Установить текст компонента. Контекст: `send_usb_text_probe:1953`. |
| `62 31 33 2e 70 69 63 63 3d 38 34 ff ff ff` | `b13.picc=84` | Установить картинку/иконку компонента. Контекст: `set_z_tilt_step:1222`. |
| `62 31 34 2e 70 69 63 63 3d 38 33 ff ff ff` | `b14.picc=83` | Установить картинку/иконку компонента. Контекст: `set_z_tilt_step:1222`. |
| `62 31 32 2e 70 69 63 63 3d 38 34 ff ff ff` | `b12.picc=84` | Установить картинку/иконку компонента. Контекст: `set_z_tilt_step:1223`. |
| `70 61 67 65 20 35 36 ff ff ff` | `page 56` | Переключение на page 56: error text. Контекст: `show_error_page:702`. |
| `74 30 2e 74 78 74 3d 22 3c 74 65 78 74 3e 22 ff ff ff` | `t0.txt="<text>"` | Установить текст компонента. Контекст: `show_error_page:703`. |
| `43 33 5f 73 65 6e 64 5f 66 6c 61 67 3d 3c 76 61 6c 75 65 3e ff ff ff` | `C3_send_flag=<value>` | Установить внутренний флаг страницы/процесса HMI. Контекст: `show_move_alert:682`. |

## Incoming Raw Frames

Это кадры, которые `display_uart_demo_host.py` умеет декодировать от дисплея.

| Hex raw frame | Шаблон | Описание |
|---|---|---|
| `91 ff ff ff` | `0x91` | Сигнал инициализации дисплея; демо использует его как повод заново отправить стартовую последовательность. |
| `65 <page> <component> ff ff ff` | `touch` | Touch-событие: page id и component id. |
| `71 <b0> <b1> <b2> <b3> ff ff ff` | `numeric/get` | Numeric frame. В демо трактуется двояко: либо `get_value` little-endian, либо ввод `page/component/value`. |
| `70 <ascii...> ff ff ff` | `string` | Строковый ответ дисплея на `get ...txt`. |
| `1a ff ff ff` | `status 0x1a` | Статусный ответ дисплея; в снифах встречался и после валидных команд отрисовки. |
| `1c ff ff ff` | `status/error 0x1c` | Ошибка/статус; наблюдался после невалидного id картинки. |
| `<raw bytes> ff ff ff` | `raw` | Нераспознанный кадр, логируется как raw. |

Всего уникальных шаблонов: 189.
