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
