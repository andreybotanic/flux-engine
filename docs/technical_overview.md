# FluxEngine: Technical Overview

## Назначение документа

Этот файл хранит внутреннюю техническую картину проекта: архитектуру, ключевые подсистемы, принятые инженерные решения и ограничения MVP.

Связанная навигационная документация по файлам проекта находится в `docs/project_structure.md`.

## Технологический стек

- Язык: Rust
- Движок/фреймворк: Bevy
- Рендер и вычисления на GPU: wgpu + WGSL
- Конфигурация проекта: core TOML-файлы в `config/`, default-plugin TOML-файлы в `src/plugins/default_plugin/config/`

## Точка входа и запуск

- Приложение запускается через `src/main.rs`, который вызывает `flux_engine::app::run()`.
- Основная сборка приложения находится в `src/app/mod.rs`.
- На старте загружаются конфиги игры, выполняется bootstrap plugin-системы из `plugins/`, `plugins_dev/` и `plugin_state.toml`, создаются ресурсы Bevy, выбирается backend симуляции (`CPU`/`GPU`), подключаются плагины подсистем.
- Ошибки contract/discovery/state bootstrap не валят приложение: они сохраняются в runtime-ресурс plugin registry, видны на экране `Plugins` и выводятся в startup-логи.

## Подсистемы (по модулям)

- `world`: структура клеточного мира, типы клеток, границы.
- `simulation`: generic симуляция свободного газа, backend-переключение, CPU/GPU solver, schedule sets и perf-модель.
- `render`: отрисовка мира, pipe-layer и слоёв визуализации.
- `ui`: игровые и отладочные панели, элементы управления симуляцией, инспектор клеток.
- `input`: обработка ввода, включая управление камерой.
- `editor`: инструменты редактирования мира, газа и pipe-сети.
- `debug`: отладочные режимы и диагностические инструменты.
- `save`: сохранение/загрузка мира.
- `config`: загрузка и валидация конфигурации игры.
- `plugins`: runtime plugin contract, packaged/dev source discovery, registry/state bootstrap/reload, Windows DLL handshake, startup diagnostics, default content и default plugin runtime для content-bound логики труб.

### Организация крупных модулей и документирование API

- Крупные самостоятельные блоки должны выноситься в отдельные файлы/модули; это особенно важно для потенциально переиспользуемых UI-блоков.
- `editor` разделён на несколько файлов внутри `src/editor/` (UI-setup, overlay setup, main-menu logic, runtime UI refresh/actions, input/selection logic), а `mod.rs` выступает точкой сборки.
- Для публичного API действует обязательный `Rustdoc`-минимум: `///` перед каждым `pub struct` и `pub fn` с кратким описанием назначения.
- Дополнительно декомпозированы крупные модули `save`, `simulation`, `render`, `ui`, `config` на отдельные `*_block.rs` части через `include!`, чтобы сократить размер основных файлов и упростить локальную навигацию по подсистемам.

### Runtime plugin bootstrap, menu UI и default content (stages 1-4)

- В `src/plugins/` stage-1 контрактный слой расширен bootstrap/default-content слоями: приложение поднимает `PluginSourceRegistry`, `LoadedPluginRegistry`, `EnabledPluginSet`, `ContentRegistry`, `DefaultPluginContent` и aggregate `PluginRegistryState`.
- Поддерживаются два физических источника plugin-пакетов:
  - packaged archives `plugins/*.fluxplugin`;
  - expanded dev directories `plugins_dev/<plugin_id>/`.
- По умолчанию при совпадении `PluginId` выбирается packaged source. При запуске с `--plugins-dev` включается dev mode, и expanded source из `plugins_dev/<plugin_id>` получает приоритет над packaged archive с тем же ID.
- Пользовательские настройки включения хранятся отдельно от сейвов в `plugin_state.toml` в корне репозитория. Отсутствующий файл означает "включён только default plugin".
- Synthetic default plugin `flux.default` существует всегда как built-in registry item: он принудительно `enabled`, `locked` и считается content-provider даже без внешних пакетов.
- `PluginRegistryState` держит данные для экрана `Plugins` и подробные log-friendly сообщения по каждому plugin entry; root `Main Menu` не показывает список плагинов.
- `PluginBootstrapConfig` хранится как Bevy resource: экран `Plugins` использует тот же runtime root/config, что и startup bootstrap, чтобы безопасно перестраивать registry после toggle.
- В `Main Menu` есть экран `Plugins`: он читает `PluginRegistryState` и `EnabledPluginSet`, показывает display name, id, version, source kind, status, content-флаг, короткую ошибку для missing/error записей и использует стандартный `ui::toggle_switch` для включения/выключения.
- Переключатели плагинов активны только в `MainMenuMode::Main`, пока `WorldLoadState.has_world == false`; из `Game Menu` экран `Plugins` остаётся read-only и вместо `toggle_switch` рисует обычную текстовую метку фактического состояния `On`/`Off`.
- Toggle записывает новый `EnabledPluginSet` в `plugin_state.toml`, затем вызывает `rebuild_plugin_registry_from_enabled_set(...)` и атомарно заменяет `PluginSourceRegistry`, `LoadedPluginRegistry`, `EnabledPluginSet`, `ContentRegistry` и `PluginRegistryState` без изменения world runtime state.
- Успешный toggle не пишет служебный статус в `MainMenuUiState.status_text`; строки экрана `Plugins` синхронизируются на месте, а полный rebuild списка выполняется только если после registry rebuild изменился набор plugin entries.
- `flux.default` всегда показывается как `On / Locked`; broken/missing plugin нельзя включить, но если он уже был enabled в config, UI разрешает выключить его, чтобы очистить состояние.
- Для enabled runtime DLL-плагинов дополнительно создаётся `RuntimeDllPluginRegistry`: он держит DLL library + plugin handle живыми и dispatch-ит события в единый `flux_plugin_dispatch` только для тех `PluginEvent`, на которые плагин заранее подписался во время `register`.
- Manifest валидируется отдельно от runtime: проверяются `PluginId`, semver `version`, точное совпадение `api_version` с версией движка и безопасность относительных путей.
- Перед extraction перечисляются все ZIP entries и запрещаются `..`, absolute roots, `.`-сегменты и Windows drive-prefix; это исключает выход за пределы plugin root.
- На Windows DLL загружается не из исходного архива и не из исходной dev-папки, а из временной generation-копии в `std::env::temp_dir()/FluxEngine/plugin_cache/...`; после ABI-проверки копия удаляется best-effort.
- Для каждого валидного physical source хранится `PluginSourceFingerprint`: packaged source fingerprint считается по архиву, а expanded dev source — по `manifest.toml`, DLL, `config/` и `assets/`.
- Стабильный ABI использует plain C-compatible типы: `FluxUtf8Slice`, `FluxStatus`, `FluxHostApi`, `FluxRegistrar`, `FluxPluginHandle` и четыре обязательные export-функции DLL.
- Начиная с API version `2`, `FluxRegistrar` поддерживает callback `register_gas_substance`: content-плагин может зарегистрировать газовые вещества во время handshake-сценария `api_version -> create -> register -> destroy`.
- Для позитивной e2e-проверки stage-1 в репозитории добавлен отдельный sample `cdylib` crate `src/plugins/flux_stage1_sample_plugin`: unit-тесты собирают его, упаковывают в `.fluxplugin` и проверяют, что startup scan принимает рабочий DLL-плагин.
- Если внешних plugins нет, игра стартует как раньше, но registry всё равно содержит `flux.default`. Если packaged/dev plugin сломан, приложение продолжает запуск и показывает понятную причину отклонения в `Main Menu`.

