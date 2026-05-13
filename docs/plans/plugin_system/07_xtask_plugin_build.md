# Этап 7: сборка плагинов через xtask

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 6 завершён;
- plugin package layout и manifest уже зафиксированы;
- default plugin и внешние content IDs используют один contract;
- save schema уже умеет ссылаться на plugin content IDs.

Если формат пакета ещё меняется, `xtask` не начинать.

## Будущие зоны изменений

Ожидаемые зоны кода и файлов:
- новый workspace member `xtask/`;
- root `Cargo.toml`;
- `xtask/src/main.rs`;
- базовая папка `src/plugins/<plugin_crate>/` для in-project исходников плагинов;
- build output folders `target/plugins/` или `dist/plugins/`;
- `docs/technical_overview.md`;
- `docs/project_structure.md`.

## Что реализовать

1. Добавить `xtask` как отдельный crate в workspace.

2. Минимальные команды:
   - `cargo xtask build-plugin <plugin_id>`;
   - `cargo xtask pack-plugin <plugin_id>`;
   - `cargo xtask build-all-plugins`.

3. Если в окружении нет cargo alias `cargo xtask`, документировать fallback:

   ```powershell
   cargo run -p xtask -- build-plugin <plugin_id>
   cargo run -p xtask -- pack-plugin <plugin_id>
   cargo run -p xtask -- build-all-plugins
   ```

4. `build-plugin` должен:
   - найти исходники плагина;
   - собрать DLL;
   - скопировать DLL, manifest, config и assets в expanded output;
   - провалидировать manifest.

5. `pack-plugin` должен:
   - выполнить build при необходимости;
   - создать `.fluxplugin`;
   - не включать временные файлы, `target`, editor cache или secrets;
   - проверить, что archive paths не выходят за root.

6. `build-all-plugins` должен пройти по всем plugin projects и собрать/упаковать их в детерминированном порядке.

7. Команды должны возвращать non-zero exit code при ошибке, чтобы их можно было использовать в CI.

8. Для приёмки этапа нужен минимальный видимый content-плагин:
   - завести sample crate в `src/plugins/flux_stage7_sample_content_plugin`;
   - выставить в manifest `content = true` и текущий `api_version`;
   - через ABI registrar зарегистрировать хотя бы один новый газ;
   - проверить, что после сборки, упаковки и включения плагина газ появляется в игровых dropdown.

## Edge cases

- Plugin ID неизвестен.
- Manifest есть, но DLL после сборки отсутствует.
- Плагин собирается, но `api_version` не совпадает с engine API.
- Asset path в manifest указывает на отсутствующий файл.
- Два plugin projects объявляют один `PluginId`.
- На Windows старый DLL файл занят запущенной игрой. `xtask` должен писать в build output, а игра должна грузить generation-копии.

## Тесты этапа

Запускать связанные tests для `xtask`:
- manifest validation из `xtask`;
- package file list не содержит запрещённых путей;
- unknown plugin ID возвращает ошибку;
- build-all сортирует plugin IDs стабильно.
- sample content plugin после `cargo run -p xtask -- build-all-plugins` попадает в `target/plugins/packages/` и проходит runtime validation.

После этапа выполнить:
- `cargo build --release`;
- `cargo run -p xtask -- build-all-plugins`;
- запустить release-версию и проверить, что игра стартует с собранными plugin artifacts.

Если `cargo` не найден в `PATH`, использовать `C:\Users\andreybotanic\.cargo\bin\cargo.exe`.

## Заметка для заказчика

После этапа разработчик должен показать тебе короткий сценарий: запустить `cargo run -p xtask -- build-plugin <test_plugin>` и `cargo run -p xtask -- pack-plugin <test_plugin>`, затем положить собранный `.fluxplugin` в папку плагинов и запустить игру. В `Main Menu -> Plugins` тестовый плагин должен появиться в списке; после включения и `New Game` должен быть виден хотя бы один простой результат этого плагина, например новый газ в dropdown или новый тестовый объект в доступном UI. Если пакет собирается, но в игре невозможно увидеть, что он подключился, этап для приёмки неполный.

## Критерии успешности

- Плагины можно собирать из этого репозитория одной командой.
- Плагины можно упаковывать в `.fluxplugin`.
- Ошибки сборки понятны и пригодны для CI.
- Runtime loading contract не дублируется в `xtask`, а переиспользует общую validation-логику там, где это возможно.
