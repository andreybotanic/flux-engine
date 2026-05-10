# Этап 4: default plugin

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 3 завершён;
- default plugin есть в registry и отображается в меню как locked;
- экран `Plugins` не ломает существующий запуск мира;
- текущая игра всё ещё работает со старым встроенным content.

Если UI или registry нестабильны, перенос content не начинать.

## Будущие зоны изменений

Ожидаемые зоны кода:
- `src/plugins/default_plugin.rs`;
- `src/world/grid.rs`;
- `src/world/structures.rs`;
- `src/config/mod.rs`;
- `src/config/config_loader_block.rs`;
- `src/render/world_view*.rs`;
- `src/editor/*.rs`;
- `src/ui/cell_inspector_model.rs`;
- `docs/technical_overview.md`;
- `docs/project_structure.md`.

## Что реализовать

1. В default plugin зарегистрировать весь текущий content:
   - `Boundary`;
   - `Brick`;
   - `Metal`;
   - `Pipe`;
   - `Vent`;
   - `GasSource`;
   - `GasSink`;
   - `GasPipeBridge`;
   - текущие overlay-режимы `Main`, `Gas`, `Pipes`;
   - текущие HUD descriptors.

2. Ввести стабильные content IDs:
   - `flux.default.cell.boundary`;
   - `flux.default.cell.brick`;
   - `flux.default.cell.metal`;
   - `flux.default.entity.pipe`;
   - `flux.default.entity.vent`;
   - `flux.default.entity.gas_source`;
   - `flux.default.entity.gas_sink`;
   - `flux.default.entity.gas_pipe_bridge`;
   - `flux.default.overlay.main`;
   - `flux.default.overlay.gas`;
   - `flux.default.overlay.pipes`.

3. Сначала сделать adapter-слой между текущими enum и новыми IDs. Не пытаться удалить все enum за один проход.

4. Перенести descriptors в registry:
   - слои и collision rules;
   - footprint;
   - allowed rotations;
   - sprite metadata;
   - HUD metadata;
   - storage descriptors.

5. Проверить, что поведение не меняется:
   - труба всё ещё может быть внутри стены, кроме boundary;
   - вентиляция работает только на клетке с pipe node;
   - мост остаётся `3x1/1x3`;
   - HUD показывает те же блоки;
   - `F1/F2/F3` работают как раньше.

## Edge cases

- Content ID отсутствует в default plugin registry.
- Старый enum и новый ID расходятся.
- Мост поворачивается, но sprite size и footprint считаются по разным контрактам.
- Render order отличается от текущего.
- HUD sort order поменялся.
- Default plugin случайно зависит от внешнего plugin source.

## Тесты этапа

Запускать связанные тесты:
- `world::structures` descriptor tests;
- config loader tests для default plugin descriptors;
- render helper tests для размеров/overlay visibility;
- HUD model tests;
- editor placement tests, если они есть или будут добавлены.

После этапа выполнить `cargo build --release`, запустить release-версию и вручную проверить:
- создать новый мир;
- поставить кирпич, металл, pipe, vent, bridge, source, sink;
- переключить `F1/F2/F3`;
- открыть HUD над pipe/bridge/solid.

## Заметка для заказчика

Запусти игру, нажми `New Game` и проверь обычный набор MVP-механик: поставь `Brick`, `Metal`, трубу, вентиляцию, мост, `Gas Source` и `Gas Sink`; переключи `F1/F2/F3`; наведи курсор на стену, трубу и мост, чтобы увидеть HUD-блоки. Важно: всё это теперь должно работать через locked default plugin, но для тебя как игрока поведение должно остаться прежним. Если хотя бы один старый инструмент пропал или ведёт себя иначе, этап не готов.

## Критерии успешности

- Текущий content зарегистрирован через default plugin.
- Игровое поведение не изменилось.
- Default plugin остаётся обязательным и не отключаемым.
- После этапа можно продолжать миграцию substance/save без ломки текущего MVP.
