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
- `bgm`: отдельный runtime-плагин фоновой музыки (menu/game контексты, fade и случайный цикл треков).
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
- Для runtime-плагинов создаётся единый `RuntimeDllPluginRegistry` с общими endpoint-ами: внутри него живут и built-in `flux.default`, поднятый через in-process SDK runner, и внешние DLL-плагины. Реестр хранит deterministic порядок вызова, единый subscriber map и dispatch-ит события только тем endpoint-ам, которые подписались на нужный `PluginEvent`.
- Built-in путь не использует `libloading`, export names и ABI payload serialization. Он создаёт обычный `flux_plugin_sdk::Plugin` напрямую в памяти, вызывает `register`, собирает subscriptions и затем получает те же typed runtime events, что и DLL-плагины.
- `flux_plugin_sdk` больше не завязан внутренностями на `FluxRuntimeHost`: proxy API работают через internal `RuntimeHostBinding`, а ABI-layer и built-in host path являются только двумя разными адаптерами к одному и тому же host-contract.
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

- Workspace теперь содержит `xtask/` и `xtask_wgsl/` как отдельные helper-crate-ы; cargo alias `.cargo/config.toml`: `cargo xtask ...` разворачивается в `cargo run -p xtask -- ...`, а `cargo validate-wgsl` — в `cargo run -p xtask_wgsl -- ...`.
- Для hygiene `target/` и предсборочной подготовки добавлены alias-ы `cargo clean-target`, `cargo clean-target-hard`, `cargo validate-wgsl` и `cargo build-release`.
- `cargo clean-target` и `cargo build-release` сохраняют стандартный cargo build cache (`debug/release deps`, `build`, `.fingerprint`, `incremental`), чтобы не пересобирать тяжёлые зависимости вроде Bevy на каждом прогоне.
- `cargo clean-target-hard` существует как one-off режим освобождения места: он уже удаляет и cargo-кэш, поэтому следующая сборка будет заметно холоднее и дольше.
- Для WGSL hygiene добавлен toolchain `cargo-wgsl`; отдельная команда `cargo validate-wgsl` запускает лёгкий crate `xtask_wgsl` (без зависимости на `flux_engine`), валидирует все найденные `*.wgsl`, отмечает изменённые файлы и печатает цветной отчёт со статусами и итоговой сводкой.
- `xtask` ищет plugin projects в `src/plugins/*/package_template/manifest.toml`, читает runtime manifest тем же `PluginManifest`, сортирует проекты по `PluginId` и отклоняет дубли.
- Поддерживаются команды:
  - `cargo xtask build-plugin <plugin_id>` собирает plugin DLL, копирует `manifest.toml`, `bin/`, `config/`, `assets/` в `target/plugins/expanded/<plugin_id>/` и валидирует expanded root через runtime loader.
  - `cargo xtask build-plugin <plugin_id> --dev` делает ту же сборку, затем обновляет `plugins_dev/<plugin_id>` через временную папку и повторно валидирует установленный dev-root; после этого в запущенной игре достаточно нажать `Reload`.
  - `cargo xtask pack-plugin <plugin_id>` выполняет build, пишет `.fluxplugin` в `target/plugins/packages/<plugin_id>.fluxplugin` и валидирует archive через runtime loader.
  - `cargo xtask build-all-plugins` собирает и упаковывает все найденные plugin projects в детерминированном порядке.
  - `cargo xtask generate-sprite-ktx` сканирует built-in файловые PNG-спрайты в `assets/sprites/**` и `src/plugins/default_plugin/assets/**`, конвертирует их в соседние `.ktx2` и генерирует mip-chain через `ktx create` (кроме исключений `assets/sprites/ui/main_menu_background.png` и `assets/sprites/world/backdrop_noise.png`).
  - `cargo xtask check-sprite-ktx` проверяет, что для тех же PNG-источников существуют `.ktx2` и они не старее source PNG (с теми же исключениями).
  - `cargo xtask clean-target` удаляет transient-мусор из `target/`: root-логи, `flycheck*`, `codex_runcheck`, `.rustc_info.json`, `tmp_*`, а также stray logs/скриншоты в profile-папках, но не трогает cargo-кэш сборки.
  - `cargo xtask clean-target-hard` выполняет агрессивную зачистку `target/`: помимо мусора удаляет cache-каталоги Cargo (`debug/release deps`, `build`, `.fingerprint`, `incremental`) и оставляет только живой `xtask`-бинарник и top-level release deliverables (`.exe`, `.pdb`).
  - `cargo validate-wgsl` (или `cargo xtask validate-wgsl`) валидирует все WGSL-файлы репозитория: в списке каждого файла показывает признак `CHANGED/UNCHANGED` и статус `OK/NOT OK`; после списка выводит ошибки по проблемным файлам и финальную сводку `total/changed/ok/errors`; вывод цветной.
  - `cargo xtask build-release` сначала запускает тот же WGSL-чек через `cargo validate-wgsl`, затем выполняет обычный `clean-target` и запускает штатную релизную сборку `cargo build --release`; для чистого WGSL используется `cargo wgsl --stdin`, а для shader-файлов с bevy-предпроцессором (`#import` и т.п.) используется fallback через `naga_oil` с резолвом модулей.