### Default plugin content registry (stage 4)

- Built-in content описан как content locked default plugin-а `flux.default`, без изменения игрового поведения.
- `src/plugins/content.rs` содержит общий `ContentRegistry`: provider plugins, descriptors для world cells, structures, overlay modes и HUD metadata. `ContentId` валидируется тем же каноническим форматом, что и plugin IDs.
- Core runtime-типы `CellMaterial`, `StructureKind`, `LayerKind` и `LayerMarkerKind` больше не являются enum-ами с вариантами default content. Это тонкие static-id wrapper-ы, а конкретные IDs для `Boundary`, `Brick`, `Metal`, `Pipe`, `Vent`, `GasSource`, `GasSink` и `GasPipeBridge` выдаёт facade locked default plugin-а.
- `F1/Main` и `F2/Gas` являются базовыми overlay ядра. Content-specific `F3/Pipes` регистрируется default plugin-ом как plugin overlay и включается через `OverlayMode::Plugin(...)`.
- `WorldGrid` хранит generic `CellMaterial` ID, `PlacedStructureMap` хранит generic `StructureKind` ID, а default-specific проверки и legacy mapping остаются в фасаде `src/plugins/default_plugin/mod.rs`; stable ID/root helpers вынесены в `src/plugins/default_plugin/ids.rs` и re-export-ятся фасадом.
- Начиная со save schema `6`, save/load пишет stable plugin content IDs для world-клеток, placed structures, pipe containers и substances; legacy numeric adapters default plugin-а остаются только compatibility helper-ами для старых runtime-путей и тестов.
- Helper-ы `cell_material_descriptor(...)`, `structure_descriptor(...)` и size-helper-ы сохранены как compatibility API, но внутри берут layer/footprint/sprite metadata из default plugin descriptors.
- Config loader строит visual/HUD maps из default plugin descriptors и затем валидирует TOML-конфиги default plugin-а на совпадение размеров, labels, HUD-блоков и draw priority. Default-owned sprite paths идут через Bevy asset source `flux_default://...`, а порядок HUD-блоков остаётся прежним: `Cell`, `Pipe`, `Bridge`, `Vent`, `Gas Source`, `Gas Sink`.
- Все tracked assets/config/code built-in default plugin-а находятся внутри `src/plugins/default_plugin/`. Каталог `src/plugins/` остаётся фасадом plugin-системы, но его подпапки считаются in-project plugin roots; новые plugin-папки игнорируются этим репозиторием и должны жить в собственных git-репозиториях.
- При startup bootstrap и при rebuild после toggle registry создаётся заново с default descriptors; внешний plugin source для `flux.default` не нужен, потому что он built-in, locked и always-on.

### Plugin-owned substances (stage 5)

- Базовые газы `H2`, `O2`, `CO2` теперь представлены как plugin-owned substances default plugin-а `flux.default` со stable IDs:
  - `flux.default.substance.h2`;
  - `flux.default.substance.o2`;
  - `flux.default.substance.co2`.
- `src/plugins/substances.rs` содержит generic substance-контракт: `SubstanceId`, `SubstanceDefinition`, `SubstanceFlags` и `SubstanceRegistry`.
- `ContentRegistry` хранит substance definitions рядом с descriptors клеток, структур и overlay, поэтому default plugin регистрирует не только world content, но и встроенные вещества.
- `GasRegistry` оставлен как compatibility wrapper для существующего runtime-кода, но внутри строится из plugin-owned `SubstanceRegistry` и назначает compact indices детерминированно по molecular mass + stable id.
- Старые short IDs (`h2`, `o2`, `co2`) сохранены как aliases для UI, тестов и backward-compatible load adapter-а; stable id является основным идентификатором для нового сохранения gas chunk-ов.
- `src/plugins/default_plugin/config/gases/*.toml` больше не считается источником газов ядра. Это optional data-файлы default plugin-а: они могут переопределить/добавить default-plugin gas substances, а при пустой папке базовые `H2/O2/CO2` всё равно берутся из built-in default plugin definitions.
- CPU/GPU free-gas path продолжает работать только с compact indices и массивом molecular masses; WGSL не содержит plugin-specific веток и читает dynamic `molecular_masses` storage buffer.
- Perf/parity helpers используют default substance registry и динамический список mass-error метрик, поэтому проверочные пути не ограничивают runtime тремя газами.

