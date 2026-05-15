### Редактор: выбор газа через dropdown

- Для `Add Gas` и `Gas Source` переключение газа переведено с циклической кнопки на dropdown/select-подобный control.
- Вся логика dropdown вынесена в отдельный модуль `ui/select_field` (по аналогии с `ui/input_field`): состояние селекта, обработка кликов, синхронизация визуала и сворачивание по клику вне опций.
- Визуальный стиль dropdown для выбора газа приведён к HTML-like `select` (белое поле, ASCII-стрелка справа, список опций с hover/selected-состояниями).
- Список опций рендерится как overlay-слой (`absolute` + повышенный `z-index`) и не увеличивает высоту контента панели.
- Подпись `Source gas` вынесена в отдельную строку над селектом, значение внутри селекта содержит только выбранный газ.
- Стрелка селекта переведена с текста на отдельный sprite-asset; в раскрытом состоянии селекта спрайт переворачивается (`flip_y`).
- При раскрытом селекте клик внутри панели вне кнопки селекта и вне списка опций сворачивает селект.
- Панели с селектами (`Gas Panel`, `Structure Panel`) используют `PanelScrollPolicy::Never`; для не-скроллируемых панелей включён `overflow: visible`, чтобы dropdown мог выходить за границы панели без обрезки.
- При успешной постановке Source/Sink созданная клетка сразу становится выбранной для редактирования.
- Панель настроек структуры показывается только когда реально выбрана существующая структура.
- При отсутствии выбранного инструмента и активном debug-режиме клик по структуре включает её режим редактирования.
- `Esc` в режиме редактирования структуры сначала закрывает режим редактирования, и только затем возвращается к обычной логике `Esc`.

### Редактор труб и моста

- В editor у подпaнели `Gases` теперь три режима: `Pipe`, `Vent`, `Bridge`.
- Для моста добавлен отдельный ресурс `BridgePlacementState`, который хранит rotation ghost-preview.
- Клавиша `R` меняет rotation только у preview ещё не поставленного моста.
- Ghost-preview моста использует отдельный ассет `bridge_silhouette.png` и рисуется размером `192x64` или `64x192` в зависимости от rotation.
- `EraseSolid` теперь очищает `PlacedStructureMap` целиком по клетке: если клетка принадлежит мосту, удаляется весь мост, а pipe-gas в затронутых pipe-node очищается через `PipeGasField`.
- `Scissors` модифицируют набор `pipe_cuts` внутри `PlacedStructureMap`.

### Сохранения

- Версия схемы сохранения повышена до `6`.
- Primary storage для структур теперь один: `placed_structures.bin` (`PlacedStructureSnapshot`).
- В runtime snapshot теперь входят:
  - `PlacedStructureSnapshot`;
  - `PipeGasSnapshot` в node-based формате;
  - обычный `GasFieldSnapshot`;
  - snapshot world-клеток.
- `world_cells.bin` использует binary v2: таблицу stable cell content IDs и per-cell индексы; `Empty` не считается required content.
- `placed_structures.bin` использует binary v2: stable entity content ID для каждой структуры, а `GasSource` params хранят stable `SubstanceId` вместо compact runtime index.
- `pipe_gas.bin` использует binary v2: pipe node kind хранится как stable pipe-container content ID (`flux.default.entity.pipe` или `flux.default.entity.gas_pipe_bridge`), а gas species продолжают мапиться через stable substance IDs.
- `meta.toml` содержит `required_content` с реально использованными `cell/entity/substance/pipe_container` IDs и диагностический `enabled_plugins_at_save`; просто включённые non-content/UI-плагины не блокируют загрузку.
- Перед чтением data chunks `load_save(...)` выполняет load gate: каждый required ID должен быть зарегистрирован активным enabled content-provider plugin-ом, иначе возвращается понятная ошибка `Missing plugin: ...` или `Missing content: ...` без изменения runtime world state.
- В save-meta добавлен отдельный preview-chunk `preview_png` (`preview.png`, формат `png_v1`); `SaveDescriptor` теперь хранит optional `preview_path`.
- `load_save(...)` принимает только текущую schema `6`; старые версии сейвов больше не конвертируются и считаются несовместимыми.
- После `create_save(...)` и `overwrite_save(...)` сначала коммитятся data-chunk-и слота, а затем отдельно ставится в очередь offscreen-capture preview; это позволяет не откатывать сам слот, если превью не удалось записать.
- Перед фактическим screenshot-capture preview-pipeline выдерживает один полный кадр в принудительном `OverlayMode::Main`, чтобы offscreen PNG гарантированно снимался в каноническом `F1`, а не в остаточном `F2/F3`.
- Каноническое preview строится отдельной offscreen-камерой `RenderTarget::Image` размером `512x512` в `F1`-режиме, без UI, с фиксированным охватом всего мира вместе с внешней fade-рамкой.
- Добавлен API удаления слота сохранения (`delete_save`) с валидацией целевого слота.
- Для ручной визуальной проверки pipe-сценариев добавлен служебный бинарник `src/bin/generate_pipe_scenario_saves.rs`: он создаёт пять стартовых сейвов через обычный `create_save(...)`, поэтому format/save-schema у эталонных сценариев полностью совпадает с игровыми слотами.

