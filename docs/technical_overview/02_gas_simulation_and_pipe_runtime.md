## Симуляция газа: текущее состояние MVP

- Модель газа дискретная particle-based (целочисленное хранение частиц в клетках).
- Перенос выполняется локально между соседними клетками (окрестность фон Неймана).
- Сохранение массы каждого газа является обязательным инвариантом.
- Плавучесть учитывается локально на основе окружающей смеси, чтобы поддерживать разделение лёгких и тяжёлых газов.
- Для игрока доступны разные режимы визуализации газов, а для разработки — диагностические метрики и отладочные панели.
- Ядро симуляции отвечает только за свободный газ в world-клетках: particle storage, CPU step, GPU step, buoyancy и CPU/GPU parity.
- Content-bound логика труб не является частью ядра: pipe pressure, pipe gas storage, pipe flux, pipe solver, pipe visuals и pipe tests принадлежат locked default plugin `flux.default`.

### Единый реестр структур и слои

- Основной runtime-ресурс для размещаемых объектов один: `PlacedStructureMap`.
- `PlacedStructureMap` является generic storage-слоем: он индексирует placed structures по `PlacedStructureId` и generic `StructureKind`, не по enum-вариантам default content.
- Набор игровых структур, доступных в текущей сборке (`Pipe`, `Vent`, `GasPump`, `GasSource`, `GasSink`, `GasPipeBridge`), поставляется locked default plugin-ом `flux.default` через stable IDs, descriptors и facade-предикаты.
- Каждая размещённая структура описывается через `PlacedStructure { id, kind, origin, rotation, params }`, где `kind` является generic content-id wrapper-ом.
- Для описания внешнего вида и ограничений используется общий descriptor-контракт из `src/world/structures.rs`:
  - `LayerKind`,
  - `LayerCellSpec`,
  - `StructureDescriptor`,
  - `LayerCollisionKind`.
- `StructureDescriptor` теперь также является единым источником правды для размеров объекта в клетках через `size_in_cells()`.
- Для appearance-метаданных добавлен отдельный конфиг-реестр `src/plugins/default_plugin/config/structures/*.toml`: он хранит базовый `label`, `draw_priority` и `size_in_cells` для всех встроенных стен и структур.
- HUD-метаданные контейнеров тоже вынесены в конфиги:
  - `src/plugins/default_plugin/config/cell_types.toml` хранит `world_cell_hud` для свободного газа клетки;
  - `src/plugins/default_plugin/config/structures/*.toml` хранят верхнеуровневый `label` сущности и `[hud]`-секции только с `sort_order` и списком substance-контейнеров.
- Модуль `src/config/hud.rs` хранит только типы runtime-конфигов HUD и не содержит встроенных fallback-конфигов для конкретных сущностей.
- Для визуализации отдельно зафиксирован `sprite size in cells`: в большинстве случаев он совпадает с footprint, но у моста базовый спрайт всегда считается горизонтальным `3x1`, а вертикальный вариант получается только поворотом transform-а без растяжения.
- В текущем наборе структур это означает:
  - `1x1` для обычных одноклеточных объектов (`Pipe`, `Vent`, `GasSource`, `GasSink`, world solids);
  - `2x1` или `1x2` для `GasPump`;
  - `3x1` или `1x3` только для `GasPipeBridge`.
- Мирные `Solid`-клетки не переводились в `PlacedStructureMap`, но используют тот же descriptor-подход через `cell_material_descriptor(...)`, чтобы placement-check и рендер использовали одинаковые правила слоёв.
- Core отвечает за generic geometry/collision/snapshot операции, а default-specific predicates, IDs, sort order и descriptor metadata берутся через facade `flux.default`. Старые convenience helper-ы для pipe/source/sink/bridge пока сохранены как compatibility API и делегируют на default plugin facade.
- Коллизии по слоям проверяются только по `Special`-клеткам слоя; `RenderOnly` участвует только в визуализации.
- Тип источника вещества для HUD описывается конфигом, а не хардкодом по `StructureKind`: сейчас поддержаны `world_cell` и `pipe_node`, но контракт сразу рассчитан на будущие контейнеры не только для газов.
- Это даёт контролируемое наложение объектов:
  - труба может проходить через стену;
  - мост может визуально пересекать другие объекты;
  - вентиляция и край моста конфликтуют именно потому, что оба занимают special-клетку слоя `GasPipeConnections`.