### External content plugin build workflow (stage 7)

- Workspace теперь содержит `xtask/` как отдельный crate и cargo alias `.cargo/config.toml`: `cargo xtask ...` разворачивается в `cargo run -p xtask -- ...`.
- `xtask` ищет plugin projects в `src/plugins/*/package_template/manifest.toml`, читает runtime manifest тем же `PluginManifest`, сортирует проекты по `PluginId` и отклоняет дубли.
- Поддерживаются команды:
  - `cargo xtask build-plugin <plugin_id>` собирает plugin DLL, копирует `manifest.toml`, `bin/`, `config/`, `assets/` в `target/plugins/expanded/<plugin_id>/` и валидирует expanded root через runtime loader.
  - `cargo xtask build-plugin <plugin_id> --dev` делает ту же сборку, затем обновляет `plugins_dev/<plugin_id>` через временную папку и повторно валидирует установленный dev-root; после этого в запущенной игре достаточно нажать `Reload`.
  - `cargo xtask pack-plugin <plugin_id>` выполняет build, пишет `.fluxplugin` в `target/plugins/packages/<plugin_id>.fluxplugin` и валидирует archive через runtime loader.
  - `cargo xtask build-all-plugins` собирает и упаковывает все найденные plugin projects в детерминированном порядке.
- Plugin SDK documentation lives in `docs/plugin_sdk/` as an mdBook site. Генератор reference рассматривает `crates/flux_plugin_sdk/src/*` как основной user-facing источник, а не engine-side ABI wrapper-ы в `src/plugins/`.
- Generated Plugin SDK navigation is grouped by API role: `Structures`, `Enums`, `Constants`, `Methods` and `Events`. Each concrete item gets its own generated page so the mdBook menu points to SDK-facing items вроде `Plugin`, `PluginInit`, `Registrar::subscribe`, `EntityApi::place` или `PluginEvent::MouseDownCell`, а не к внутренним ABI helper-ам.
- The generated API groups are rendered as mdBook foldable sidebar nodes and are collapsed by default through `[output.html.fold] enable = true` with `level = 0`.
- Structure pages render field tables from public struct fields, enum pages render variant tables, constant pages render declarations, method pages render arguments and return values from signatures or callback type aliases, and event pages render trigger descriptions plus payload arguments inferred from the matching `PluginEvent` variant. Structure pages also list owned methods, and documented SDK types are cross-linked from field, payload, argument and return-value cells.
- SDK examples are stored outside Rustdoc in `docs/plugin_sdk/src/examples/{methods,events,constants}/`. Generated pages встраивают эти snippets, если соответствующий markdown-файл существует и не содержит legacy v4 ABI surface вроде `FluxRuntimeHost`/`extern "C"`, и ссылаются обратно на source snippet file; отсутствие внешнего snippet-а больше не ломает documentation pipeline.
- The SDK docs generator hides internal runtime helpers вроде `PluginRuntime`, собирает event payload tables из typed SDK event-структур через `AbiEventPayload` mapping и подставляет безопасные fallback-описания для полей/enum-вариантов, если у конкретного user-facing item нет отдельного подробного section-блока. `cargo xtask generate-plugin-sdk-docs` и `cargo xtask check-plugin-sdk-docs` всё ещё валятся на реально плохих состояниях вроде отсутствующего summary doc-comment, пустого external snippet-а или устаревших tracked generated files.
- Plugin SDK docs commands:
  - `cargo xtask generate-plugin-sdk-docs` rewrites generated Markdown chapters.
  - `cargo xtask check-plugin-sdk-docs` validates the generated reference model and fails when tracked generated docs are stale.
  - `cargo xtask build-plugin-sdk-docs` regenerates docs and builds the mdBook site into `target/plugin_sdk_docs`.
- Упаковщик включает в archive только разрешённые package paths (`manifest.toml`, `bin/`, `config/`, `assets/`) и запрещает служебные/опасные segments вроде `target`, `.git`, editor cache, secrets, `..` и absolute paths.
- Stage-7 sample content plugin находится в `src/plugins/flux_stage7_sample_content_plugin`: его ABI v2 DLL регистрирует газ `flux.sample_content.substance.neon` с alias `neon`.
- Runtime registration сохраняется в `PluginRuntimeRegistration`, затем `LoadedPluginRegistry` передаёт её в `ContentRegistry`. При включении/выключении content-плагина из `Main Menu -> Plugins` rebuild пересоздаёт `ContentRegistry`, `GasRegistry`, world/pipe gas fields, pipe flux state и GPU solver buffers, а gas dropdown-поля обновляются без перезапуска.
- `GameConfig::load_from_default_location_with_content(...)` и `load_gas_registry_from_default_location(...)` строят `GasRegistry` из default substances плюс substances включённых content-плагинов. Save/load gate использует те же stable substance IDs, поэтому мир с plugin-owned газом требует соответствующий enabled content plugin.

### Dev mode и hot reload (stage 8)

- Ручной reload доступен на экране `Main Menu -> Plugins` только пока `WorldLoadState.has_world == false`; в `Game Menu -> Plugins` экран остаётся read-only.
- `src/plugins/reload.rs` является единой точкой reload-контракта: сначала строится новый `PluginBootstrapOutput`, затем новый `GasRegistry`, и только после полного успеха UI заменяет активные Bevy resources.
- Reload не меняет `WorldGrid`, `PlacedStructureMap`, `WorldLoadState` и save state. Сброс `GasField`, `PipeGasField`, `PipeFluxField` и GPU solver выполняется только в main menu без загруженного мира.
- `PluginReloadReport` содержит monotonic generation, список source IDs с изменившимся fingerprint и готовые resources для атомарной замены.
- Dev plugins можно менять как expanded directory `plugins_dev/<plugin_id>/`; для приоритета dev source игру нужно запускать с `--plugins-dev`. Изменения manifest/config/assets/DLL подхватываются reload/rescan без упаковки `.fluxplugin`. Изменения кода всё равно требуют пересборки DLL через `cargo xtask build-plugin <plugin_id> --dev` или обычный build plugin crate.

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
- Набор игровых структур, доступных в текущей сборке (`Pipe`, `Vent`, `GasSource`, `GasSink`, `GasPipeBridge`), поставляется locked default plugin-ом `flux.default` через stable IDs, descriptors и facade-предикаты.
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
  - `3x1` или `1x3` только для `GasPipeBridge`.
