# Этап 9: plugin overlay scene graph и compositing

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 8 завершён;
- SDK v5 уже активен и внешний/встроенный plugin runtime работает через единый dispatch;
- `RenderOverlayEvent` стабильно приходит в runtime plugins;
- current `F3/Pipes` работает как baseline и покрыт regression-тестами.

Если в проекте ещё остались разрозненные special-case пути рендера плагинов мимо plugin dispatch, этот этап не начинать.

## Ключевые правила этапа

1. `CoreWorld` не должен знать про pipe/bridge/vent или другой content конкретного плагина.
2. В scene graph убираем узлы `RenderSpaceNode` и `RenderTintFieldNode`.
3. Не добавляем отдельный `BackgroundNode`: фон можно рисовать через `RenderImageNode`.
4. Для plugin-owned графики используем один универсальный `RenderImageNode`:
   - одиночные спрайты/иконки;
   - набор инстансов;
   - одна большая grid-aligned карта (например температурная).
5. Селекторы сущностей должны работать через `ContentId` и `ContentTag`, чтобы один плагин мог использовать сущности другого при совпадении тегов.

## Будущие зоны изменений

Ожидаемые зоны кода:
- `crates/flux_plugin_sdk/src/descriptors.rs`;
- `crates/flux_plugin_sdk/src/api.rs`;
- `crates/flux_plugin_sdk/src/events.rs` (если потребуется уточнение payload для overlay);
- `src/plugins/api/render_api.rs`;
- `src/render/world_view_overlay_block.rs` (или новый модуль compositor, если выносится);
- `src/plugins/default_plugin/*` (адаптация F3/Pipes к новому контракту);
- `docs/technical_overview.md`;
- `docs/project_structure.md`;
- `docs/CHANGELOG.md`.

## Что реализовать

1. Ввести typed-модель overlay graph в SDK.
   - Добавить структуры `OverlayGraph`, `OverlayNodeId`, `OverlayNode`.
   - Граф — именованный DAG: у узлов уникальные ID, зависимости перечисляются явно.
   - Добавить валидацию DAG (нет циклов, нет ссылок на несуществующие узлы, нет дублей ID).
   - Критерий успеха: плагин может описать граф декларативно и получить детерминированный порядок исполнения.

2. Зафиксировать минимальный набор узлов.
   - `RenderEntitiesNode`.
   - `RenderFreeGasNode`.
   - `RenderImageNode`.
   - `BlendNode`.
   - `MaterialNode`.
   - Критерий успеха: отсутствуют `RenderSpaceNode`, `RenderTintFieldNode`, `BackgroundNode`.

3. Реализовать entity selector с поддержкой тегов.
   - Добавить отдельный typed ID для тегов, например `ContentTag` (валидация как у `ContentId`: canonical lowercase namespace path).
   - Добавить selector-выражения:
     - `ByIds(Vec<ContentId>)`;
     - `ByAnyTag(Vec<ContentTag>)`;
     - `ByAllTags(Vec<ContentTag>)`;
     - `Not(Box<OverlaySelectorExpr>)`;
     - `And(Vec<OverlaySelectorExpr>)`;
     - `Or(Vec<OverlaySelectorExpr>)`.
   - Критерий успеха: плагин может выбрать сущности другого плагина только по публичным ID/тегам без core-knowledge.

4. Реализовать `RenderImageNode` как универсальный источник plugin-графики.
   - Узел принимает список `OverlayImageInstance`.
   - Каждый инстанс содержит:
     - ссылку на ресурс изображения (`asset id`/`content id`);
     - placement;
     - tint/alpha;
     - blend hint (если нужен локальный override).
   - Поддержать два placement-режима:
     - `CellLocal`: рисунок внутри одной клетки;
     - `GridLocal`: рисунок в координатах клеточной сетки (включая дробные смещения), без ручного пересчёта через камеру.
   - Критерий успеха: можно рисовать иконки/пакеты и одну большую карту температур без перехода в screen-space математику.

5. Добавить compositing-узлы.
   - `BlendNode`: смешивает входы по режиму (`Normal`, `Add`, `Multiply`, `Screen`, `Overlay`, `SoftLight`, `HardLight`, `Custom` при поддержке движка).
   - `MaterialNode`: применяет material/shader к входному изображению или набору входов.
   - Критерий успеха: `F3/Pipes` можно собрать через graph+material без старого special-case shader path.