- Plugin SDK documentation lives in `docs/plugin_sdk/` as an mdBook site. Генератор reference рассматривает `crates/flux_plugin_sdk/src/*` как основной user-facing источник, а не engine-side ABI wrapper-ы в `src/plugins/`.
- Generated Plugin SDK navigation is grouped by API role: `Structures`, `Enums`, `Constants`, `Methods` and `Events`. Each concrete item gets its own generated page so the mdBook menu points to SDK-facing items вроде `Plugin`, `PluginInit`, `Registrar::subscribe`, `EntityApi::place` или `PluginEvent::MouseDownCell`, а не к внутренним ABI helper-ам.
- The generated API groups are rendered as mdBook foldable sidebar nodes and are collapsed by default through `[output.html.fold] enable = true` with `level = 0`.
- Structure pages render field tables from public struct fields, enum pages render variant tables, constant pages render declarations, method pages render arguments and return values from signatures or callback type aliases, and event pages render trigger descriptions plus payload arguments inferred from the matching `PluginEvent` variant. Structure pages also list owned methods, and documented SDK types are cross-linked from field, payload, argument and return-value cells.
- SDK examples are stored outside Rustdoc in `docs/plugin_sdk/src/examples/{methods,events,constants}/`. Generated pages встраивают эти snippets, если соответствующий markdown-файл существует и не содержит legacy v4 ABI surface вроде `FluxRuntimeHost`/`extern "C"`, и ссылаются обратно на source snippet file; отсутствие внешнего snippet-а больше не ломает documentation pipeline.
- При генерации Plugin SDK docs все markdown-файлы в `docs/plugin_sdk/src/` теперь записываются в `UTF-8 with BOM`, а ведущий BOM внешнего snippet-а удаляется перед встраиванием в generated page. Это убирает артефакты `п»ї` в `SDK Example` и ложные «невидимые» git-изменения после регенерации.
- The SDK docs generator hides internal runtime helpers вроде `PluginRuntime`, собирает event payload tables из typed SDK event-структур через `AbiEventPayload` mapping и подставляет безопасные fallback-описания для полей/enum-вариантов, если у конкретного user-facing item нет отдельного подробного section-блока. `cargo xtask generate-plugin-sdk-docs` и `cargo xtask check-plugin-sdk-docs` всё ещё валятся на реально плохих состояниях вроде отсутствующего summary doc-comment, пустого external snippet-а или устаревших tracked generated files.
- Plugin SDK docs commands:
  - `cargo xtask generate-plugin-sdk-docs` rewrites generated Markdown chapters.
  - `cargo xtask check-plugin-sdk-docs` validates the generated reference model and fails when tracked generated docs are stale.
  - `cargo xtask build-plugin-sdk-docs` regenerates docs and builds the mdBook site into `target/plugin_sdk_docs`.