### Source / Sink / Vent / Pump / Bridge как структуры

- `GasSource` и `GasSink` теперь тоже хранятся в `PlacedStructureMap`, а их параметры живут в `StructureParams`.
- Редактируемыми считаются только `GasSource` и `GasSink`; для этого есть helper `editable_structure_at(...)`.
- `Vent` хранится как самостоятельная структура и использует два слоя:
  - `Appearance`;
  - `GasPipeConnections`.
- `GasPump` хранится как самостоятельная поворотная структура `2x1`:
  - в `GasPipeConnections` входная клетка маркируется `gas_in`, выходная — `gas_out`;
  - pump-порты участвуют в pipe-overlay так же, как другие pipe-порты;
  - насос не создаёт собственного внутреннего контейнера газа, а переносит газ между уже существующими pipe-node.
- `GasPipeBridge` — первая многоклеточная структура:
  - размещение `1x3` или `3x1`;
  - rotation через `StructureRotation`;
  - спрайт `bridge.png` + силуэт `bridge_silhouette.png`;
  - в `Appearance` все три клетки — `RenderOnly`;
  - в `GasPipeConnections` special-маркеры стоят только на крайних клетках.
- Выделение и удаление моста работают целиком по `PlacedStructureId`, даже если игрок кликнул по средней или крайней клетке.

### Node-based pipe runtime

- Трубная подсистема является runtime-частью locked default plugin `flux.default`, потому что трубы, вентиляции и мосты являются plugin-owned content.
- Трубная подсистема больше не хранит газ по схеме `one cell = one buffer`.
- `PipeGasField` теперь хранит набор pipe-узлов через стабильные ключи `PipeNodeKey`:
  - `PipeContainerKind::Pipe`,
  - `PipeContainerKind::BridgePipe`.
- Один node соответствует одному pipe-объёму:
  - обычная труба создаёт node в своей клетке;
  - мост создаёт отдельный bridge-node, визуально принадлежащий центральной клетке моста.
- Благодаря этому в одной клетке теперь корректно сосуществуют:
  - `world gas`,
  - обычная труба,
  - внутренняя труба моста.
- `PipeGasField` хранит только `u32`-счётчики частиц по видам газа и больше не имеет pipe-capacity или clamp-а на уровень сегмента.
- `PipeGasField::sync_to_structures(...)` перестраивает node-layout по текущему `PlacedStructureMap`, сохраняя уже накопленный газ по стабильным ключам.
- `PipeGasSnapshot` сериализуется как список node-ов, а не как flat-массив по world-клеткам.
- Инспектор клетки не держит собственного списка видов газа: все названия и порядок обхода берутся из общего `GasRegistry`, поэтому HUD автоматически подхватывает новый состав registry без UI-правок.
- Давление считается отдельными helper-ами из `src/plugins/default_plugin/pipe_runtime/pressure.rs`:
  - world: `particles * cell_particle_pressure_pa`;
  - pipe: `particles * cell_particle_pressure_pa * cell_volume_ratio`.
- При дефолтных настройках `cell_particle_pressure_pa = 0.2` и `cell_volume_ratio = 25.0`, поэтому `1 Pa` в world соответствует `5` частицам, а `1` частица в трубе создаёт `5 Pa`.
- Runtime-состояние pipe-графа хранится в `PipeFluxField` как служебный topology-signature (без самостоятельного pressure-flux решения); состояние не сериализуется в save и сбрасывается при `New Game`, `Load` и изменении pipe-топологии.
- Канонические test/save fixtures для труб вынесены в `src/plugins/default_plugin/pipe_runtime/scenarios.rs`, чтобы тесты и `generate_pipe_scenario_saves` использовали один и тот же builder.

### Pipe runtime default plugin и граф сети