6. Переделать runtime overlay pipeline на граф.
   - `RenderOverlayEvent` должен инициировать исполнение graph для активного plugin overlay.
   - Исполнение графа детерминированно:
     - топологическая сортировка;
     - одинаковые tie-break правила (по node id) для стабильности между запусками.
   - Критерий успеха: один runtime path для overlay от любых plugins, без ручных веток “если F3”.

7. Сформировать эталонные графы для двух сценариев.
   - Pipe overlay:
     - затемнение мира (через `RenderEntitiesNode + selector` или `MaterialNode`);
     - яркая подсветка pipe-like сущностей через selector+material;
     - газ в трубах через `RenderImageNode` (grid-local инстансы);
     - движущиеся пакеты через `RenderImageNode` (cell/grid-local);
     - иконки входов/выходов через `RenderImageNode`.
   - Temperature overlay:
     - базовый фон через `RenderImageNode`;
     - отдельная большая температурная карта для пустых клеток (`RenderImageNode`, grid-aligned raster);
     - слой твёрдых клеток через `RenderEntitiesNode` + альтернативные спрайты/material;
     - смешивание температурной карты поверх solids через `BlendNode`/`MaterialNode`;
     - слой сущностей и их температурный микс аналогично.
   - Критерий успеха: температурный оверлей не требует “одна нода на каждую клетку/сущность”.

8. Валидация и диагностика.
   - Проверки на этапе register/load:
     - DAG-корректность;
     - корректность селекторов и ссылок на теги;
     - отсутствие unsupported blend/material режимов.
   - Понятные ошибки в plugin diagnostics и startup log.
   - Критерий успеха: невалидный граф не валит игру, а аккуратно помечает plugin error.

## Edge cases

- Селектор по тегу не находит сущности ни в одном включённом плагине.
- Один и тот же entity подходит под несколько selector-веток и может быть нарисован дважды.
- Плагин передал граф с циклом.
- `RenderImageNode` содержит `GridLocal` инстанс, выходящий за границы мира.
- Плагин использует material, который недоступен на текущем backend.
- Активный overlay отключён/удалён в runtime, пока граф уже собран.

## Тесты этапа

Запускать только связанные тесты:
- unit-тесты SDK:
  - валидация `OverlayGraph` (DAG, duplicate IDs, missing deps);
  - валидация `ContentTag` и selector-выражений;
  - сериализация/десериализация descriptor-ов overlay графа.
- render/runtime тесты:
  - deterministic topological execution order;
  - корректная отрисовка `RenderImageNode` в `CellLocal` и `GridLocal`;
  - корректный `BlendNode` порядок (нижний/верхний слой не путается);
  - fallback/error path при недоступном material.
- regression по `F3/Pipes`:
  - визуальная подсветка труб;
  - видимость иконок vent/bridge;
  - отображение статических/движущихся gas пакетов;
  - deterministic кадр при одинаковом состоянии мира.

Ручная проверка:
- запустить игру;
- включить `F3/Pipes` и проверить, что визуально поведение совпадает с baseline;
- переключить overlay туда-сюда (`F1/F2/F3`) и убедиться, что нет артефактов;
- проверить температурный overlay prototype: карта температуры накладывается корректно, без “разрыва” в позиционировании при движении/зуме камеры.

После этапа выполнить `cargo build --release`, запустить release-версию, проверить startup/runtime логи и закрыть приложение.

## Заметка для заказчика

Запусти игру и проверь два сценария:
- `F3/Pipes`: трубы читаются на затемнённом фоне, видны пакеты газа и иконки входов/выходов, всё стабильно при паузе и при движении камеры;
- температурный overlay: температура видна как цельная карта, которая остаётся привязанной к world grid при любом зуме и панорамировании.

Если оба сценария собираются plugin-графом без специальных веток в core render коде — этап принят.

## Критерии успешности

- Overlay rendering для plugin overlay построен как named DAG.
- Узлы `RenderSpaceNode`, `RenderTintFieldNode`, `BackgroundNode` отсутствуют.
- `RenderImageNode` покрывает и одиночные/batch-инстансы, и большие grid-aligned карты.
- Entity selection работает через `ContentId` и `ContentTag`.
- `F3/Pipes` собирается через новый graph/compositor path без special-case веток.
- Температурный overlay выражается существующими узлами через render + blend/material, без генерации тысяч узлов “по одной клетке”.