- Мирные `Solid`-клетки не переводились в `PlacedStructureMap`, но используют тот же descriptor-подход через `cell_material_descriptor(...)`, чтобы placement-check и рендер использовали одинаковые правила слоёв.
- Core отвечает за generic geometry/collision/snapshot операции, а default-specific predicates, IDs, sort order и descriptor metadata берутся через facade `flux.default`. Старые convenience helper-ы для pipe/source/sink/bridge пока сохранены как compatibility API и делегируют на default plugin facade.
- Коллизии по слоям проверяются только по `Special`-клеткам слоя; `RenderOnly` участвует только в визуализации.
- Тип источника вещества для HUD описывается конфигом, а не хардкодом по `StructureKind`: сейчас поддержаны `world_cell` и `pipe_node`, но контракт сразу рассчитан на будущие контейнеры не только для газов.
- Это даёт контролируемое наложение объектов:
  - труба может проходить через стену;
  - мост может визуально пересекать другие объекты;
  - вентиляция и край моста конфликтуют именно потому, что оба занимают special-клетку слоя `GasPipeConnections`.

### Source / Sink / Vent / Bridge как структуры

- `GasSource` и `GasSink` теперь тоже хранятся в `PlacedStructureMap`, а их параметры живут в `StructureParams`.
- Редактируемыми считаются только `GasSource` и `GasSink`; для этого есть helper `editable_structure_at(...)`.
- `Vent` хранится как самостоятельная структура и использует два слоя:
  - `Appearance`;
  - `GasPipeConnections`.
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
- При дефолтных настройках `1` частица в world-клетке создаёт `1 Pa`, а `1` частица в трубе создаёт `25 Pa`.
- Runtime-состояние потока вынесено в отдельный `PipeFluxField`: он хранит signed `f32` flux по stable edge keys, не сериализуется в save и сбрасывается при `New Game`, `Load` и изменении pipe-топологии.
- Канонические test/save fixtures для труб вынесены в `src/plugins/default_plugin/pipe_runtime/scenarios.rs`, чтобы тесты и `generate_pipe_scenario_saves` использовали один и тот же builder.

### Pipe runtime default plugin и граф сети

- `apply_pipe_network_step(...)` живёт в `src/plugins/default_plugin/pipe_runtime.rs` и получает `PlacedStructureMap`, а не legacy `PipeGrid`.
- Граф pipe-сети строится runtime-ом из descriptors/structure-footprints:
  - соседние обычные трубы соединяются по ортогональному соседству;
  - вентиляция подключается только к pipe-node своей клетки;
  - мост подключается наружу только через крайние клетки;
  - автоматического same-cell соединения между pipe под мостом и bridge-node нет.
- Ножницы (`pipe_cuts`) теперь являются частью `PlacedStructureMap`; они режут только внешние соединения между соседними pipe-node и не ломают внутреннюю топологию моста.
- Drag-построение обычных труб тоже управляет `pipe_cuts`: stroke-API в `PlacedStructureMap` снимает cut только между соседними клетками самой линии и, наоборот, ставит cut между новыми клетками штриха и боковыми pipe-соседями вне линии. Благодаря этому вертикальный штрих между двумя горизонтальными трубами не склеивает их автоматически, но повторный проход по уже существующей линии может явно восстановить нужное соединение.
- Редактирование `GasSource/GasSink` не должно помечать `PlacedStructureMap` изменённым без реального изменения параметров: editor UI сравнивает желаемые значения с текущими, а `update_gas_source/update_gas_sink` дополнительно делают no-op на одинаковых параметрах. Это предотвращает лишний respawn pipe-visual entity и потерю видимости `F3`-индикаторов.
- Pressure-driven часть pipe runtime вынесена в `src/plugins/default_plugin/pipe_runtime/solver.rs`, а `src/plugins/default_plugin/pipe_runtime.rs` оставлен фасадом storage/visual API.
- CPU-алгоритм pipe pre-step внутри default plugin остаётся эталоном; parity smoke-тест для runtime-пути обновлён под новую node-based модель.

### Pre-step структур default plugin

- `DefaultPluginRuntimePlugin` выполняет pre-step фазу до core gas step:
  - `Source` добавляет выбранный газ в свою клетку;
  - `Sink` удаляет газ пропорционально долям газов в клетке, с полным удалением при нехватке массы.
- Пропорциональное удаление реализовано детерминированно целочисленно (largest remainder + стабильный tie-break по индексу газа).
- Одинаковая pre-step логика используется перед runtime CPU и runtime GPU путями (GPU получает состояние после pre-step через upload).
- Та же pre-step логика применена в debug one-step (`Enter`) и parity-сценариях.

### Pipe pre-step в default plugin runtime

- `DefaultPluginRuntimePlugin` выполняет pipe step `apply_pipe_network_step(...)` до основного шага свободного газа.
- Этот шаг одинаково вызывается:
  - в runtime CPU backend;
  - в runtime GPU backend до upload состояния;
  - в debug one-step;
  - в parity smoke/regression тестах.
- В текущем MVP pipe-логика реализована единым CPU-эталоном внутри default plugin и используется как backend-neutral pre-step перед дальнейшим CPU/GPU шагом свободного газа. Это гарантирует одинаковые правила работы труб в обоих backend-путях без переноса pipe-specific логики в ядро.
- Для pipe solver-а введён отдельный конфиг `PipeSimulationConfig`:
  - `cell_volume_ratio`,
  - `cell_particle_pressure_pa`,
  - `pipe_flux_gain`,
  - `pipe_flux_damping`,
  - `max_pipe_flux_particles_per_tick`,
  - `vent_discharge_coefficient`,
  - `max_vent_flux_particles_per_tick`,
  - `vent_choked_pressure_ratio`,
  - `pressure_epsilon_pa`.
