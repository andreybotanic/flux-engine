# Структура проекта FluxEngine

Документ фиксирует зону ответственности папок и файлов репозитория. Обновляйте его при изменении структуры проекта.

## Папки (ASCII-дерево)

```text
FluxEngine/
|-- .cargo/                  # Локальные cargo alias-ы проекта: сборка/проверка офлайн KTX2-спрайтов, очистка target и релизная сборка через xtask.
|-- assets/                  # Core-графика, музыка и шейдерные ресурсы приложения.
|   |-- fonts/               # UI-шрифты, загружаемые через AssetServer.
|   |-- music/               # Фоновая музыка игры.
|   |   |-- game/            # MP3-треки игрового контекста (мир загружен / Game Menu).
|   |   `-- menu/            # MP3-треки контекста главного меню.
|   |-- shaders/             # Core WGSL-шейдеры вычислений.
|   `-- sprites/             # Source PNG core-спрайтов UI/мира и соседние runtime `.ktx2` с офлайн mip-chain (кроме `ui/main_menu_background.png` и `world/backdrop_noise.png`).
|       |-- ui/              # Source+runtime core UI-спрайты меню/селектов/общих инструментов.
|       `-- world/           # Source+runtime core фоновые текстуры мира.
|-- config/                  # Core TOML-конфиги симуляции/free-gas поведения.
|   |-- backups/             # Резервные копии конфигов.
|   |-- settings.toml        # Пользовательские настройки звука (`audio.music_volume` в шкале 0..100).
|   `-- simulation.toml      # Core runtime-настройки без default-plugin content.
|-- crates/                  # Отдельные workspace-crate-ы публичного Plugin SDK и внутреннего ABI.
|   |-- flux_plugin_abi/     # Внутренний ABI/glue crate для runtime DLL handshake и dispatch.
|   `-- flux_plugin_sdk/     # Публичный Rust-first SDK для авторов runtime-плагинов.
|-- docs/                    # Проектная документация.
|   `-- plugin_sdk/          # mdBook-сайт Plugin SDK с ручными guide-главами и generated API reference.
|-- plugins/                 # Runtime drop-in каталог packaged plugins (`*.fluxplugin`) рядом с игрой.
|-- plugins_dev/             # Runtime dev-каталог expanded plugin-папок `plugins_dev/<plugin_id>/`.
|-- src/                     # Исходный код Rust.
|   |-- app/                 # Сборка и запуск Bevy-приложения.
|   |-- bgm/                 # Runtime-подсистема фоновой музыки (menu/game контексты).
|   |-- bin/                 # Вспомогательные бинарники (перф, утилиты).
|   |-- config/              # Загрузка/валидация конфигов в коде.
|   |-- debug/               # Диагностические режимы и метрики.
|   |-- editor/              # Инструменты редактирования мира, газа и pipe-сети.
|   |-- input/               # Обработка пользовательского ввода.
|   |-- plugins/             # Runtime plugin facade плюс in-project plugin-папки.
|   |   |-- api/              # Core Rust-first Plugin API contracts used by the generated Plugin SDK reference.
|   |   |-- default_plugin/   # Built-in locked `flux.default`: код, configs, assets и pipe-runtime.
|   |   |-- flux_stage1_sample_plugin/        # Tracked sample non-content DLL-плагин для ABI/e2e-тестов.
|   |   `-- flux_stage7_sample_content_plugin/ # Tracked sample content DLL-плагин с Neon gas.
|   |-- render/              # Визуализация мира, pipe-layer и overlay-режимов.
|   |-- simulation/          # CPU/GPU симуляция свободного газа и parity-инфраструктура.
|   |-- ui/                  # Общие UI-компоненты и панели.
|   `-- world/               # Клеточный мир и unified structures.
|-- xtask/                   # Cargo helper crate для сборки, очистки target, релизной сборки и упаковки sample runtime-плагинов.
|-- AGENTS.md                # Правила работы агента.
|-- Cargo.toml               # Манифест проекта и workspace.
`-- Cargo.lock               # Lock-файл зависимостей.
```

## Файлы

- `AGENTS.md`: Правила работы агента в этом репозитории.
- `.cargo/config.toml`: Локальные cargo alias-ы `cargo xtask`, `cargo generate-sprite-ktx`, `cargo check-sprite-ktx`, `cargo clean-target`, `cargo clean-target-hard`, `cargo validate-wgsl` и `cargo build-release` для запуска helper-crate-ов `xtask`/`xtask_wgsl`, офлайн подготовки sprite KTX2-артефактов и release workflow.
- `.gitignore`: Игнорирует runtime artifacts и новые `src/plugins/*/` in-project plugin-папки; tracked исключения — core `src/plugins/api/`, `default_plugin`, API demo plugins, `flux_stage1_sample_plugin`, `flux_stage7_sample_content_plugin`.
- `assets/fonts/ui_main.ttf`: Основной UI-шрифт с поддержкой кириллицы для всех текстовых элементов интерфейса.
- `assets/shaders/gas_solver.wgsl`: GPU-шейдер газового шага (WGSL), синхронизированный с CPU-эталоном.
- `assets/sprites/ui/main_menu_background.png`: Source и runtime fullscreen-фон главного меню (без `.ktx2`).
- `assets/sprites/ui/select_arrow.{png,ktx2}`: Source+runtime UI-спрайт стрелки для выпадающих списков.
- `assets/sprites/ui/tool_build.*`, `tool_erase.*`, `tool_add_gas.*`, `tool_clear_gas.*`: Core source PNG и runtime `.ktx2` спрайты общих инструментов; content-specific tool icons лежат в default plugin assets.
- `assets/sprites/world/backdrop_noise.png`: Source и runtime core фоновая текстура мира (без `.ktx2`); default-owned тайлы/структуры лежат в default plugin assets.
- `assets/music/menu/*.mp3`: Набор треков фоновой музыки для `Main Menu`; сканируется один раз на старте и проигрывается в случайном цикле с fade и паузами.
- `assets/music/game/*.mp3`: Набор треков фоновой музыки для режима загруженного мира (`игра` + `Game Menu`) с тем же циклом воспроизведения.
- `Cargo.lock`: Зафиксированные версии зависимостей Cargo.
- `Cargo.toml`: Манифест Rust-проекта, workspace и зависимости; основной crate, `xtask`, `crates/flux_plugin_sdk` и `crates/flux_plugin_abi` входят в workspace, sample plugin crates живут под `src/plugins/*` и собираются отдельно через `xtask`.
- `crates/flux_plugin_abi/Cargo.toml`: Манифест внутреннего ABI crate-а для runtime plugin handshake.
- `crates/flux_plugin_abi/src/events.rs`: Внутренние ABI payload-структуры событий и mapping raw event kind values.
- `crates/flux_plugin_abi/src/ffi.rs`: C-compatible `Flux*` ABI-структуры, callback typedef-ы и export-name константы для скрытого DLL-контракта.
- `crates/flux_plugin_abi/src/lib.rs`: Точка входа внутреннего ABI crate-а и re-export его модулей.
- `crates/flux_plugin_sdk/Cargo.toml`: Манифест публичного Rust-first Plugin SDK.
- `crates/flux_plugin_sdk/src/api.rs`: Публичные proxy API плагина (`WorldApi`, `EntityApi`, `GasApi`, `UiApi`, `OverlayApi`, `SaveApi`, `TimeApi`, `InputApi`, `LoggerApi`).
- `crates/flux_plugin_sdk/src/descriptors.rs`: Typed descriptor-ы SDK для сущностей, газов, overlay, optional overlay graph, tool и save chunk.
- `crates/flux_plugin_sdk/src/dispatch_state_builder.rs`: Внутренний builder typed dispatch-state из ABI payload для работы proxy API во время handler-вызова.
- `crates/flux_plugin_sdk/src/error.rs`: `PluginError` и базовые ошибки публичного SDK.
- `crates/flux_plugin_sdk/src/events.rs`: Typed runtime events SDK, `PluginEvent` и ABI decode logic, скрытая от plugin author-а за trait-слоем.
- `crates/flux_plugin_sdk/src/ids.rs`: Typed identifier wrapper-ы SDK (`PluginId`, `ContentId`, `ContentTag`, `SubstanceId`) и базовые геометрические helper-типы.
- `crates/flux_plugin_sdk/src/lib.rs`: Публичная точка входа SDK, re-export-ы и macro `declare_plugin!`.
- `crates/flux_plugin_sdk/src/overlay_graph.rs`: Public SDK model for plugin overlay scene graph: node IDs, selectors by `ContentId`/`ContentTag`, `RenderImageNode`, blend/material nodes and DAG validation.
- `crates/flux_plugin_sdk/src/overlay_graph_tests.rs`: SDK unit-тесты graph validation/order, selector builders, `ContentTag` matching и JSON roundtrip overlay graph descriptors.
- `crates/flux_plugin_sdk/src/plugin.rs`: Публичные `Plugin`, `PluginInit` и скрытый `PluginRuntime`, который связывает Rust-плагин с внутренним ABI dispatch.
- `crates/flux_plugin_sdk/src/registrar.rs`: `Registrar<Self>`, typed `subscribe(...)` и внутренняя таблица зарегистрированных Rust-обработчиков.
- `crates/flux_plugin_sdk/src/runtime_host.rs`: Internal SDK host-contract (`RuntimeHostBinding`, `RuntimeHostFns`, save/time snapshots) shared by ABI and built-in runtime execution.
- `crates/flux_plugin_sdk/src/scope.rs`: Внутренний dispatch scope SDK, который временно привязывает proxy API к текущему runtime host.
- `config/backups/simulation.toml.pre_tuning_20260503_174021.toml`: Резервная копия конфигурации симуляции для отката/сравнения.
- `config/simulation.toml`: Core-параметры симуляции/free-gas и визуализации газа; pipe-runtime настройки default plugin-а вынесены отдельно.
- `config/settings.toml`: Пользовательские настройки аудио (`[audio].music_volume` в диапазоне `0..100`) для экрана `Settings`.
- `docs/CHANGELOG.md`: Краткая история важных изменений проекта.
- `docs/game_overview.md`: Описание игрового процесса и пользовательских механик MVP.
- `docs/plugin_sdk/book.toml`: Конфигурация mdBook-сайта Plugin SDK; build output направлен в `target/plugin_sdk_docs`, а sidebar folding включён для collapsed-by-default generated API групп.
- `docs/plugin_sdk/src/SUMMARY.md`: Генерируемая навигация Plugin SDK book: guide-главы и generated API reference, сгруппированный по структурам, enum-ам, константам, методам и событиям.
- `docs/plugin_sdk/src/*.md`: Ручные guide-главы Plugin SDK: обзор, lifecycle, структура package, manifest, build/reload и отдельная глава `overlay-graph-pipeline.md` для нового graph-based overlay runtime path.
- `docs/plugin_sdk/src/examples/{methods,events,constants}/*.md`: Внешние markdown-snippet примеры для generated Plugin SDK страниц; generated reference встраивает их как `SDK Example`, если файл для конкретного item существует и не содержит legacy v4 ABI surface, но отсутствие snippet-а не ломает сборку docs.
- `docs/plugin_sdk/src/generated/*.md`: Детерминированно сгенерированные индексные API-главы Plugin SDK для групп `Structures`, `Enums`, `Constants`, `Methods` и `Events`; обновляются через `cargo xtask generate-plugin-sdk-docs`.
- `docs/plugin_sdk/src/generated/{structures,enums,constants,methods,events}/*.md`: Детерминированно сгенерированные страницы конкретных Plugin SDK API-сущностей с описаниями полей, вариантов, деклараций, аргументов, возвращаемых значений, ссылками на связанные SDK-типы, списками методов структур и встраиваемыми external example-snippets.
- `docs/plugin_sdk/theme/sdk.css`: Кастомные стили интерактивных SDK API-блоков, бейджей и фильтра.
- `docs/plugin_sdk/theme/sdk.js`: Кастомная интерактивность Plugin SDK book: фильтр API items и copy-кнопки для code blocks.
- `docs/project_structure.md`: Карта структуры проекта: дерево папок + зоны ответственности файлов.
- `docs/technical_overview.md`: Техническая архитектура, подсистемы и инженерные ограничения.
- `plugin_state.toml`: Локальный runtime-файл пользовательских настроек plugin enable-state; хранится в корне проекта и игнорируется через `.gitignore`.
- `plugins/.gitkeep`: Фиксирует пустой runtime-каталог для packaged plugins; реальные `.fluxplugin` игнорируются через `.gitignore`.
- `plugins_dev/.gitkeep`: Фиксирует пустой runtime-каталог expanded dev plugins; реальные папки плагинов игнорируются через `.gitignore`.
- `src/app/mod.rs`: Сборка Bevy-приложения, plugin bootstrap/config resource, CLI-флаги запуска включая `--plugins-dev`, backend-инициализация, запуск и подключение общего runtime host-пути для built-in/DLL plugin dispatch.
- `src/bgm/mod.rs`: Отдельный runtime-plugin фоновой музыки: одноразовый startup-скан `assets/music/menu|game`, state machine `fade-in -> play -> fade-out -> pause`, случайный независимый выбор следующего трека, мгновенное переключение контекста `menu <-> game` и полная остановка планировщика при громкости `0`.
- `src/bin/generate_pipe_scenario_saves.rs`: Вспомогательный бинарник, который пересоздаёт стартовые save-slots для пяти эталонных pipe-сценариев через штатный save API.
- `src/bin/gas_perf.rs`: Пайплайн перф-бенчмарка газа (CPU/GPU), parity-gate и отчёты.
- `src/config/hud.rs`: Публичные типы runtime-конфигов HUD, включая substance-контейнеры и режимы видимости по hover, без встроенных entity-label/fallback-конфигов.
- `src/config/config_loader_block.rs`: Внутренняя логика чтения/валидации core/default-plugin TOML-конфигов и подключение gas substances из активного `ContentRegistry`.
- `src/config/config_tests_block.rs`: Тесты загрузки и валидации конфигов.
- `src/config/audio_settings_block.rs`: Runtime-состояние звуковых настроек, нормализация громкости `0..100 -> 0.0..1.0`, загрузка/сохранение `config/settings.toml` и тесты fallback/roundtrip.
- `src/config/mod.rs`: Публичные конфиг-типы, включая `AudioSettingsState`; compatibility `GasRegistry` поверх plugin-owned substance registry, runtime-реестры base/visual/layout/HUD-метаданных и входная точка загрузки конфигов.
- `src/debug/mod.rs`: Debug-режимы, оверлейные метрики и диагностические ресурсы.
- `src/editor/editor_ui_block.rs`: Runtime-обработка editor UI: tooltip, state sync, панели.
- `src/editor/input_block.rs`: Мышь/кисть/выделение и применение инструментов к миру, unified pipe/structure-сети и мосту.
- `src/editor/main_menu_actions_block.rs`: Обработчики действий меню: save/load/new/exit/plugins/reload/confirm, очередь preview-capture и post-save follow-up сценарии.
- `src/editor/main_menu_block.rs`: Композиция логики main menu (escape/actions/ui refresh), включая экран `Settings`.
- `src/editor/main_menu_escape_block.rs`: Обработка Esc и переходов состояний меню/инструментов, включая возврат из `Plugins` и `Settings` к root screen.
- `src/editor/main_menu_plugins_block.rs`: Сборка и in-place синхронизация списка runtime-плагинов для экрана `Plugins`, правила доступности toggle/reload и safe registry rebuild после изменения `EnabledPluginSet`.
- `src/editor/main_menu_save_list_block.rs`: Общая отправка action-ивентов кнопок главного меню, сборка карточек save/load, загрузка preview PNG в UI и hit-test логика primary-click по всей карточке.
- `src/editor/main_menu_settings_block.rs`: Логика экрана `Settings`: открытие вкладок и live-применение изменений слайдера громкости в `AudioSettingsState`.
- `src/editor/main_menu_ui_block.rs`: Обновление состояния и видимости элементов меню, включая экраны save/load/confirm/plugins/settings и отображение текущего значения slider-громкости.
- `src/editor/mod.rs`: Публичные editor-типы/ресурсы и точка сборки editor-систем, включая `Pipe/Vent/Bridge` и состояние поворота моста.
- `src/editor/overlay_setup_block.rs`: Инициализация визуальных editor-оверлеев.
- `src/editor/ui_setup_block.rs`: Сборка editor-UI: панели, кнопки, поля и привязка виджетов.
- `src/editor/ui_setup_debug_panels_block.rs`: Построение контента `Debug Panel` с вложенными сворачиваемыми блоками (`Time`, `Gas simulation`, `Gas overlay`), switch-строками и полями параметров.
- `src/editor/ui_setup_menu_button_factory_block.rs`: Фабрика кнопок модального меню.
- `src/editor/ui_setup_setup_fn_block.rs`: Основная функция первичной сборки editor-UI, включая кнопку `Gases`, подпaнель выбора `Pipe/Vent/Bridge`, контейнеры экранов главного меню и layout экрана `Settings` со слайдером громкости.
- `src/editor/ui_setup_structure_buttons_block.rs`: Вспомогательные фабрики кнопок инструментов/материалов.
- `src/input/camera.rs`: Управление камерой, зум/пан и тесты корректности якоря.
- `src/input/mod.rs`: Плагин подсистемы ввода и wiring систем ввода.
- `src/lib.rs`: Корневой модуль библиотеки и экспорт подсистем, включая `plugins` и подсистему фоновой музыки `bgm`.
- `src/main.rs`: Точка входа бинаря; запускает приложение.
- `src/plugins/abi.rs`: Engine-side wrapper над `crates/flux_plugin_abi`: сборка host/registrar payload для loader/runtime и mapping ABI event kinds в внутренние engine events.
- `src/plugins/api/mod.rs`: Engine-side shared plugin API module root и re-exports для событий, runtime registry, render/UI/save contracts, которые использует хост plugin-системы.
- `src/plugins/api/events.rs`: Plugin event kinds and payloads, including simulation lifecycle, save lifecycle, low-level mouse cell input and keyboard events.
- `src/plugins/api/render_api.rs`: Overlay render contract with `OverlayRenderPolicy` and re-exported overlay scene graph SDK types for engine-side runtime code.
- `src/plugins/api/ui_api.rs`: Declarative plugin UI descriptors for tools, panels, HUD blocks and simple UI node trees.
- `src/plugins/api/save_api.rs`: In-memory plugin save chunk store and chunk payload contracts.
- `src/plugins/api/runtime.rs`: Runtime registry for plugin event subscriber groups, tool descriptors, overlay descriptors (including optional in-process overlay graph) and save chunk descriptors.
- `src/plugins/content.rs`: Content registry runtime-модель: stable `ContentId`, provider plugins, descriptors клеток/структур/overlay, HUD metadata и registered substances.
- `src/plugins/default_plugin/mod.rs`: Built-in locked `flux.default` content/runtime: default descriptor registration, generic ID facade, legacy numeric save adapters и wiring built-in SDK runtime + pipe-runtime support.
- `src/plugins/default_plugin/runtime_sdk.rs`: In-process SDK plugin type для `flux.default`: typed proxy API + lifecycle/simulation/HUD обработчики, и graph-based `RenderOverlay` producer для `F3/Pipes` через `OverlayApi::submit_graph`.
- `src/plugins/default_plugin/overlay_graph.rs`: Builder graph-пайплайна `F3/Pipes`: декларативные selector/material/image узлы, включая static pipe gas squares, moving packets и bridge/vent port icons.
- `src/plugins/default_plugin/descriptors_block.rs`: Внутренний блок сборки descriptors default plugin-а: layer/collision rules, footprint, rotations, sprite metadata и HUD blocks.
- `src/plugins/default_plugin/ids.rs`: Stable IDs `flux.default` для cells/structures/plugin overlays/substances, typed wrapper helpers и asset/config root helpers default plugin-а.
- `src/plugins/default_plugin/tests.rs`: Unit-тесты фасада default plugin-а: legacy ID roundtrip, полнота registry и порядок HUD-блоков.
- `src/plugins/default_plugin/assets/ui/tool_*.{png,ktx2}`: Content-specific source PNG и runtime `.ktx2` UI-иконки default plugin-а для материалов и структур.
- `src/plugins/default_plugin/assets/shaders/pipe_highlight_material.wgsl`: Plugin-owned WGSL-шейдер `Material2d` для яркой подсветки труб в `F3/Pipes`.
- `src/plugins/default_plugin/assets/world/pipe_mask_*.{png,ktx2}`: Source+runtime файловые спрайты труб для всех connection-mask вариантов, загружаемые через `flux_default://world/...`.
- `src/plugins/default_plugin/assets/world/pipe_silhouette_mask_*.{png,ktx2}`: Source+runtime silhouette-спрайты труб для ghost-preview.
- `src/plugins/default_plugin/assets/world/bridge*.*`, `gas_*.*`, `silhouette_*.*`, `tile_*.*`: World source PNG и runtime `.ktx2` спрайты default plugin-а для стен, структур, мостов и pipe overlay.
- `src/plugins/default_plugin/config/cell_types.toml`: Настройки визуала/параметров default-клеток и HUD-конфиг world-клетки для свободного газа.
- `src/plugins/default_plugin/config/gases/*.toml`: Optional data-конфиги default plugin gas substances; при пустой папке базовые `H2/O2/CO2` берутся из built-in default plugin definitions.
- `src/plugins/default_plugin/config/pipe_runtime.toml`: Runtime-настройки конвейерной pipe-модели default plugin-а (cadence hop-step, branch residual, vent-параметры и pressure-конверсия).
- `src/plugins/default_plugin/config/structures/*.toml`: Конфиги appearance и HUD-метаданных встроенных стен и структур (`label`, `draw_priority`, `size_in_cells`, `hud.sort_order` и substance-контейнеры).
- `src/plugins/flux_stage1_sample_plugin/Cargo.toml`: Отдельный `cdylib` crate минимального non-content sample plugin-а на `flux_plugin_sdk`.
- `src/plugins/flux_stage1_sample_plugin/package_template/manifest.toml`: Шаблон packaged plugin manifest для sample DLL, используемый позитивным e2e-тестом.
- `src/plugins/flux_stage1_sample_plugin/package_template/config/sample.toml`: Минимальный config-файл sample plugin package.
- `src/plugins/flux_stage1_sample_plugin/package_template/assets/placeholder.txt`: Минимальный asset-файл sample plugin package.
- `src/plugins/flux_stage1_sample_plugin/src/lib.rs`: Минимальный sample runtime-плагин на `Plugin` + `declare_plugin!`, используемый smoke/e2e workflow-ом сборки.
- `src/plugins/flux_stage7_sample_content_plugin/Cargo.toml`: Отдельный `cdylib` crate sample content plugin-а stage-7.
- `src/plugins/flux_stage7_sample_content_plugin/package_template/manifest.toml`: Шаблон packaged plugin manifest для sample content plugin-а с `content = true`.
- `src/plugins/flux_stage7_sample_content_plugin/package_template/config/sample.toml`: Минимальный config-файл sample content plugin package.
- `src/plugins/flux_stage7_sample_content_plugin/package_template/assets/placeholder.txt`: Минимальный asset-файл sample content plugin package.
- `src/plugins/flux_stage7_sample_content_plugin/src/lib.rs`: Sample content plugin на `flux_plugin_sdk`, регистрирующий внешний газ `flux.sample_content.substance.neon`.
- `src/plugins/diagnostics.rs`: Startup scan packaged archives, дедупликация `PluginId`, resource с результатами проверки и текст для статуса главного меню.
- `src/plugins/id.rs`: Типизированные `PluginId`, `PluginVersion`, `PluginApiVersion` и проверка канонического формата ID.
- `src/plugins/loader.rs`: Чтение packaged/dev plugin-кандидатов, cache-копии runtime-root, загрузка DLL, ABI handshake `create/register/dispatch/destroy`, fingerprint source-а и сбор runtime registration/subscription-модели.
- `src/plugins/manifest.rs`: Парсинг и валидация `manifest.toml` в runtime-структуру `PluginManifest`.
- `src/plugins/mod.rs`: Точка сборки plugin-подсистемы и её публичный re-export API; подключает общий runtime layer для DLL и built-in endpoint-ов.
- `src/plugins/reload.rs`: Атомарный manual reload/rescan runtime-плагинов без загруженного мира: rebuild registry, пересборка gas registry, generation/report и сравнение source fingerprints.
- `src/plugins/registration.rs`: Runtime-структура результата ABI-регистрации plugin capabilities/content, включая сущности, газы, инструменты, overlay, save chunks и event subscriptions без `handler_name`.
- `src/plugins/runtime_builtin.rs`: Built-in runtime endpoint-ы на базе `flux_plugin_sdk::BuiltinPluginRuntime`, conversion engine events -> typed SDK events и сборка runtime registration для `flux.default`.
- `src/plugins/runtime_dll.rs`: Общий runtime executor верхнего уровня: unified plugin endpoint registry, DLL host callbacks, dispatch и per-frame store для declarative overlay graph submit path.
- `src/plugins/runtime_dll_events.rs`: Typed runtime event dispatch для SDK v5: кодирует ABI payload и вызывает единый `flux_plugin_dispatch` у каждого подписанного DLL-плагина.
- `src/plugins/runtime_host_binding.rs`: Engine-side adapter `RuntimeHostContext -> flux_plugin_sdk::__private::RuntimeHostBinding` для единого host-исполнения built-in и ABI plugin runtimes и graph-only overlay submit path (`submit_overlay_graph`).
- `src/plugins/flux_api_cell_demo_plugin/`: Runtime DLL fixture на `flux_plugin_sdk`, демонстрирующий entity/tool/input path нового SDK.
- `src/plugins/flux_api_tick_demo_plugin/`: Runtime DLL fixture на `flux_plugin_sdk`, демонстрирующий simulation pre-step handler без named ABI exports.
- `src/plugins/flux_api_temperature_overlay_plugin/`: Runtime DLL fixture на `flux_plugin_sdk`, демонстрирующий temperature-style graph overlay (`RenderImageNode` + selector-based entity layer) через `submit_graph`.
- `src/plugins/flux_api_ui_save_demo_plugin/`: Runtime DLL fixture на `flux_plugin_sdk`, демонстрирующий HUD, save chunk и input-driven runtime path.
- `src/plugins/registry.rs`: Bootstrap runtime registry/state, default plugin source priority, `LoadedPluginRegistry` и rebuild-helper для menu toggle; content registry создаётся из default descriptors плюс runtime registration включённых content-плагинов.
- `src/plugins/source.rs`: Discovery packaged/dev plugin sources, structured rejected-source diagnostics, source fingerprint и resolve plugin layout внутри plugin root.
- `src/plugins/state.rs`: `EnabledPluginSet`, `plugin_state.toml`, runtime plugin statuses и aggregate `PluginRegistryState`.
- `src/plugins/substances.rs`: Generic plugin-owned substance contract: `SubstanceId`, `SubstanceDefinition`, `SubstanceFlags` и deterministic `SubstanceRegistry` для compact runtime indices.
- `src/render/mod.rs`: Render plugin wiring: core world-view systems + overlay graph runtime systems (assets setup, graph compositor sync).
- `src/render/overlay_graph_runtime.rs`: Generic runtime overlay graph compositor: evaluates DAG order, materializes layer plan (`RenderEntities`/`RenderFreeGas`/`RenderImage`/`Blend`/`Material`), spawns overlay entities и обслуживает dynamic `Rgba8` image instances.
- `src/render/pipe_highlight_material.rs`: Кастомный `Material2d` и helper-логика для shader-подсветки труб в `F3`.
- `src/render/save_preview.rs`: Offscreen preview pipeline для save-slots: отдельная камера, settle-frame в каноническом `F1`, screenshot capture, PNG-запись и восстановление UI/overlay состояния после кадра.
- `src/render/world_view.rs`: Публичные render-системы world view, config-driven appearance z-order и layer-based pipe/bridge visuals.
- `src/render/world_view_cursor_highlight_block.rs`: Helper отрисовки внутренней белой пунктирной рамки внутри наведённой клетки.
- `src/render/world_view_overlay_block.rs`: Логика core overlay `F1/F2` и plugin overlay `F3/Pipes`, курсорной сетки, multi-container pipe gas-square sizing, flow-packet анимации и фильтрации визуального шума для пакетов `< 5` частиц.
- `src/render/world_view_setup_block.rs`: Построение сущностей мира/слоёв, config-driven z-order стен/структур и спавн визуалов из `PlacedStructureMap`.
- `src/render/world_view_tests_block.rs`: Тесты вспомогательной математики рендера.
- `src/save.rs`: Публичный save/load API, типы состояния меню/сессии, plugin menu screen state и queue/event контракты preview-capture.
- `src/save_api_block.rs`: Операции верхнего уровня: list/create/overwrite/load snapshot и canonical preview-path для slot-а.
- `src/save_content_gate_block.rs`: Сбор required plugin content IDs для save-meta и load-gate проверка доступности content перед чтением world chunks.
- `src/save_content_gate_tests_block.rs`: Тесты required-content meta и load-gate сценариев plugin-compatible save schema.
- `src/save_format_tests_block.rs`: Тесты отказа старых/битых save schema и mapping edge cases для gas chunks.
- `src/save_gas_io_block.rs`: Чтение/запись chunk-ов мира, газа, unified placed-structures и node-based pipe-gas формата save schema `6`; world/structure/pipe chunks хранят stable content IDs, gas chunks мапятся между saved stable substance IDs/legacy aliases и текущими compact indices.
- `src/save_meta_io_block.rs`: Метаданные сейва, required content, диагностический список enabled plugins, валидация единственной поддерживаемой save-схемы и preview-chunk `png_v1`.
- `src/save_pipe_gas_io_block.rs`: Чтение/запись node-based pipe-gas chunk v2 со stable pipe-container content IDs и mapping saved substance IDs в текущий compact registry.
- `src/save_tests_block.rs`: Основные тесты сохранения/загрузки, roundtrip, preview meta и shared helpers для save test blocks.
- `src/simulation/backend.rs`: Конфиг backend и параметры размера мира для симуляции.
- `src/simulation/discrete_step.rs`: Публичные контракты дискретного CPU-шага газа.
- `src/simulation/discrete_step_helpers_block.rs`: Вспомогательные функции дискретного шага (kernel/RNG/утилиты).
- `src/simulation/discrete_step_step_block.rs`: Основной алгоритм дискретного шага CPU симуляции.
- `src/simulation/gas.rs`: Публичная модель GasField и связка CPU/GPU состояния.
- `src/simulation/gas_core_block.rs`: Основная логика операций GasField в runtime.
- `src/simulation/gas_test_support_block.rs`: Вспомогательные test-only функции для buoyancy/reachability.
- `src/simulation/gas_tests_block.rs`: Набор тестов GasField/поведения симуляции и регрессий.
- `src/simulation/gpu_solver.rs`: Публичный интерфейс GPU solver и инициализация ресурсов wgpu.
- `src/simulation/gpu_solver_helpers_block.rs`: Вспомогательные функции буферов, bind-групп и dispatch.
- `src/simulation/gpu_solver_impl_core_block.rs`: Core-инициализация/загрузка состояния GPU solver.
- `src/simulation/gpu_solver_impl_exec_block.rs`: Исполнение шага GPU, readback и генерация параметров.
- `src/simulation/mod.rs`: Плагин core-симуляции свободного газа, ресурсы состояния, schedule sets, CPU/GPU backend orchestration, reset GPU solver state и общие perf-метрики.
- `src/simulation/parity.rs`: Публичные parity API и сценарии сравнения CPU/GPU.
- `src/simulation/parity_runtime_block.rs`: Runtime parity-метрики, прогоны сценариев и gate-оценка.
- `src/simulation/parity_tests_block.rs`: Тесты parity-порогов, smoke и GPU-регрессий.
- `src/plugins/default_plugin/pipe_runtime.rs`: Node-based `PipeGasField`, runtime-only `PipeFluxField`, `PipeFlowVisualState` с фазой конвейера, публичный фасад pipe runtime/visual API, support-plugin и wiring тестов pipe-сети; сам pre-step dispatch идёт через built-in SDK runtime.
- `src/plugins/default_plugin/pipe_runtime/pressure.rs`: Helper-ы перевода `particles -> pressure` и форматирования давления для HUD/pipe-рендера.
- `src/plugins/default_plugin/pipe_runtime/scenarios.rs`: Общий builder пяти канонических pipe-сценариев для save-утилиты и acceptance-тестов.
- `src/plugins/default_plugin/pipe_runtime/solver.rs`: Внутренний конвейерный transport solver pipe-сети (`offer -> demand -> match -> commit`) с edge-level downstream outlet check, backpressure-остановкой тупиковых веток, vent budget-ами и записью hop-трансферов в `PipeFlowVisualState`.
- `src/plugins/default_plugin/pipe_runtime/tests.rs`: Acceptance/regression тесты pipe-модели: blocked/backpressure сценарии, возобновление потока после разблокировки, а также multi-vent stress-проверки `100 Pa / 1 kPa / 1 MPa` в smoke/full режимах.
- `src/simulation/runtime_tick_block.rs`: Runtime-шаги core-симуляции свободного газа, GPU/CPU подшаги и perf-метрики; pipe pre-step выполняется default plugin runtime-ом до этого шага.
- `src/simulation/simulation_tests_block.rs`: Тесты конфигурации тика и структурных pre-step правил.
- `src/ui/cell_inspector.rs`: Runtime-сборка и позиционирование HUD инспектора клетки как стека отдельных entity-блоков с общей тенью; plugin HUD теперь собирается через общий `BuildHudForCell` dispatch, включая built-in `flux.default`.
- `src/ui/cell_inspector_model.rs`: Модель данных и formatter HUD инспектора клетки, включая config-driven контейнеры, solid-материалы как отдельные блоки и registry-driven отображение состава газа.
- `src/ui/collapsible_block.rs`: Переиспользуемый UI-компонент сворачиваемого блока с кликабельным заголовком и `select_arrow`-иконкой состояния (как у dropdown `select`); рамка рисуется только вокруг content-области (цвет как у header), включая runtime-синхронизацию видимости и unit-тесты.
- `src/ui/input_field.rs`: Публичные типы text-input и точка сборки input-систем.
- `src/ui/input_field_helpers_block.rs`: Вспомогательная геометрия курсора текста и точный hit-test/каретка через `ComputedTextBlock`.
- `src/ui/input_field_systems_block.rs`: Системы focus/keyboard/render/caret для текстовых полей.
- `src/ui/modal.rs`: Публичные типы reusable modal backdrop subsystem, helper-ы спавна backdrop-слоёв и wiring `ModalPlugin`.
- `src/ui/modal_capture_block.rs`: Snapshot/capture runtime для modal backdrop-ов: offscreen-камера, resize target-а, blur world-snapshot и cache lifecycle.
- `src/ui/modal_runtime_block.rs`: Выбор topmost модалки, cover-layout backdrop-изображений и переключение режимов `PanelFrosted` / `FullscreenBlur`.
- `src/ui/modal_tests_block.rs`: Unit-тесты modal helper-ов, cover-layout и правил refresh/capture для world-snapshot backdrop.
- `src/ui/mod.rs`: UI-плагин, wiring общих UI-систем и exports переиспользуемых UI-компонентов, включая `slider`, `toggle_switch` и `collapsible_block`.
- `src/ui/palette.rs`: Единая палитра цветов UI (панели, меню, текст, input/select, tooltip, HUD и тени HUD).
- `src/ui/panels.rs`: Публичные типы panel-системы и композиция блоков панели.
- `src/ui/panels_manager_block.rs`: Состояние и API PanelManager, hit-rect и управление панелями.
- `src/ui/panels_runtime_block.rs`: Runtime-системы панели: layout, состояние viewport-ов и события заголовка; input scroll делегирован общему `scroll_area`.
- `src/ui/panels_tests_block.rs`: Тесты layout/scroll/stack-поведения панелей.
- `src/ui/scroll_area.rs`: Общий scroll-area runtime для modal/panel viewport-ов: wheel input, drag thumb, click on track, visibility scrollbar и приоритет групп ввода.
- `src/ui/select_field.rs`: Dropdown/select-компонент для UI-панелей, динамическая перерисовка option buttons при смене списка и его тесты.
- `src/ui/slider.rs`: Переиспользуемый slider-компонент (`min/max/step`, clamp/квантизация, click+drag по треку) и события изменения значения.
- `src/ui/sim_controls.rs`: UI-контролы симуляции (pause/speed/hotkeys).
- `src/ui/toggle_switch.rs`: Переиспользуемый двухпозиционный toggle-switch UI-компонент для включения/выключения настроек, включая compact-layout для строк без label.
- `src/world/grid.rs`: Клеточная сетка мира, generic material ID wrapper, координатные утилиты и тесты.
- `src/world/mod.rs`: Плагин мира и события изменений клеток.
- `src/world/structures.rs`: Unified layer/descriptor-модель структур, generic structure/layer ID wrapper-ы, `PlacedStructureMap`, rotation, bridge-footprint compatibility helpers и pipe-cut state.
- `xtask/Cargo.toml`: Манифест helper-crate-а для сборки/упаковки runtime-плагинов, cleanup `target` и генерации Plugin SDK документации.
- `xtask/src/lib.rs`: Реализация команд `build-plugin`, `build-plugin --dev`, `pack-plugin`, `build-all-plugins`, `generate-sprite-ktx`, `check-sprite-ktx`, `clean-target`, `clean-target-hard`, `validate-wgsl`, `build-release`, Plugin SDK docs команд, discovery plugin projects, установка expanded output в `plugins_dev/<plugin_id>`, запуск `cargo validate-wgsl` перед `build-release` и безопасная упаковка `.fluxplugin`.
- `xtask/src/sprite_ktx.rs`: Офлайн pipeline built-in sprite-ассетов: поиск `ktx`, скан source PNG в core/default-plugin директориях, генерация `.ktx2` с mipmaps и проверка актуальности generated файлов.
- `xtask/src/plugin_sdk_docs.rs`: Orchestration-модуль Plugin SDK docs команд: собирает generated Markdown, stale-check и mdBook build.
- `xtask/src/target_cleanup.rs`: Очистка transient-артефактов `target/` в двух режимах: обычный cleanup сохраняет cargo build cache, а deep cleanup удаляет и cache-каталоги; здесь же живёт обёртка релизной сборки с предочисткой.
- `xtask_wgsl/Cargo.toml`: Манифест лёгкого helper-crate-а для WGSL-валидации без зависимости на `flux_engine`.
- `xtask_wgsl/src/main.rs`: CLI entrypoint команды `cargo validate-wgsl`, включая проверку аргументов и выход с кодом ошибки при провале валидации.
- `xtask_wgsl/src/wgsl_imports.rs`: WGSL import resolver для `validate-wgsl`: обход репозитория/registry, поиск `#define_import_path` модулей и резолв `#import` зависимостей (включая bevy-модули).
- `xtask_wgsl/src/wgsl_validation.rs`: Реализация команды `validate-wgsl`: валидирует все WGSL по всему репозиторию, отмечает `CHANGED/UNCHANGED`, печатает цветной список `OK/NOT OK`, выводит найденные ошибки и итоговую сводку; использует `cargo wgsl --stdin` и fallback-валидацию bevy-шейдеров (`#import`) через `naga_oil` + резолв импортируемых модулей.
- `xtask/src/plugin_sdk_docs/collector.rs`: Сбор Plugin SDK API-сущностей из Rust AST через `syn`: структуры, методы, callback-типы, константы и события, exclude-фильтрация внутренних helper-ов, mapping `PluginEvent -> typed payload` через `AbiEventPayload`, fallback-описания полей/вариантов и загрузка optional external example-snippets с пропуском legacy v4 ABI примеров.
- `xtask/src/plugin_sdk_docs/model.rs`: Общие модели generated Plugin SDK reference: группы API, item docs, поля, аргументы, варианты, source metadata и схема путей для external examples.
- `xtask/src/plugin_sdk_docs/parser.rs`: Парсинг SDK-facing Rustdoc через `syn`, извлечение summary/section-блоков и поддержка `#[doc(hidden)]` для исключения внутренних SDK helper-ов из generated reference.
- `xtask/src/plugin_sdk_docs/render.rs`: Рендер generated API items в Markdown/HTML-блоки mdBook, включая cross-links на документированные SDK-типы, списки методов структур и подключение external example-snippets.
- `xtask/src/main.rs`: CLI entrypoint, который запускает `xtask::run_from_env()` и возвращает non-zero exit code при ошибке.