- `apply_pipe_network_step(...)` живёт в `src/plugins/default_plugin/pipe_runtime.rs` и получает `PlacedStructureMap`, а не legacy `PipeGrid`.
- Граф pipe-сети строится runtime-ом из descriptors/structure-footprints:
  - соседние обычные трубы соединяются по ортогональному соседству;
  - вентиляция подключается только к pipe-node своей клетки;
  - насос публикует два порта (`PumpIn`/`PumpOut`) на своих клетках, но не создаёт внутреннего pipe-ребра между ними;
  - мост подключается наружу только через крайние клетки;
  - автоматического same-cell соединения между pipe под мостом и bridge-node нет.
- Ножницы (`pipe_cuts`) теперь являются частью `PlacedStructureMap`; они режут только внешние соединения между соседними pipe-node и не ломают внутреннюю топологию моста.
- Drag-построение обычных труб тоже управляет `pipe_cuts`: stroke-API в `PlacedStructureMap` снимает cut только между соседними клетками самой линии и, наоборот, ставит cut между новыми клетками штриха и боковыми pipe-соседями вне линии. Благодаря этому вертикальный штрих между двумя горизонтальными трубами не склеивает их автоматически, но повторный проход по уже существующей линии может явно восстановить нужное соединение.
- Редактирование `GasSource/GasSink` не должно помечать `PlacedStructureMap` изменённым без реального изменения параметров: editor UI сравнивает желаемые значения с текущими, а `update_gas_source/update_gas_sink` дополнительно делают no-op на одинаковых параметрах. Это предотвращает лишний respawn pipe-visual entity и потерю видимости `F3`-индикаторов.
- Conveyor transport-часть pipe runtime вынесена в `src/plugins/default_plugin/pipe_runtime/solver.rs`, а `src/plugins/default_plugin/pipe_runtime.rs` оставлен фасадом storage/visual API.
- CPU-алгоритм pipe pre-step внутри default plugin остаётся эталоном; parity smoke-тест для runtime-пути обновлён под новую node-based модель.

### Pre-step структур default plugin

- Built-in SDK runtime `flux.default` выполняет pre-step фазу до core gas step через общий plugin dispatch:
  - `Source` добавляет выбранный газ в свою клетку;
  - `Sink` удаляет газ пропорционально долям газов в клетке, с полным удалением при нехватке массы.
- Пропорциональное удаление реализовано детерминированно целочисленно (largest remainder + стабильный tie-break по индексу газа).
- Одинаковая pre-step логика используется перед runtime CPU и runtime GPU путями (GPU получает состояние после pre-step через upload).
- Та же pre-step логика применена в debug one-step (`Enter`) и parity-сценариях.

### Pipe pre-step в default plugin runtime

- `flux.default` выполняет pipe step `apply_pipe_network_step(...)` до основного шага свободного газа, но уже не как отдельный special-case Bevy runtime plugin. Core system только эмитит общий `SimulationPreCellGasStep`, после чего unified runtime registry dispatch-ит событие в built-in SDK plugin instance.
- Этот шаг одинаково вызывается:
  - в runtime CPU backend;
  - в runtime GPU backend до upload состояния;
  - в debug one-step;
  - в parity smoke/regression тестах.
- В текущем MVP pipe-логика реализована единым CPU-эталоном внутри default plugin и используется как backend-neutral pre-step перед дальнейшим CPU/GPU шагом свободного газа. Это гарантирует одинаковые правила работы труб в обоих backend-путях без переноса pipe-specific логики в ядро.
- Для pipe solver-а введён отдельный конфиг `PipeSimulationConfig`:
  - `cell_volume_ratio`,
  - `cell_particle_pressure_pa`,
  - `pipe_step_interval_ticks`,
  - `max_pipe_hop_particles_per_step`,
  - `min_pipe_branch_residual_particles`,
  - `vent_discharge_coefficient`,
  - `max_vent_flux_particles_per_tick`,
  - `vent_choked_pressure_ratio`,
  - `pressure_epsilon_pa`.
  - `pump_input_pressure_pa`.
