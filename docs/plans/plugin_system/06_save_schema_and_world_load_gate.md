# Этап 6: save schema и проверка загрузки мира

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 5 завершён;
- content registry содержит entity/cell/substance IDs;
- default plugin content работает в новом registry;
- текущие газы и структуры имеют стабильные plugin IDs.

Если мир всё ещё сохраняет только enum-коды без ID adapter, этот этап не начинать.

## Будущие зоны изменений

Ожидаемые зоны кода:
- `src/save.rs`;
- `src/save_meta_io_block.rs`;
- `src/save_gas_io_block.rs`;
- `src/save_api_block.rs`;
- `src/save_tests_block.rs`;
- plugin/content registry modules;
- `src/editor/main_menu_actions_block.rs` для отображения ошибки load-gate;
- `docs/technical_overview.md`;
- `docs/project_structure.md`;
- `docs/CHANGELOG.md`.

## Что реализовать

1. Повысить save schema до новой plugin-compatible версии.

2. Старые save schema не мигрировать:
   - `list_saves` может скрывать старые слоты или показывать как unsupported;
   - `load_save` должен явно возвращать ошибку "unsupported schema".

3. В save хранить content IDs, реально использованные миром:
   - cell material IDs, если клетка не empty;
   - placed entity IDs;
   - substance IDs с ненулевым количеством в world gas;
   - substance IDs с ненулевым количеством в pipe gas;
   - storage/container kind IDs, если они являются plugin content.

4. Не сохранять как обязательные:
   - просто включённые плагины без использованного content;
   - UI-only плагины;
   - overlay-only плагины, если save не хранит активный plugin overlay как часть мира.

5. Перед загрузкой мира выполнить `validate_world_content_available`:
   - прочитать manifest/meta save;
   - собрать required content IDs;
   - проверить, что каждый ID зарегистрирован активным enabled plugin;
   - если чего-то не хватает, не менять текущий runtime world state и вернуть понятную ошибку.

6. Ошибка должна назвать missing plugin/content:
   - `Missing plugin: example.mod`;
   - `Missing content: example.mod.entity.magic_pipe`.

7. Если отсутствует non-content plugin, который был включён при сохранении, мир должен загрузиться.

## Edge cases

- Save с plugin ID, который есть, но выключен.
- Save с plugin ID, пакет которого отсутствует.
- Save с substance ID без частиц: не блокировать загрузку из-за пустого вещества.
- Save с неизвестным content ID в chunk-е.
- Ошибка возникает после частичного чтения файлов: runtime state не должен быть изменён.
- Preview save slot есть, но data chunks unsupported.

## Тесты этапа

Запускать только save/plugin-gate tests:
- старые schema отклоняются;
- мир с content выключенного плагина не загружается;
- мир с отсутствующим content-плагином не загружается;
- мир загружается, если отсутствует только non-content plugin;
- validation не меняет текущий world state при ошибке;
- error message содержит missing IDs.

После этапа выполнить `cargo build --release`, запустить release-версию и вручную проверить load screen error.

## Заметка для заказчика

Попроси разработчика подготовить два save-slot: один мир с content из тестового content-плагина и один мир, где при сохранении был включён только non-content/UI-плагин. Запусти игру, отключи content-плагин и попробуй загрузить первый мир: загрузка должна остановиться с понятной ошибкой о missing plugin/content, а текущий мир не должен измениться. Затем включи content-плагин и загрузи тот же мир успешно. После этого удали или отключи non-content плагин и загрузи второй мир: он должен открыться без блокировки.

## Критерии успешности

- Save schema хранит plugin content IDs.
- Load gate предотвращает загрузку мира с отсутствующим required content.
- Non-content plugins не блокируют загрузку мира.
- Старые сейвы не мигрируются и не загружаются молча.
