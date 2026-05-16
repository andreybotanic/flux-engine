# Lifecycle runtime-плагина

Runtime DLL-плагин проходит один и тот же цикл:

1. Движок читает `manifest.toml` и проверяет `api_version = 6`.
2. Движок загружает DLL из безопасной временной копии plugin root.
3. Движок вызывает `flux_plugin_api_version`.
4. Движок вызывает `flux_plugin_create` и получает opaque handle.
5. Движок вызывает `flux_plugin_register`.
6. Во время `register` плагин регистрирует content и подписки через `Registrar`.
7. В runtime движок хранит список подписчиков по `PluginEvent` и вызывает только тех плагинов, которые заранее подписались на событие.
8. Для каждого runtime-события движок вызывает единый `flux_plugin_dispatch`, а SDK внутри DLL маршрутизирует событие в нужный Rust-обработчик.
9. При выгрузке registry или shutdown вызывается `flux_plugin_destroy`.

Важно:

- плагин не должен хранить raw host pointers;
- игровые API живут в полях объекта плагина, но реально работают только внутри активного dispatch scope;
- вызов runtime API вне обработчика должен завершаться ошибкой `PluginError::ApiUnavailable`.