- `SimulationPerfStats` дополнительно хранит `last_pipe_step_ms` и `avg_pipe_step_ms`, а debug-панель показывает отдельное время расчёта труб рядом с общим временем simulation step.
- `DefaultPluginSupportPlugin` после миграции оставлен только как lightweight support-plugin для инициализации `PipeSimulationConfig`, `PipeGasField`, `PipeFluxField` и `PipeFlowVisualState`; сам runtime execution `flux.default` через него больше не идёт.
- Debug Panel использует стандартный panel-режим `AutoHalfScreen`: её общая высота ограничивается половиной доступной высоты окна с учётом верхнего/нижнего отступа, а при превышении этого лимита включается общий scrollbar.
- Кнопка сворачивания у panel-header использует тот же `select_arrow`-спрайт, что и dropdown/select и collapsible-блоки; направление иконки синхронизируется по `collapsed` через `flip_y`.
- Когда panel-scrollbar активен, viewport debug-контента резервирует ширину под полосу прокрутки (`padding-right += scrollbar width`), поэтому scrollbar не накладывается на контентный правый отступ и не «съедает» границы вложенных блоков.
- Контент внутри Debug Panel разбит на вложенные сворачиваемые блоки через переиспользуемый `ui::collapsible_block`; блоки можно вкладывать друг в друга без ограничений, а их контент не имеет собственного скролла.
- `collapsible_block` использует строку заголовка без общей внешней рамки: рамка рисуется только вокруг content-части (левая/правая/нижняя границы; верхняя выключена) и по цвету совпадает с фоном header-строки; сворачивание/разворачивание вызывается кликом по любой точке header-строки, а в правом конце заголовка отображается тот же `select_arrow`-спрайт, что и у dropdown `select` (направление через `flip_y`).
- В корне Debug Panel оставлены только `Iterations` и `Simulation Hz`; секции `Time`, `Gas simulation` и `Gas overlay` вынесены в отдельные collapsible-блоки.
- `Time` показывает метрики по строкам (`Step`, `Step avg`, `Pipe`, `Pipe avg`, `Actual Hz`), а GPU-строки (`compute/upload/readback/total`) отображаются только при backend `GPU`.
- `Gas simulation` использует стандартные `toggle_switch` для `Buoyancy` и `Show impulses` (switch без label прижимается к правой границе строки и использует compact-высоту, сопоставимую с input-строками), показывает `mass error` отдельной строкой и больше не содержит расчётов/вывода `anisotropy` и `radial waves`.
- `Gas overlay` отображается только в режиме `F2` и содержит `Gamma`/`Max color at`; все input/switch-контролы debug-блока выровнены по правой границе строк.
- Editor-панели используют общий `ui::panels::DEFAULT_PANEL_STACK_GAP`, поэтому расстояние между stacked-панелями семантически относится ко всей panel-системе, а не к конкретной паре `Debug/Gas`.
- Для pipe runtime добавлены быстрые `_smoke` проверки и полный stress-контур: сложная multi-vent сеть запускается в трёх pressure-tier режимах (`100 Pa`, `1 kPa`, `1 MPa`) и проверяет стабильность, сохранение массы и backpressure-блокировку тупиковых веток.
- Добавлен регрессионный pipe-тест на сценарий `room with gas -> empty room`: фронт заполнения по прямому сегменту обязан продвигаться на каждый hop (`N` тиков) без пропуска интервалов, а входной vent-сегмент не должен «пустеть через hop».

### Контракт pipe-тика

