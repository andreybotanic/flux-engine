# Этап 2: Registry и state плагинов

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 1 завершён;
- есть manifest/ABI contract;
- ошибки чтения manifest и загрузки DLL представлены как обычные validation errors;
- приложение всё ещё стартует без внешних плагинов.

Если contract ещё нестабилен, этот этап не начинать.

## Будущие зоны изменений

Ожидаемые зоны кода:
- `src/plugins/registry.rs`;
- `src/plugins/source.rs`;
- `src/plugins/state.rs`;
- `src/app/mod.rs` для подключения plugin bootstrap;
- `config/` или отдельный runtime-файл настроек включённых плагинов;
- `docs/technical_overview.md`;
- `docs/project_structure.md`.

Не трогать на этом этапе:
- main menu UI;
- save schema;
- рендер и editor tools;
- газовую симуляцию.

## Что реализовать

1. Ввести runtime resources:
   - `PluginSourceRegistry`: найденные packaged и dev-плагины;
   - `LoadedPluginRegistry`: успешно загруженные plugin instances;
   - `EnabledPluginSet`: какие плагины включены;
   - `ContentRegistry`: общий реестр контента, который плагины смогут наполнять в следующих этапах.

2. Поддержать два источника:
   - packaged archives из `plugins/*.fluxplugin`;
   - expanded dev folders из `plugins_dev/<plugin_id>/`.

3. Ввести правило приоритета:
   - если один `PluginId` найден и как archive, и как dev folder, dev folder выигрывает только при включённом dev mode;
   - без dev mode использовать packaged archive;
   - duplicate packaged archives с одним `PluginId` считаются ошибкой.

4. Ввести default plugin как обязательную запись registry:
   - `PluginId = "flux.default"`;
   - `enabled = true`;
   - `locked = true`;
   - `content = true`.

5. Сохранять пользовательский `EnabledPluginSet` отдельно от save-файлов мира. Настройки включения не должны лежать внутри конкретного мира.

6. Если плагин есть в настройках enabled set, но его пакет отсутствует, показывать его как missing в registry state. На этом этапе это ещё не UI, но состояние должно быть доступно.

## Edge cases

- Default plugin отсутствует во внешних папках: это нормально, он встроенный.
- Пользовательский config пытается отключить default plugin: игнорировать и логировать warning.
- Плагин был включён, но файл удалён.
- Плагин найден, но manifest сломан.
- Dev folder и archive имеют одинаковый `PluginId`.
- Плагин `content = false` не должен попадать в content registry как поставщик world content.

## Тесты этапа

Запускать только тесты plugin registry/state:
- default plugin всегда enabled и locked;
- попытка отключить default plugin не меняет state;
- dev source имеет ожидаемый приоритет только в dev mode;
- missing enabled plugin сохраняется в состоянии как missing;
- duplicate `PluginId` даёт validation error.

После этапа выполнить `cargo build --release`, запустить release-версию и проверить логи.

## Заметка для заказчика

После этапа ты должен иметь возможность запустить игру без внешних плагинов и увидеть в главном меню видимую диагностику вроде `Loaded plugins: flux.default` или `Default plugin: enabled/locked`. Затем разработчик должен дать простой тестовый plugin folder/package: если положить его в папку плагинов и перезапустить игру, диагностика должна показать, что плагин найден и в каком он состоянии (`enabled`, `disabled`, `missing` или `error`). После этого нажми `New Game`: мир должен создаться, потому что registry-этап не должен ломать текущий gameplay.

## Критерии успешности

- Приложение умеет обнаруживать plugin sources без изменения gameplay.
- Default plugin существует как обязательный registry item.
- Есть единая модель enabled/disabled/missing/error.
- Все текущие игровые механики продолжают работать старым путём.
