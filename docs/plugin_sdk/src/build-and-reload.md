# Сборка и dev reload

Внутрипроектные sample plugin crates собираются через `xtask`:

```powershell
cargo xtask build-plugin flux.sample_content
cargo xtask pack-plugin flux.sample_content
cargo xtask build-plugin flux.sample_content --dev
```

`--dev` устанавливает expanded package в `plugins_dev/<plugin_id>`. После этого в игре можно открыть `Main Menu -> Plugins` и нажать `Reload`, пока мир ещё не загружен.

Plugin SDK docs собираются отдельно:

```powershell
cargo xtask check-plugin-sdk-docs
cargo xtask build-plugin-sdk-docs
```

Reference-примеры для generated API страниц лежат отдельно в `docs/plugin_sdk/src/examples/{methods,events,constants}/`. Если вы добавляете новый SDK-метод, callback, константу или событие, нужно не только задокументировать Rust тип, но и положить сюда соответствующий markdown-snippet, иначе `check-plugin-sdk-docs` завершится ошибкой.
