# FluxEngine Plugin SDK

Эта документация описывает публичный Rust-first runtime SDK FluxEngine (`crates/flux_plugin_sdk`).

Базовый рабочий путь для автора плагина:

- реализовать `Plugin` и объявить entrypoint через `declare_plugin!`;
- зарегистрировать дескрипторы и подписки через `Registrar`;
- обрабатывать typed runtime events (`PluginEvent`) и использовать proxy API (`WorldApi`, `EntityApi`, `GasApi`, `OverlayApi`, `SaveApi`, `TimeApi`, `InputApi`, `LoggerApi`).

Для plugin-controlled overlay используется только graph-пайплайн:

- событие `PluginEvent::RenderOverlay`;
- построение `OverlayGraph`;
- отправка через `OverlayApi::submit_graph`.

Legacy frame-based API больше не является рабочей частью SDK/runtime контракта.

Сгенерированные reference-разделы обновляются командой:

```powershell
cargo xtask generate-plugin-sdk-docs
```

Сайт документации собирается командой:

```powershell
cargo xtask build-plugin-sdk-docs
```