- Pipe runtime работает как конвейер с шагом `N` тиков (`pipe_step_interval_ticks`, по умолчанию `10`):
  1. Каждый simulation tick синхронизируется topology (`PipeGasField` + `PipeRuntime` + `PipeFluxField`) и обновляется визуальная фаза `PipeFlowVisualState`.
  2. На тиках без hop (`N-1` из `N`) новый `pipe -> pipe` перенос не выполняется.
  3. На hop-тике solver выполняет явный pipeline `offer -> demand -> match -> commit`:
     - для каждой connected pipe-сети сначала считаются external vent-offer значения `offer_i = Σ_j (P_j - P_i)` только по внешним давлениям вентиляций сети;
     - в том же расчёте порты насоса участвуют как отдельные типы:
       - `PumpIn` получает фиксированное отрицательное давление `pump_input_pressure_pa`,
       - `PumpOut` получает максимально допустимое pipe-давление;
    - intake выполняют только вентиляции с отрицательным offer (`offer_i < 0`), а дальше используется линейная формула от `x_pa = delta_per_path = abs(offer_i) / n_outgoing_requests`:
      `x_particles = x_pa / cell_particle_pressure_pa`,
      `request = round(x_particles / 25)`,
      `request = min(request, 200000, max_vent_flux_particles_per_tick)`,
      и применяется минимум одной частицы, если `x_particles >= 5` и после округления получилось `0`.
       где `n_outgoing_requests` — число исходящих запросов (уникальных исходящих направлений спроса) из этой вентиляции в текущем hop; входящие в неё запросы в `n` не учитываются;
     - направления сегментов строятся по маршрутизации source-vent -> sink-vent спроса (от отрицательного offer к положительному) и фиксируются на весь текущий hop;
     - на source-ветке сохраняется минимальный остаток (`min_pipe_branch_residual_particles`);
     - переносы ограничиваются `max_pipe_hop_particles_per_step`.
  4. Перед принятием ребра проверяется `downstream outlet` без мгновенного возврата назад: если у целевой ветки нет выхода, ребро блокируется.
  5. Блокировка распространяется рекурсивно: входящие ветки, у которых нет альтернативного выходного пути, тоже останавливаются (backpressure).
  6. Vent-обмен на hop-тике выполняется отдельным трёхфазным пайплайном: `pipe -> vent_buffer` (drain всех vent-узлов), затем `world -> pipe` intake, затем `vent_buffer -> world` release.
     Перед intake направление в каждом pipe-сегменте пересчитывается заново в текущем hop; для сегментов `vent <-> vent` приоритет имеет разность внешних давлений на вентиляциях.
     Выбранное на intake-этапе направление фиксируется для планирования переносов текущего hop-интервала и не пересчитывается повторно после `vent_buffer -> world` release в том же hop.
     Intake из мира разрешён только vent-узлам с forward-demand по этому пересчитанному направлению и не выполняется для vent-узлов, которые уже сливают свою pipe-массу в buffer в том же hop.
     Если vent-узел должен был быть intake-узлом по новому направлению, но в этом hop имеет buffer (и поэтому intake пропущен), направление всего соответствующего сегмента сбрасывается на текущий hop (сегмент останавливается до следующего пересчёта).
     Обмен работает строго локально по одной world-клетке вентиляции; компонентный/комнатный direct-reservoir за один hop не используется.
  7. На hop-тике solver планирует переносы сразу для обычных pipe-рёбер и для насоса:
     - для насоса планируется `PumpIn -> PumpOut` в объёме `min(input_total, room_left(output))`;
     - при отсутствии выходной трубы или `room_left == 0` перенос не планируется (блокировка насоса);
     - planned-перенос насоса пишется в `PipeFlowVisualState.transfers`, поэтому в `F3` виден такой же moving packet на интервале `N` тиков.
  8. Коммит переносов (и pipe-рёбер, и насоса) выполняется на следующем hop-старте через `previous_transfers`, поэтому перенос внутри насоса не является мгновенным и следует общему конвейерному cadence.
  9. Все species-переносы остаются дискретными и детерминированными; итоговые изменения записываются в `PipeGasField`.
- Такой контракт сохраняет читаемый «фронт» в длинных трубах, но убирает мгновенный проброс по сети и стабилизирует multi-vent сценарии под разными pressure-tier режимами.

### Детерминированная целочисленная математика переноса

- Pipe-перенос использует дискретное представление частиц и не нарушает инвариант сохранения массы.
- Пропорциональное всасывание/выпуск используют детерминированное целочисленное распределение.
- Перед `match` solver фиксирует направление потока на уровне сегмента (между boundary-узлами: вентиляция или развилка): для рёбер сегмента в текущем hop разрешается только одно из двух противоположных направлений.
- На развилках распределение по нескольким исходящим сегментам допускается; ограничение «строго одно направление» применяется именно внутри каждого сегмента.
- Повторный запуск на одинаковом состоянии должен давать одинаковый результат на CPU.
- Для GPU runtime-path parity проверяется одинаковый pre-step и дальнейшее сравнение итоговых gas fields CPU-GPU.

