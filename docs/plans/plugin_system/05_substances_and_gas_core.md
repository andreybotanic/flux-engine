# Этап 5: substances и gas core

## Цель этапа

Перевести газы на plugin-owned substance IDs так, чтобы `H2`, `O2`, `CO2` приходили из locked default plugin `flux.default`, а ядро работало только с generic substance registry и compact indices.

Важно: pipe pressure / pipe gas logic тоже принадлежит default plugin. Ядро после полного перехода на плагины не должно знать о трубах, вентиляциях, мостах или правилах pressure-driven pipe-сети. В ядре остаётся только физика свободного газа в world-клетках: хранение частиц, CPU-шаг, GPU-шаг, buoyancy и CPU/GPU parity.

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 4 завершён;
- `flux.default` всегда включён, заблокирован и виден на экране `Plugins`;
- `ContentRegistry` создаётся при startup и rebuild после toggle;
- default plugin регистрирует текущие cells, structures, overlays и HUD metadata через stable `ContentId`;
- current gameplay с `Boundary/Brick/Metal`, `Pipe/Vent/GasSource/GasSink/GasPipeBridge`, `F1/F2/F3` работает без изменения поведения;
- registry умеет различать content и non-content plugin;
- pipe-runtime уже находится в зоне default plugin, а не в `src/simulation/pipes`.

Проверка по текущему состоянию репозитория на 2026-05-10: предпосылки этапа 4 выполнены, и к этапу 5 можно переходить. Если в процессе работы выяснится, что какой-то пункт выше снова сломан, сначала восстановить его и только потом продолжать этап 5.

## Архитектурная граница

Разработчику важно не перепутать две разные зоны:

- Core gas simulation: generic алгоритм свободного газа в world-клетках. Он не знает, что такое `H2`, `O2`, `CO2` как встроенный content, и не знает, что такое pipe.
- Default plugin runtime/content: встроенные substances, pipe entities, source/sink entities, pipe storage, pipe pressure helpers, pipe solver, pipe visuals и pipe tests.

Запрещено добавлять в core новые зависимости от `StructureKind::Pipe`, `PipeGasField`, `PipeSimulationConfig`, `pipe_pressure_pa`, `Vent`, `GasPipeBridge` или других pipe-specific типов. Если такой тип нужен, код должен жить в `src/plugins/default_plugin/pipe_runtime*` или в plugin-facing adapter-е, а не в `src/simulation`.

## Ожидаемые зоны изменений

Вероятные файлы:
- `src/plugins/content.rs`;
- `src/plugins/default_plugin/mod.rs`;
- `src/plugins/default_plugin/descriptors_block.rs`;
- `src/config/mod.rs`;
- `src/config/config_loader_block.rs`;
- `src/simulation/gas.rs`;
- `src/simulation/discrete_step*.rs`;
- `src/simulation/gpu_solver*.rs`;
- `assets/shaders/gas_solver.wgsl`;
- `src/ui/cell_inspector_model.rs`;
- `src/render/world_view*.rs`;
- `src/editor/*`, если списки газов в инструментах всё ещё завязаны на old gas configs;
- `src/plugins/default_plugin/pipe_runtime*.rs`, если pipe HUD/render/tests требуют новый substance registry;
- `src/save*.rs` только если нужен временный adapter между compact index и stable substance id;
- `docs/technical_overview.md`;
- `docs/project_structure.md`;
- `docs/CHANGELOG.md`.

Не переносить в этот этап полную plugin save schema и load gate: это зона этапа 6. На этапе 5 можно оставить текущий save format на compact gas indices, если есть явный adapter и тест, что порядок registry стабилен.

## План реализации для джуниора

### 1. Зафиксируй стартовое состояние

1. Прочитай `docs/game_overview.md`, `docs/technical_overview.md`, `docs/plans/plugin_system/00_roadmap.md`, `docs/plans/plugin_system/04_default_plugin.md` и этот файл.
2. Запусти базовые проверки stage 4:
   - `cargo test default_plugin --lib`;
   - `cargo test plugin_registry_non_content_plugin_does_not_enter_content_registry --lib`;
   - `cargo test descriptor --lib`;
   - `cargo test hud --lib`;
   - `cargo test visual --lib`.
3. Если эти тесты не проходят, не начинай stage 5. Сначала почини stage 4.