- Упаковщик включает в archive только разрешённые package paths (`manifest.toml`, `bin/`, `config/`, `assets/`) и запрещает служебные/опасные segments вроде `target`, `.git`, editor cache, secrets, `..` и absolute paths.
- Stage-7 sample content plugin находится в `src/plugins/flux_stage7_sample_content_plugin`: его ABI v2 DLL регистрирует газ `flux.sample_content.substance.neon` с alias `neon`.
- Runtime registration сохраняется в `PluginRuntimeRegistration`, затем `LoadedPluginRegistry` передаёт её в `ContentRegistry`. При включении/выключении content-плагина из `Main Menu -> Plugins` rebuild пересоздаёт `ContentRegistry`, `GasRegistry`, world/pipe gas fields, pipe flux state и GPU solver buffers, а gas dropdown-поля обновляются без перезапуска.
- `GameConfig::load_from_default_location_with_content(...)` и `load_gas_registry_from_default_location(...)` строят `GasRegistry` из default substances плюс substances включённых content-плагинов. Save/load gate использует те же stable substance IDs, поэтому мир с plugin-owned газом требует соответствующий enabled content plugin.

### Built-in sprite KTX2 pipeline

- Built-in файловые спрайты ядра и `flux.default` хранят исходники в PNG, а в рантайме в основном загружаются из заранее сгенерированных `.ktx2` рядом с исходниками.
- `assets/sprites/ui/main_menu_background.png` и `assets/sprites/world/backdrop_noise.png` являются исключениями: оба ассета загружаются напрямую как PNG и не участвуют в `ktx2`-генерации/check-проверке.
- Глобальный `ImagePlugin` в приложении использует `default_sampler` с линейной фильтрацией (`mag/min/mipmap = Linear`) вместо `default_nearest()`, чтобы precomputed mip-chain у файловых спрайтов использовался корректно.
- Runtime-изображения газа для core-оверлеев `F1/F2` (`texture_f1_*`, `texture_f2_*`, формат `Rgba32Float`) задают локальный sampler `ImageSampler::nearest`, чтобы клеточные границы газа не размывались на стыках с твёрдыми блоками.
- Конвертация в `.ktx2` выполняется офлайн через `xtask` и KTX CLI (`ktx create --format R8G8B8A8_SRGB --generate-mipmap`), без runtime-генерации mipmaps.
- Для RGB PNG без alpha применяется swizzle `rgb1`, чтобы рантайм всегда получал RGBA-совместимый KTX2.
- Runtime-текстуры, создаваемые в коде (`Image::new`, `Image::new_fill`, `Image::from_dynamic`), и `preview.png` внутри save-слотов остаются на PNG-пути и не участвуют в офлайн KTX2-пайплайне.

### Dev mode и hot reload (stage 8)

- Ручной reload доступен на экране `Main Menu -> Plugins` только пока `WorldLoadState.has_world == false`; в `Game Menu -> Plugins` экран остаётся read-only.
- `src/plugins/reload.rs` является единой точкой reload-контракта: сначала строится новый `PluginBootstrapOutput`, затем новый `GasRegistry`, и только после полного успеха UI заменяет активные Bevy resources.
- Reload не меняет `WorldGrid`, `PlacedStructureMap`, `WorldLoadState` и save state. Сброс `GasField`, `PipeGasField`, `PipeFluxField` и GPU solver выполняется только в main menu без загруженного мира.
- `PluginReloadReport` содержит monotonic generation, список source IDs с изменившимся fingerprint и готовые resources для атомарной замены.
- Dev plugins можно менять как expanded directory `plugins_dev/<plugin_id>/`; для приоритета dev source игру нужно запускать с `--plugins-dev`. Изменения manifest/config/assets/DLL подхватываются reload/rescan без упаковки `.fluxplugin`. Изменения кода всё равно требуют пересборки DLL через `cargo xtask build-plugin <plugin_id> --dev` или обычный build plugin crate.