- `SimulationPerfStats` дополнительно хранит `last_pipe_step_ms` и `avg_pipe_step_ms`, а debug-панель показывает отдельное время расчёта труб рядом с общим временем simulation step.
- Debug Panel использует стандартный panel-режим `AutoHalfScreen`: её общая высота ограничивается половиной доступной высоты окна с учётом верхнего/нижнего отступа, а при превышении этого лимита включается общий scrollbar.
- Editor-панели используют общий `ui::panels::DEFAULT_PANEL_STACK_GAP`, поэтому расстояние между stacked-панелями семантически относится ко всей panel-системе, а не к конкретной паре `Debug/Gas`.
- Для самых долгих канонических pipe-сценариев в `src/plugins/default_plugin/pipe_runtime/tests.rs` есть быстрые `_smoke` версии: они проверяют раннее сокращение pressure-gap и факт потока по ключевым веткам, а полные acceptance-сценарии запускаются только после успешного smoke-gate.

### Контракт pipe-тика

- Один pipe-тик выполняется в фиксированном порядке:
  1. Синхронизировать `PipeGasField` с текущим `PlacedStructureMap`, построить `PipeRuntime` и выровнять topology-signature в `PipeFluxField`.
  2. Снять snapshot pipe-газа начала тика (`starting_pipe_totals` и `starting_pipe_species`) по всем node-ам.
  3. Для каждой connected component решить semi-implicit pressure field по pipe-node-ам:
     - weight на ребре выводится из отношения давлений и меньшего pipe-volume;
     - remembered edge flux из `PipeFluxField` сохраняет инерцию потока между тиками;
     - итоговый `planned_flux` дополнительно сглаживается вдоль компоненты, чтобы уменьшать локальную рябь на длинных линиях.
  4. Принять только тот `pipe -> pipe` outflow, который реально обеспечен газом в source-node-ах.
  5. Применить все `pipe -> pipe` переносы синхронно одним hop-слоем: газ, пришедший в node в этом тике, не идёт дальше до следующего тика.
  6. После pipe-hop пересчитать vent budgets:
     - `world -> pipe` видит освобождённый входной сегмент и может заполнить его в тот же тик;
     - `pipe -> world` не может выпустить наружу газ, который только что вошёл в сегмент из соседней трубы в этом же тике.
  7. Ограничить vent-request по реальной массе/pressure-equalization gap и применить `pipe -> world`, затем `world -> pipe`.
  8. Перенести species proportions целочисленно, обновить persistent `PipeGasField`, сохранить фактические edge flux обратно в `PipeFluxField` и записать принятые соседние переносы в `PipeFlowVisualState` для анимации `F3`.
- Это позволяет:
  - заполнять длинные трубы плотным фронтом от сильного источника;
  - выравнивать давление между комнатами через длинные магистрали и параллельные ветви без topology-specific хаков;
  - дренировать уже заполненную трубу после падения внешнего давления;
  - делить поток между параллельными ветками одной сети без мгновенного телепорта газа по всей компоненте.

### Детерминированная целочисленная математика переноса

- Pipe-перенос использует дискретное представление частиц и не нарушает инвариант сохранения массы.
- Пропорциональное всасывание/выпуск и деление общего outflow по нескольким соседям используют детерминированное целочисленное распределение со стабильным tie-break.
- Повторный запуск на одинаковом состоянии должен давать одинаковый результат на CPU.
- Для GPU runtime-path parity проверяется одинаковый pre-step и дальнейшее сравнение итоговых gas fields CPU-GPU.

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

### GPU backend: политика запуска и отказоустойчивость

- Выбор backend теперь фиксируется только на старте приложения.
- Дефолтный backend: `GPU`.
- Если `GPU` недоступен при старте, backend автоматически фиксируется в `CPU` до конца процесса.
- Runtime-переключение `GPU -> CPU` внутри симуляционного тика запрещено.
- Если backend уже зафиксирован как `GPU` и во время шага GPU возникает ошибка, приложение пишет error-лог и аварийно завершает работу (fail-fast политика).
- В кодовой базе оставлен ровно один исполняемый вариант на backend:
  - CPU: `simulation::discrete_step::step_discrete_in_place` (runtime и perf используют этот же путь);
  - GPU: `GpuGasSolver` + WGSL `assets/shaders/gas_solver.wgsl` (единый runtime compute-путь, без альтернативных transfer-режимов).
- Управление скоростью симуляции (`x1/x2/x5`) выполняется через частоту `FixedUpdate`:
  - в каждом тикe симуляции выполняется ровно один шаг;
  - ускорение достигается повышением частоты тиков (`target_hz * multiplier`), а не выполнением нескольких шагов в одном тикe.

### CPU/GPU parity и калибровка

- Добавлен отдельный модуль `simulation::parity` для сравнения CPU/GPU симуляции по поклеточным локальным средним концентрациям.
- Основной parity-прогон использует два фиксированных сценария:
  - открытое поле без внутренних стен;
  - поле с внутренними стенами и узкими проходами.
