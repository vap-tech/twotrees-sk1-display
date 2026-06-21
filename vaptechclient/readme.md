### Примерная архитектура.
HMI task       -> AppEvent::Hmi(...)
Moonraker task -> AppEvent::Moonraker(...)
Timer task     -> AppEvent::Tick
Thumbnail task -> AppEvent::ThumbnailReady(...)

                 ↓

              app loop
                 ↓

              state
                 ↓

              renderer
                 ↓

              commands
То есть задумывал прям backend-like архитектуру: producer → event bus → reducer/state → side effects.
