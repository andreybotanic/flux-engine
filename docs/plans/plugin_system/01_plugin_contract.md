# Этап 1: Plugin contract и ABI

## Проверка предпосылок

Перед началом этапа убедиться:
- прочитан `docs/game_overview.md`;
- существует `docs/plans/plugin_system/00_roadmap.md`;
- в roadmap зафиксировано, что целевой плагин - пакет с manifest, DLL, конфигами и ассетами;
- команда работает в чистой постановке: на этом этапе ещё не требуется меню плагинов, save-gate или `xtask`.

Если одна из предпосылок не выполнена, сначала привести roadmap и постановку в согласованное состояние.

## Будущие зоны изменений

Ожидаемые зоны кода на будущем этапе:
- новый модуль `src/plugins/`;
- `Cargo.toml` для зависимостей загрузчика DLL и чтения архива;
- `docs/technical_overview.md`;
- `docs/project_structure.md`;
- тесты нового plugin-модуля.

Не трогать на этом этапе:
- gameplay-логику газа и труб;
- main menu UI;
- save schema;
- `xtask`.

## Что реализовать

1. Ввести базовые ID:
   - `PluginId`;
   - `PluginVersion`;
   - `PluginApiVersion`.

2. Описать manifest плагина:
   - `id`;
   - `display_name`;
   - `version`;
   - `api_version`;
   - `dll`;
   - `configs`;
   - `assets`;
   - `content = true/false`;
   - optional `description`.

3. Зафиксировать формат пакета:

   ```text
   my_plugin.fluxplugin
   |-- manifest.toml
   |-- bin/
   |   `-- my_plugin.dll
   |-- config/
   `-- assets/
   ```

4. Зафиксировать формат expanded dev-папки:

   ```text
   plugins_dev/my_plugin/
   |-- manifest.toml
   |-- bin/
   |   `-- my_plugin.dll
   |-- config/
   `-- assets/
   ```

5. Ввести C ABI, через который ядро вызывает DLL. Через ABI нельзя передавать Bevy-типы, Rust references, generic-типы, `String`, `Vec` или owned Rust-структуры.

   Минимальные функции DLL:
   - `flux_plugin_api_version() -> u32`;
   - `flux_plugin_create(host: *const FluxHostApi, out_plugin: *mut *mut FluxPluginHandle) -> FluxStatus`;
   - `flux_plugin_register(plugin: *mut FluxPluginHandle, registrar: *mut FluxRegistrar) -> FluxStatus`;
   - `flux_plugin_destroy(plugin: *mut FluxPluginHandle)`;

6. Все строки через ABI передавать как UTF-8 pointer + length. Ошибки возвращать как status code + host-readable error buffer.

7. Для Windows сразу заложить правило: DLL загружается не из исходного пути, а из временной generation-копии в cache. Это позволит пересобирать исходную DLL в dev mode.

## Edge cases

- Manifest отсутствует или не парсится.
- `api_version` плагина не совпадает с поддерживаемым API ядра.
- DLL отсутствует.
- DLL экспортирует не все обязательные функции.
- DLL возвращает ошибку при `create` или `register`.
- `PluginId` пустой, содержит пробелы, нестабильный casing или недопустимые символы.
- Архив содержит пути с `..` или absolute paths.
- Два плагина используют один `PluginId`.

## Тесты этапа

Запускать только тесты plugin-contract слоя:
- manifest parser принимает валидный manifest;
- manifest parser отклоняет пустой `id`;
- manifest parser отклоняет несовместимый `api_version`;
- archive path validation запрещает выход из plugin root;
- duplicate `PluginId` отклоняется.

После этапа выполнить `cargo build --release`, затем запустить release-версию и проверить, что приложение стартует как раньше.

## Заметка для заказчика

После этапа разработчик должен дать тебе простой способ увидеть plugin-contract проверку прямо при запуске игры. Минимальная ручная проверка: положить рядом с игрой заранее подготовленный сломанный тестовый plugin package, запустить игру и увидеть в главном меню понятное сообщение, что плагин отклонён из-за manifest/API/DLL ошибки; затем убрать сломанный пакет, запустить игру снова, нажать `New Game` и убедиться, что обычный мир создаётся как раньше. Если результат виден только в коде или unit-тестах, этап нельзя считать полностью готовым для приёмки.

## Критерии успешности

- Есть documented и протестированный contract плагина.
- Ядро умеет прочитать manifest и подготовить загрузку DLL, но ещё не обязано подключать gameplay content.
- Ошибки загрузки представлены структурно, без panic в штатных invalid-plugin сценариях.
- Приложение без внешних плагинов запускается как до этапа.