- Отдельный GPU-регрессионный тест `wall_adjacency_gpu_does_not_create_systematic_concentration_drop` усилен corner-сценарием: запертая комната (`3x3`, `5x5`, `9x9`) с начальной концентрацией `100` единиц на клетку и прогоном `100` шагов; тест проверяет, что концентрация в углах не деградирует систематически относительно среднего по комнате.
- Целевой долгий прогон parity: `5000` шагов.
- Пороговые константы parity фиксируются в коде и не пересчитываются в обычных прогонах.
- Одноразовая CPU-only калибровка сохранена как отдельный явный `#[ignore]` тест для ручного запуска при существенных изменениях математики симуляции.
- В обоих backend закреплено единое правило дискретного переноса: закрытые соседние клетки полностью исключаются из распределения вероятностей (их вес равен нулю), перенос в них не производится.
- Для снижения wall-depletion артефакта используется единая CPU/GPU схема перераспределения blocked-потока: если направление закрыто, его базовый вес перераспределяется в два тангенциальных направления (вдоль стены), а при полном блоке уходит в `stay`.
- Для corner-case (у заблокированного направления открыт только один тангенциальный сосед) в CPU и GPU добавлено частичное удержание: половина blocked-веса уходит в доступный тангенциальный ход, вторая половина — в `stay`, что устраняет систематическую просадку концентрации в углах замкнутых комнат.
- GPU shader переведен на ту же дискретную математику, что и CPU: buoyancy-контекст, инициализация RNG (`seed ^ 0x9E37_79B9`) и распределение долей совпадают по алгоритму.
- GPU хранит species как `u32` (flat-buffer), без промежуточного `vec4<f32>` округления.
- Лимит GPU на `3` газа снят: количество газов задается состоянием `GasField` и передается в GPU как динамический `gas_count`, молекулярные массы передаются отдельным буфером.
- Добавлен parity smoke-тест pipe-runtime пути: он прогоняет pipe pre-step перед CPU и GPU solver-путями и проверяет, что итоговое поведение остаётся в допустимых порогах CPU-GPU parity.

### WGSL shader pipeline: зафиксированный анти-паттерн

- Для функции выбора направления в GPU-шейдере зафиксирована безопасная форма `weighted_pick5(w0..w4, sum, state)` с явным unrolled-кодом.
- Вариант с циклом по `array<f32,5>` внутри `weighted_pick` (индексная адресация в loop) ранее вызывал падение при создании shader pipeline (`STATUS_ACCESS_VIOLATION`) на целевой конфигурации драйвера.
- Поэтому в этом месте запрещено возвращаться к loop/index-паттерну: функция должна оставаться scalar/unrolled, даже если это менее компактно.

### Performance pipeline

- `src/bin/generate_pipe_scenario_saves.rs` — отдельная dev-утилита для пересборки визуальных pipe-reference saves; она не влияет на runtime игры и использует обычный save pipeline.
- `src/bin/gas_perf.rs` теперь запускает pipeline:
  1. parity-gate (два сценария по 5000 шагов),
  2. малый preflight,
  3. полный перф-прогон.
- `gas_perf` разрешён только в release-профиле (`cargo run --release --bin gas_perf -- ...`), debug-запуск завершается ошибкой.
- В отчёты `reports/gas_perf_report.csv` и `reports/gas_perf_report.md` добавляется UTC-метка времени старта прогона.
- Полный прогон требует наличие размеров `502x502` и `1002x1002`.
- CPU-часть perf-прогона использует тот же общий код шага, что и runtime (`simulation::discrete_step`), а не отдельную standalone-реализацию.
- Набор газов для perf-прогона берётся из default plugin substance registry, а не из локальных hardcoded констант.
- float/LBM perf-модель полностью исключена из perf-прогона.
- Критерии приемки производительности формализованы в коде:
  - `GPU` быстрее `CPU` на `502x502`,
  - `GPU` быстрее `CPU` на `1002x1002`,
  - `speedup(1002x1002) > speedup(502x502)`.

### Рендер мира и главное меню

- В `src/ui/modal.rs` добавлена переиспользуемая modal-подсистема backdrop-эффектов; любая модалка может выбрать один из двух режимов:
  - `PanelFrosted`: снаружи fullscreen-фон остаётся чётким, а blur живёт только внутри clipped panel-surface, как у матовой пластины;
  - `FullscreenBlur`: весь backdrop под модалкой заменяется размытой fullscreen-копией.
- Источник backdrop и эффект разведены отдельно: modal runtime поддерживает комбинации `Asset(...)` и `WorldSnapshot` с любым из двух режимов, а текущее главное меню использует `Asset + PanelFrosted`, тогда как внутриигровое меню использует `WorldSnapshot + FullscreenBlur`.
- Внешняя тень modal-shell вынесена в reusable helper: `modal_panel_box_shadow()` добавляет более тёмную `BoxShadow`-рамку вокруг `ModalPanelSurface`, не меняя clipped frosted/fullscreen backdrop-логику внутри панели.
- Поверх panel-local blur теперь есть отдельный глобально поддержанный `panel_overlay_tint`: это полупрозрачный цветовой слой между blur и контентом, который можно включать и задавать индивидуально для каждой модалки через `ModalBackdropSpec`.
- Для `PanelFrosted` используется единый helper `cover`-layout: sharp fullscreen-изображение и panel-local blur получают одинаковое масштабирование без искажения пропорций, а панель через `Overflow::clip()` показывает только тот участок blur, который реально находится под ней.
- Для `WorldSnapshot` modal runtime поднимает отдельную offscreen-камеру и делает snapshot текущего мира в `RenderTarget::Image`, затем один раз блюрит получившийся кадр и переиспользует его, пока модалка не закрыта; при новом открытии или изменении размера окна snapshot переснимается.
- Добавлен отдельный fullscreen-фон для `Main Menu` (`assets/sprites/ui/main_menu_background.png`); он показывается только в режиме `MainMenuMode::Main`.
- Pipe-спрайты загружаются из default-plugin asset source `flux_default://world/...` для всех connection-mask вариантов; silhouette-варианты для ghost-preview также хранятся в `src/plugins/default_plugin/assets/world/`.
- Порядок appearance-рисования стен и структур теперь конфигозависимый: основной world-спрайт получает `z` из общего `draw_priority`, а при равенстве используется детерминированный tie-break (`PlacedStructureId` для структур, координаты клетки для стен).
- Для моста добавлены отдельные world-ассеты `bridge.png` и `bridge_silhouette.png`; вертикальный вариант получается поворотом того же спрайта.
- Для `F3` добавлен отдельный pipe-highlight filter layer на `Material2d`/WGSL: поверх обычного pipe-спрайта рисуется отдельный `Mesh2d`, который повторно сэмплирует тот же `pipe_mask_*` и вычисляет яркость highlight в shader-е `flux_default://shaders/pipe_highlight_material.wgsl`.
- Overlay-символ вентиляции для `F3` загружается как отдельный world-ассет `gas_in_out.png`: это жёлтый контурный квадрат с двунаправленной вертикальной стрелкой и чёрным контуром. Обычный world-спрайт вентиляции при этом тоже остаётся видимым под иконкой.
- В `F3/Pipes` (`OverlayMode::Plugin(...)`, content ID default plugin-а) обычный мир затемняется, а для pipe-layer отрисовываются:
  - сама геометрия труб;
  - world-спрайт моста;
  - отдельные vent overlay-символы;
  - цветные квадраты газа внутри труб;
  - движущиеся пакеты переноса из `PipeFlowVisualState`.