Критерий успеха: stage-4 тесты зелёные, а ты понимаешь, где создаётся `ContentRegistry` и где регистрируется `flux.default`.

### 2. Введи stable substance-модель

1. Добавь тип `SubstanceId` рядом с content/plugin ID моделями. Самый ожидаемый вариант: `src/plugins/content.rs`, если substances считаются content, или отдельный `src/plugins/substances.rs`, если файл начнёт разрастаться.
2. Валидация `SubstanceId` должна быть такой же строгой, как у `ContentId`: lowercase ASCII, сегменты через `.`, `_` или `-`, без пустых частей.
3. Добавь `SubstanceDefinition`:
   - `id: SubstanceId`;
   - `plugin_id: PluginId`;
   - `label: String`;
   - `molecular_mass: f32`;
   - `color: [f32; 3]`;
   - flags, если они уже нужны коду. Если flags пока не используются, не придумывай лишнюю систему.
4. Добавь `SubstanceRegistry`:
   - хранит definitions по stable id;
   - умеет вернуть compact index для runtime buffers;
   - умеет вернуть definition по compact index;
   - гарантирует детерминированный порядок compact indices.

Критерий успеха: есть unit-тесты на valid/invalid `SubstanceId`, duplicate id, empty registry, невалидную массу и невалидный цвет.

### 3. Зарегистрируй default substances в `flux.default`

1. В default plugin добавь stable IDs:
   - `flux.default.substance.h2`;
   - `flux.default.substance.o2`;
   - `flux.default.substance.co2`.
2. Перенеси текущие данные `H2/O2/CO2` из `src/plugins/default_plugin/config/gases/*.toml` в default plugin registration path.
3. Если TOML-файлы пока нужны как source данных, сделай это явно: config loader читает их только для default plugin descriptors, а не как "газы ядра".
4. Не меняй gameplay-значения:
   - label;
   - molecular mass;
   - color.

Критерий успеха: `GasRegistry::index_of(...)` или его новый аналог находит базовые газы через stable substance ids default plugin-а, а старые short aliases `h2/o2/co2` работают только через compatibility helper, если они ещё нужны тестам.

### 4. Раздели `GasRegistry` и substance registry без поломки compact buffers

1. Найди все места, где код сейчас ожидает `GasDefinition { id, label, molecular_mass, color }`.
2. Реши, будет ли `GasRegistry` переименован в `SubstanceRegistry` сразу или временно станет wrapper-ом над `SubstanceRegistry`.
3. Для junior-safe пути лучше сначала сделать wrapper:
   - публичные методы `count`, `get`, `molecular_masses`, `color_as_bevy` сохраняются;
   - внутри registry хранит `SubstanceDefinition`;
   - `index_of` принимает stable id и, при необходимости, legacy alias.
4. Compact index должен строиться детерминированно. Текущий порядок по molecular mass можно сохранить, но tie-break должен идти по full `SubstanceId`, а не по короткому имени.
5. Добавь явные helpers:
   - `compact_index(SubstanceId) -> Option<usize>`;
   - `definition_by_index(index) -> Option<&SubstanceDefinition>`;
   - `stable_id_by_index(index) -> Option<&SubstanceId>`.

Критерий успеха: `GasField::from_registry` не знает о default plugin напрямую, а получает уже готовый registry с compact order.

### 5. Проверь CPU/GPU путь свободного газа

1. Убедись, что CPU-алгоритм в `src/simulation/discrete_step*.rs` получает только compact indices и molecular masses.
2. Убедись, что GPU path в `src/simulation/gpu_solver*.rs` получает тот же compact order и тот же массив molecular masses.
3. Если меняешь layout GPU buffer-а, сразу обнови `assets/shaders/gas_solver.wgsl`.
4. Не добавляй в shader plugin-specific branches. WGSL должен видеть только generic gas/substance data.

Критерий успеха: CPU и GPU используют один и тот же порядок substances; никакой hardcoded `3 gases` не возвращается.

### 6. Переведи HUD, render и editor на substance definitions

1. В `src/ui/cell_inspector_model.rs` проверь все места, где выводятся label, composition и color газа.
2. В `src/render/world_view*.rs` проверь все места, где берётся цвет газа для `F1/F2/F3`.
3. В editor UI проверь списки `Add Gas` и `Gas Source`.
4. Все эти места должны брать данные из registry, а не из hardcoded списков.
5. Проверяй не только world gas, но и pipe gas HUD/render: pipe containers должны показывать те же plugin substances.