### Меню, ввод и текст

- Добавлена единая UI-палитра (`ui/palette`) для цветов меню/HUD/панелей/select/input/tooltip.
- Меню переведено в режим полного input-capture:
  - при открытом menu блокируются hotkeys симуляции (`Space`, `,`, `.`), переключение overlay (`F1/F2`), debug-hotkeys и управление камерой.
- Текстовый ввод обновлён на использование `KeyboardInput.text` с fallback на `logical_key`, что сохраняет Unicode-ввод (включая кириллицу) для имени сохранения.
- В `TextInputField` при каждом редактирующем/навигационном клавиатурном действии каретка принудительно «просыпается» (visible + reset blink timer), чтобы позиция курсора сразу читалась даже при вводе пробелов.
- Позиция каретки и hit-test клика в `TextInputField` рассчитываются по точному layout из `ComputedTextBlock.buffer()` (`cosmic-text`): без эвристик и без hardcode ширин символов.
- Для крайних позиций курсора (в т.ч. конец строки) X-координата каретки берётся с boundary реального glyph (`start/end`), чтобы каретка отображалась на границе символа, а не в его середине.
- Добавлен регрессионный unit-тест на trailing-space через `cosmic-text` shaping: строка с пробелом в конце должна иметь большую вычисленную ширину, чем та же строка без завершающего пробела.
- Добавлен project-шрифт `assets/fonts/ui_main.ttf`, который централизованно применяется ко всем UI-текстам через `UiPlugin`.
- Для списков вне panel-системы добавлен общий `ui/scroll_area`-модуль: теперь он является единым владельцем wheel-scroll, drag-thumb и click-on-track логики как для panel-content, так и для save/load списка.
- `ScrollAreaViewport` хранит per-viewport флаги `enabled`, `interaction_group` (`Panel` / `Modal`) и `input_priority`, чтобы modal scroll мог глобально приоритизироваться над panel-scroll, не дублируя математику в panel runtime.
- Карточки save/load собираются отдельным блоком editor UI: primary action (`Load` / `Overwrite`) теперь живёт на root-card hit-area, а `Delete` остаётся отдельной кнопкой внутри карточки, чтобы delete-click не триггерил загрузку слота.

### HUD клетки и курсорная рамка

- Логика сборки HUD-модели вынесена из `src/ui/cell_inspector.rs` в отдельный модуль `src/ui/cell_inspector_model.rs`.
- `cell_inspector_model` строит стек независимых блоков:
  - блок world-клетки для свободного газа;
  - отдельный title-only блок для solid-материала клетки (`Boundary` / `Brick` / `Metal`), если клетка занята;
  - по одному блоку на каждую структуру в наведённой клетке.
- Порядок блоков фиксирован и детерминирован: сначала `world_cell_hud`, затем структуры по `hud.sort_order`, затем по `PlacedStructureId`.
- Газовые строки HUD формируются единым formatter-ом:
  - всегда показывают `Pressure` и `Particles`;
  - скрывают отсутствующие газы;
  - используют dominant-mixture формат при доле лидера `> 90%`;
  - используют `< 0.1%` для очень малых примесей.
- Для `GasPipeBridge` HUD-блок структуры виден на всех занятых клетках, но строки состава внутренней bridge-трубы рендерятся только на центральной клетке через конфиговый режим `visible_on_hover = "container_cell"`.
- Заголовки HUD-блоков структур и твёрдых материалов берутся из общего base-config сущности, а не из HUD-секции.
- Визуально HUD теперь строится как один root-контейнер с общей тенью (`BoxShadow`) и вертикальным стеком дочерних entity-блоков, а не как один текстовый blob.
- Runtime HUD не пересоздаёт блоки на каждый hover/simulation update: он держит заранее созданные UI-слоты и обновляет их текст in-place, чтобы не мерцать на живой симуляции и не запаздывать при быстром движении курсора.
- Курсорная рамка вынесена в отдельный render-helper `src/render/world_view_cursor_highlight_block.rs`: поверх текущей grid-подсветки он рисует внутреннюю белую полупрозрачную пунктирную рамку через `gizmos.line_2d`.