- В обычном мире pipe-спрайты рисуются ниже непроницаемых world-клеток, поэтому труба может визуально лежать “внутри” стены; в `F3` основная appearance-геометрия труб тоже остаётся ниже стен, а читаемость обеспечивается отдельными overlay/highlight-слоями поверх solid-спрайтов.
- Служебные pipe-visuals (`vent overlay`, `gas squares`, `flow packets`, `pipe highlight`) не подчиняются `draw_priority` и остаются на отдельных overlay-слоях поверх appearance-спрайтов.
- Tint труб и vent overlay в `F3` намеренно сделан почти белым, чтобы pipe-сеть не терялась на затемнённом фоне.
- Статический газ в трубе рисуется процедурно как белая рамка + цветной fill.
- Если в клетке один pipe-container, квадрат остаётся центральным.
- Если в клетке два pipe-container (обычная труба + bridge pipe), используются два отдельных слота: верхний для `BridgePipe`, нижний для обычной трубы.
- Движущийся gas-packet в `F3` рисуется тем же приёмом белой рамки + цветного fill, но в более компактном диапазоне `14% .. 42%` клетки.
- Для движущихся gas-packet в `F3` по количеству переносимого газа масштабируются цветовая интенсивность и прозрачность только внутренней цветной части; белая рамка остаётся полностью непрозрачной, чтобы пакет не терял читаемую форму.
- Для статического квадрата газа в `F3` и для HUD используется только persistent `PipeGasField`; transient flow-пакеты рендерятся отдельно как moving packets и не подмешиваются в статическое заполнение, чтобы при сильном первом тике труба не выглядела кратковременно “залитой” целиком.
- При любом переходе `paused - running` transient `PipeFlowVisualState` очищается заранее: это не даёт старым пакетам прошлого тика кратковременно появляться после снятия с паузы до первого нового `FixedUpdate`.
- Пока симуляция стоит на паузе, HUD и статический `F3`-overlay показывают только persistent `PipeGasField`; transient flow-пакеты прошлого тика в этот момент игнорируются, чтобы свежие правки pipe-layout не выглядели как мгновенное самозаполнение.
- Вокруг игрового поля добавлены `4` внешних визуальных слоя `border` (только рендер, без изменения симуляционной сетки) с fade к чёрному clear-color.
- Ширина fade-маски синхронизирована с числом внешних border-слоёв: переход занимает `4` клетки и достигает полной непрозрачности на внешней границе 4-го слоя.
- Размер world-backdrop ограничен размерами игрового поля, чтобы за внешними border-слоями гарантированно оставалась чернота clear-color.
- Для внешнего затемнения используется процедурно сгенерированная при запуске спрайт-маска (runtime `Image`), рассчитанная от текущего размера мира; отдельный PNG-файл маски не используется.
- В `apply_overlay_mode` убран ранний выход по change-detection, чтобы видимость world-слоёв и outer-border корректно синхронизировалась каждый кадр и не оставляла артефакты после `Exit to Main`.
- При закрытом меню `MainMenuBackdrop` принудительно переводится в `Hidden` для исключения остаточного отображения при загруженном мире.
- При `New Game` и `Load` применяется единый preset представления: `paused=true`, speed=`x1`, `OverlayMode::Main` (`F1`), камера сбрасывается в центр с `scale=1.0`.
- HUD инспектора клетки динамически расширяет свою высоту под содержимое и, если под курсором есть труба, добавляет отдельный pipe-блок с количеством частиц и строкой давления `Pipe pressure: ...` для каждого pipe-контейнера.
- World-тайлы (`tile_brick`, `tile_metal`, `tile_boundary`, `tile_gas_source`, `tile_gas_sink`) поддерживаются как `64x64` текстуры без встроенной сетки; `tile_boundary` визуально темнее остальных solid-материалов.
- `tile_boundary` выполнен в тёмной палитре (база около `#202020`) со светлыми включениями разной формы (`~#606060..#A0A0A0`) и должен бесшовно тайлиться.
- Плотность светлых включений в `tile_boundary` ограничена так, чтобы тёмная база визуально занимала большую часть площади тайла.

### Камера и панорамирование

- Панорамирование выполняется по дельте курсора окна (`Window::cursor_position`) в логических пикселях, а не по raw `MouseMotion`, чтобы исключить рассинхрон physical/logical координат на HiDPI.
- Для middle-drag используется инвариант якоря курсора: при перемещении камеры точка мира под курсором сохраняется при любом `OrthographicProjection::scale`.

## Конфигурация

Основные группы конфигов:
- `config/simulation.toml` для core/free-gas настроек движка;
- `src/plugins/default_plugin/config/pipe_runtime.toml` для pipe-runtime default plugin-а;
- `src/plugins/default_plugin/config/cell_types.toml`;
- `src/plugins/default_plugin/config/gases/*.toml`;
- `src/plugins/default_plugin/config/structures/*.toml`.

Назначение: вынести игровые и симуляционные параметры из кода в данные, чтобы расширять набор газов и тюнинговать поведение без изменения исходников.

## Поток данных (high-level)

1. Загрузка конфигов на старте.
2. Инициализация ресурсов приложения и мира.
3. Цикл фиксированного шага симуляции.
4. Обновление состояния газа/мира.
5. Синхронизация визуализации и UI.
6. Рендер кадра.