Критерий успеха: добавление нового substance через registry автоматически появляется в dropdown, HUD и render без отдельной правки списков.

### 7. Оставь pipe-runtime в default plugin

1. Проверь, что `src/simulation/mod.rs` не экспортирует `pipes`.
2. Проверь, что pipe imports идут из `crate::plugins::default_plugin::pipe_runtime`.
3. `PipeSimulationConfig`, `PipeGasField`, `PipeFluxField`, `PipeFlowVisualState`, `pipe_pressure_pa`, `world_pressure_pa`, `apply_pipe_network_step` должны оставаться в default plugin runtime.
4. Если при переводе substances нужно менять pipe tests или pipe HUD, меняй их внутри `src/plugins/default_plugin/pipe_runtime*`.
5. Не переноси pipe pressure helper-ы обратно в core ради удобства.

Критерий успеха: поиск `rg "simulation::pipes|mod pipes|pub mod pipes" src` ничего не находит, а `cargo test --release pipe_runtime --lib` проходит.

### 8. Обнови тесты

Сначала узкие тесты:
- substance id validation;
- substance registry validation;
- default plugin substance registration;
- `GasField::from_registry`;
- HUD composition formatting через plugin substances;
- editor dropdown source для `Add Gas` / `Gas Source`;
- pipe HUD/render tests, если менялся путь labels/colors.

Если менялись CPU/GPU buffers или shader:
- связанные CPU gas tests;
- `cargo test smoke --lib`;
- нужные parity tests;
- при изменении pipe-runtime сначала pipe `_smoke`, потом `cargo test --release pipe_runtime --lib`.

Критерий успеха: тесты покрывают и старые default gases, и хотя бы один synthetic extra substance.

### 9. Обнови документацию и финальные проверки

1. Обнови `docs/technical_overview.md`: опиши новую границу core/free-gas и default-plugin pipe-runtime.
2. Обнови `docs/project_structure.md`, если добавлены или изменены роли файлов.
3. Обнови `docs/CHANGELOG.md` кратко и без мелкого шума.
4. `docs/game_overview.md` меняй только если поменялась пользовательская механика. Если поведение игрока не изменилось, не трогай.
5. Выполни `cargo build --release`.
6. Запусти release-версию, проверь логи, закрой приложение.

Критерий успеха: приложение стартует без ошибок, текущий gameplay визуально работает как раньше, а новая substance-модель видна через тестовый plugin/fixture.

## Edge cases

- Плагин добавляет duplicate `SubstanceId`.
- Плагин добавляет substance без molecular mass.
- Molecular mass не finite или `<= 0`.
- Цвет не finite или не в диапазоне `0..=1`.
- Плагин пытается зарегистрировать substance под чужим plugin namespace.
- Порядок compact indices меняется между запусками без причины.
- GPU получает registry с количеством газов больше трёх.
- HUD получает pipe container с газом, id которого отсутствует в registry.
- Save/load временно работает по compact index до этапа 6: такой adapter должен явно падать, если registry несовместим.

## Заметка для заказчика

Запусти игру с включённым тестовым плагином, который добавляет новый газ, например `Neon`. В `New Game` открой инструмент `Add Gas` или `Gas Source`: новый газ должен появиться в выпадающем списке рядом с `H2/O2/CO2`. Добавь его в мир, переключись в `F2` и наведи HUD на область газа: название, цвет и состав должны идти из plugin substance registry.

Затем проверь трубы: поставь источник, вентиляцию и трубу, дай газу попасть в pipe-сеть, переключись в `F3` и наведи HUD на трубу. Pipe-блок должен показывать тот же substance label/color, а pipe pressure логика должна работать как раньше. Это подтверждает, что pipe gas logic живёт в default plugin и корректно использует plugin substances.

## Критерии успешности

- `H2/O2/CO2` регистрируются default plugin-ом как substances со stable IDs.
- Core free-gas simulation не зависит от default plugin и pipe-specific типов.
- Pipe pressure / pipe gas logic остаётся в default plugin runtime.
- Добавление нового газа через registry не требует правок HUD/render/editor списков.
- CPU/GPU parity не ухудшилась.
- `F1/F2/F3`, HUD, gas source/sink и pipe gameplay работают как до этапа.
