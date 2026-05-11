# Lifecycle runtime-плагина

Runtime DLL-плагин проходит один и тот же жизненный цикл:

1. Движок читает `manifest.toml` и проверяет `api_version`.
2. Движок загружает DLL из безопасной временной копии.
3. Движок вызывает `flux_plugin_api_version`.
4. Движок вызывает `flux_plugin_create` и получает opaque handle.
5. Движок вызывает `flux_plugin_register`, где плагин объявляет capabilities.
6. В `flux_plugin_register` плагин явно объявляет `FluxEventKind -> handler_name` через `register_event_handler`; raw numeric tag остаётся только ABI-деталью.
7. В runtime движок вызывает только те named handler exports, которые плагин зарегистрировал для соответствующих событий.
8. При выгрузке registry или shutdown вызывается `flux_plugin_destroy`.

Плагин не должен хранить указатели на host callback tables дольше текущего вызова. Строки передаются как borrowed UTF-8 slices; владение памятью остаётся у вызывающей стороны.