## Ограничения текущего MVP

- Мир фиксированного размера 102x102.
- Фокус разработки на подсистеме газов и базовом редакторе.
- Подсистемы жидкостей, биологии и расширенных материалов находятся в стадии проектирования.
## Pipe Packet Rendering

- Анимация flow-пакетов в `F3` не меняет pipe-симуляцию и использует только служебную visual-path метку внутри `PipeTransferRecord`.
- `PipeTransferRecord` хранит внутреннюю visual-path метку, чтобы render мог отличать bridge-node transfer от обычной трубы в тех же world-клетках, например для трубы под мостом.
- Для обычных transfer-ов пакеты продолжают интерполироваться по прямой между центрами world-клеток.
- Для bridge transfer-ов вида `bridge center <-> bridge port` render семплирует соответствующую половину одной общей quadratic bezier дуги моста, а не строит отдельную дугу для каждого шага.
- Control point этой общей дуги смещён от центра центральной клетки моста на `0.45` клетки в сторону изгиба моста.
- Направление изгиба заранее поддерживает все `StructureRotation`: `Deg0` вверх, `Deg90` влево, `Deg180` вниз, `Deg270` вправо.
- Flow-пакеты меньше `5` частиц не рисуются вообще: для текущей модели это считается статистическим шумом, который только засоряет `F3`.
- Пустые `F3` overlay-слоты труб теперь полностью скрываются; пустой контейнер не должен оставлять даже минимальную точку в клетке.
- Статичный квадрат газа для bridge-pipe в центральной клетке моста использует то же направление изгиба, что и дуга пакетов; если под мостом есть обычная труба, её квадрат сдвигается в противоположную сторону.
## Plugin SDK v5

- Текущий публичный контракт runtime-плагинов живёт в `crates/flux_plugin_sdk`. Это Rust-first SDK: автор плагина реализует `Plugin`, получает `PluginInit`, регистрирует content и подписки через `Registrar<Self>`, а ABI glue генерируется макросом `declare_plugin!(Type)`.
- Внутренний ABI слой полностью вынесен в `crates/flux_plugin_abi`. Все `extern "C"`, `#[repr(C)]`, `Flux*`-структуры, export names и dispatch glue скрыты от пользовательского кода плагина.
- DLL по-прежнему экспортирует обязательные `flux_plugin_api_version`, `flux_plugin_create`, `flux_plugin_register`, `flux_plugin_dispatch` и `flux_plugin_destroy`, но эти entrypoints создаются SDK автоматически. Плагин больше не экспортирует пользовательские named handlers.
- Loader работает по пайплайну `api_version -> create -> register -> dispatch -> destroy`. Во время `register` движок получает зарегистрированный content и список `PluginSubscriptionRegistration`, строит subscriber map по `PluginEvent`, а в runtime вызывает только подписанные плагины.
- В отличие от v4, подписка не хранит `handler_name`. На стороне ABI существует один общий `dispatch`, а маршрутизация к конкретному Rust-методу выполняется внутри SDK по сохранённой таблице `event_kind -> handler`.
- Обработчики событий имеют форму `fn(&mut self, &TypedEvent) -> Result<(), PluginError>`. Объект события обязателен, а `FluxPluginHandle` и `FluxRuntimeHost` больше не участвуют в публичной сигнатуре.
- Игровые API доступны через сам объект плагина. `PluginInit` выдаёт долгоживущие proxy-объекты `WorldApi`, `EntityApi`, `GasApi`, `UiApi`, `PanelApi`, `OverlayApi`, `SaveApi`, `TimeApi`, `InputApi` и `LoggerApi`, которые плагин хранит у себя в полях.
- Эти proxy API работают через скрытый dispatch scope. Перед вызовом обработчика SDK привязывает текущий runtime host, после завершения обработчика очищает scope. Любой вызов API вне разрешённого runtime scope возвращает `PluginError::ApiUnavailable`.
- `WorldApi` теперь отвечает только за чтение мира и координатные helper-ы. Из публичного контракта удалена модель `set_cell_material`: твёрдые клетки и структуры описываются как сущности и редактируются через `EntityApi`.
- `EntityApi` объединяет placement/removal/mutation placeable-объектов, включая `brick`, `metal`, `boundary`, трубы, вентиляции, мосты, `gas source` и `gas sink`. Внутри движка это пока адаптируется к legacy split `WorldGrid + PlacedStructureMap`, но наружу этот split больше не течёт.
- `GasApi` покрывает только операции над свободным газом клетки; `UiApi` ограничен HUD/tool/panel интеграцией без системы уведомлений; `TimeApi` теперь не только читает состояние симуляции, но и умеет менять pause/speed.
- `RuntimeDllPlugin` больше не хранит таблицу named export handlers. Вместо этого на плагин кэшируется один `FluxPluginDispatchFn`, а `runtime_dll_events.rs` только кодирует typed payload и вызывает единый dispatch entrypoint.
- При создании live runtime-экземпляра движок теперь выполняет не только `create`, но и повторный `register` на уже созданном plugin handle. Это нужно, потому что SDK строит свою внутреннюю таблицу `event_kind -> handler` именно во время `register`; без этого плагин считался загруженным, но фактически не реагировал ни на одно событие.
- Runtime host теперь пробрасывает `write_log_fn` и для event-dispatch. Из-за этого `LoggerApi` и сообщения об ошибках из обработчиков больше не теряются во время игры и попадают в stderr-лог с plugin id и уровнем сообщения.
- Sample plugin crates `src/plugins/flux_api_*`, `flux_stage7_sample_content_plugin` и `flux_stage1_sample_plugin` мигрированы на `flux_plugin_sdk`; пользовательский код этих плагинов больше не содержит ручного ABI.
- `xtask` и ручная документация Plugin SDK должны рассматривать `crates/flux_plugin_sdk/src/*` как главный источник user-facing контракта. `src/plugins/abi.rs` в ядре теперь является только engine-side wrapper-слоем над внутренним ABI crate.
