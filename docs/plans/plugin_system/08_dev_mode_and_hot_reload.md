# Этап 8: dev mode и hot reload

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 7 завершён;
- `xtask` умеет собирать plugin DLL и expanded output;
- игра умеет загружать packaged и expanded plugins;
- экран `Plugins` уже есть;
- save load gate уже защищает мир от отсутствующего content.

Если ещё нет безопасной unload/reload модели plugin registry, этот этап не начинать.

## Будущие зоны изменений

Ожидаемые зоны кода:
- `src/plugins/reload.rs`;
- `src/plugins/source.rs`;
- `src/plugins/loader.rs`;
- `src/editor/main_menu_actions_block.rs`;
- `src/editor/main_menu_ui_block.rs`;
- `docs/technical_overview.md`;
- `docs/project_structure.md`.

## Что реализовать

1. Ввести dev mode, который грузит плагин из expanded directory:

   ```text
   plugins_dev/<plugin_id>/
   |-- manifest.toml
   |-- bin/<plugin>.dll
   |-- config/
   `-- assets/
   ```

2. Dev mode не должен требовать упаковки `.fluxplugin` после каждого изменения.

3. Изменения кода DLL всё равно требуют пересборки DLL. Для этого использовать `xtask build-plugin <plugin_id>` или обычную сборку plugin crate.

4. Hot reload разрешён только без загруженного мира:
   - `WorldLoadState.has_world == false`;
   - root `Main Menu` или экран `Plugins`;
   - если мир загружен, reload откладывается или блокируется с понятным сообщением.

5. На Windows DLL загружать из cache-копии:

   ```text
   plugin_cache/<plugin_id>/<generation>/<plugin>.dll
   ```

   Это нужно, чтобы исходную DLL можно было перезаписать при следующей сборке.

6. Reload должен быть атомарным:
   - подготовить новый registry snapshot;
   - загрузить новую generation DLL;
   - выполнить register в temporary registry;
   - провалидировать conflicts;
   - только после успеха заменить active registry;
   - при ошибке оставить предыдущую рабочую версию активной.

7. Hot reload должен обновлять:
   - manifest;
   - configs;
   - assets paths;
   - DLL generation;
   - content registry.

8. Не перезагружать world runtime state. Если мир загружен, сначала выйти в main menu без мира.

9. UI MVP остаётся простым: список плагинов и переключатели. Ошибку reload можно показывать через существующий status text, без полноценного log viewer.

## Edge cases

- DLL пересобрана, но manifest не изменился.
- Manifest изменился, но DLL старая.
- Reload нового плагина создаёт duplicate content ID.
- Плагин был enabled, reload сломался: старая версия остаётся активной.
- Плагин был disabled: reload source может обновить metadata, но code не должен стать active.
- Пользователь пытается reload во время загруженного мира.
- Файл DLL копируется в момент, когда сборка ещё не завершилась.

## Тесты этапа

Запускать связанные tests:
- reload success заменяет generation;
- reload failure оставляет старую generation active;
- reload запрещён при `has_world = true`;
- source hash/mtime меняется при изменении manifest/config/DLL;
- disabled plugin не регистрирует content после reload.

Ручная проверка:
- открыть игру в main menu;
- открыть экран `Plugins`;
- пересобрать dev plugin DLL через `xtask`;
- убедиться, что reload подхватывает новую generation без упаковки архива;
- загрузить мир, убедиться, что reload теперь недоступен;
- выйти в main menu без мира и повторить reload.

После этапа выполнить `cargo build --release`, запустить release-версию и проверить логи.

## Заметка для заказчика

Запусти игру с тестовым dev-плагином из `plugins_dev/<plugin_id>/`, открой `Main Menu -> Plugins` и убедись, что он виден как dev-source. Попроси разработчика изменить в expanded folder простую видимую вещь: например label газа, цвет газа или имя плагина, затем нажми reload/rescan в меню без упаковки архива. В списке плагинов или в игре после `New Game` изменение должно появиться. Затем загрузи мир и проверь, что reload теперь недоступен или показывает понятное сообщение: hot reload разрешён только без загруженного мира.

## Критерии успешности

- Dev plugin можно менять как набор отдельных файлов.
- Архив после каждого изменения не нужен.
- Hot reload работает без загруженного мира.
- Ошибка reload не ломает текущую active plugin set.
- Default plugin остаётся включённым и locked.
