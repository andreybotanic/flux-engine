# Обзор системы плагинов

FluxEngine поддерживает runtime-плагины, которые обнаруживаются на старте или через `Reload` до загрузки мира.

Поддерживаются два источника:

- packaged archives `plugins/*.fluxplugin`;
- expanded dev directories `plugins_dev/<plugin_id>/`.

Встроенный `flux.default` всегда включён, заблокирован и предоставляет базовый content игры. Внешние плагины могут регистрировать свой content и подписки во время handshake.

Текущая стабильная внешняя граница — Rust-first SDK v6:

- публичный SDK живёт в `crates/flux_plugin_sdk`;
- внутренний ABI/glue живёт в `crates/flux_plugin_abi`;
- пользовательский код плагина реализует `Plugin`, вызывает `declare_plugin!(...)`, регистрирует content через `Registrar` и подписывается на `PluginEvent`;
- обработчики получают только typed event, а игровые API доступны через поля самого объекта плагина (`self.world`, `self.entities`, `self.gases`, `self.time` и так далее).

Публичный код плагина больше не должен работать напрямую с `extern "C"`, `FluxRuntimeHost`, `FluxPluginHandle`, `FluxStatus` и named handler exports.