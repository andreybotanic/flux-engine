# Структура проекта FluxEngine

Документ фиксирует зону ответственности папок и файлов репозитория. Обновляйте его при изменении структуры проекта.

## Папки (ASCII-дерево)

```text
FluxEngine/
|-- assets/                  # Графические и шейдерные ресурсы приложения.
|   |-- fonts/               # UI-шрифты, загружаемые через AssetServer.
|   |-- shaders/             # WGSL-шейдеры вычислений/рендера.
|   `-- sprites/             # Спрайты UI и мира.
|       |-- ui/              # Иконки инструментов и UI-элементы.
|       `-- world/           # Тайлы мира и фоновые текстуры.
|-- config/                  # Внешние TOML-конфиги игрового/симуляционного поведения.
|   |-- backups/             # Резервные копии конфигов.
|   |-- gases/               # Конфиги отдельных газов.
|   `-- structures/          # Конфиги базовых entity-параметров, appearance и HUD-метаданных стен и структур.
|-- crates/                  # Вспомогательные Rust-crate-ы, не входящие в основную библиотеку игры.
|   `-- flux_stage1_sample_plugin/   # Минимальный sample DLL-плагин для stage-1 ABI и e2e-тестов packaged plugins.
|-- docs/                    # Проектная документация.
|   `-- plans/               # Плановые документы будущих крупных изменений.
|       `-- plugin_system/   # Roadmap и этапные планы перехода на runtime-плагины.
|-- plugins/                 # Runtime drop-in каталог packaged plugins (`*.fluxplugin`) рядом с игрой.
|-- plugins_dev/             # Runtime dev-каталог expanded plugin-папок `plugins_dev/<plugin_id>/`.
|-- src/                     # Исходный код Rust.
|   |-- app/                 # Сборка и запуск Bevy-приложения.
|   |-- bin/                 # Вспомогательные бинарники (перф, утилиты).
|   |-- config/              # Загрузка/валидация конфигов в коде.
|   |-- debug/               # Диагностические режимы и метрики.
|   |-- editor/              # Инструменты редактирования мира, газа и pipe-сети.
|   |-- input/               # Обработка пользовательского ввода.
|   |-- plugins/             # Stage-1 runtime plugin contract: manifest, ZIP/DLL validation, ABI и startup diagnostics.
|   |-- render/              # Визуализация мира, pipe-layer и overlay-режимов.
|   |-- simulation/          # CPU/GPU симуляция газа, pipe pre-step и parity-инфраструктура.
|   |   `-- pipes/           # Внутренние модули pressure/fixtures/solver/test-инфраструктуры труб.
|   |-- ui/                  # Общие UI-компоненты и панели.
|   `-- world/               # Клеточный мир и unified structures.
|-- AGENTS.md                # Правила работы агента.
|-- Cargo.toml               # Манифест проекта.
|-- Cargo.lock               # Lock-файл зависимостей.
`-- tmp_size.rs              # Локальный вспомогательный черновой файл.
```

## Файлы

- `AGENTS.md`: Правила работы агента в этом репозитории.
- `assets/fonts/ui_main.ttf`: Основной UI-шрифт с поддержкой кириллицы для всех текстовых элементов интерфейса.
- `assets/shaders/gas_solver.wgsl`: GPU-шейдер газового шага (WGSL), синхронизированный с CPU-эталоном.
- `assets/shaders/pipe_highlight_material.wgsl`: WGSL-шейдер `Material2d` для яркой подсветки труб в `F3`.
- `assets/sprites/ui/main_menu_background.png`: Отдельный fullscreen-фон главного меню.
- `assets/sprites/ui/select_arrow.png`: UI-спрайт стрелки для выпадающих списков.
- `assets/sprites/ui/tool_*.png`: UI-спрайты иконок инструментов, включая отдельную иконку моста `tool_bridge.png`.
- `assets/sprites/world/backdrop_*.png`: Фоновые текстуры мира.
- `assets/sprites/world/pipe_mask_*.png`: Файловые спрайты труб для всех connection-mask вариантов.
- `assets/sprites/world/pipe_silhouette_mask_*.png`: Файловые silhouette-спрайты труб для всех connection-mask вариантов.
- `assets/sprites/world/bridge.png`: Основной world-спрайт газового моста размером `192x64`.
- `assets/sprites/world/bridge_silhouette.png`: Силуэтный preview-спрайт газового моста размером `192x64`.
- `assets/sprites/world/silhouette_*.png`: World-силуэты предпросмотра под курсором.
- `assets/sprites/world/gas_in_out.png`: Базовый жёлтый overlay-спрайт вентиляции для pipe-view `F3`.
- `assets/sprites/world/gas_in.png`: Зелёный вариант overlay-спрайта вентиляции со стрелкой только внутрь.
- `assets/sprites/world/gas_out.png`: Белый вариант overlay-спрайта вентиляции со стрелкой только наружу.
- `assets/sprites/world/tile_*.png`: Спрайты тайлов мира.
- `assets/sprites/world/`: Не содержит статической fade-маски мира; затемняющая маска генерируется в runtime в `src/render/world_view.rs`.
- `Cargo.lock`: Зафиксированные версии зависимостей Cargo.
- `Cargo.toml`: Манифест Rust-проекта и зависимости.
- `config/backups/simulation.toml.pre_tuning_20260503_174021.toml`: Резервная копия конфигурации симуляции для отката/сравнения.
- `config/cell_types.toml`: Настройки визуала/параметров типов клеток и HUD-конфиг world-клетки для свободного газа.
- `config/gases/*.toml`: Конфиги отдельных газов (физические и визуальные параметры).
- `config/structures/*.toml`: Конфиги базовых параметров, appearance и HUD-метаданных встроенных стен и структур (`label`, `draw_priority`, `size_in_cells`, `hud.sort_order` и описания substance-контейнеров).
- `config/simulation.toml`: Основные параметры симуляции и runtime-настройки, включая секцию `[pipe]` для pressure-driven труб.
- `crates/flux_stage1_sample_plugin/Cargo.toml`: Отдельный `cdylib` crate минимального рабочего stage-1 sample plugin-а.
- `crates/flux_stage1_sample_plugin/package_template/manifest.toml`: Шаблон packaged plugin manifest для sample DLL, используемый позитивным e2e-тестом.
- `crates/flux_stage1_sample_plugin/package_template/config/sample.toml`: Минимальный config-файл sample plugin package.
- `crates/flux_stage1_sample_plugin/package_template/assets/placeholder.txt`: Минимальный asset-файл sample plugin package.
- `crates/flux_stage1_sample_plugin/src/lib.rs`: Реализация sample DLL-плагина с обязательными ABI export-ами `flux_plugin_*`.
- `docs/CHANGELOG.md`: Краткая история важных изменений проекта.
- `docs/game_overview.md`: Описание игрового процесса и пользовательских механик MVP.
- `docs/plans/plugin_system/00_roadmap.md`: Общий roadmap будущей миграции FluxEngine на runtime-плагины.
- `docs/plans/plugin_system/*.md`: Детальные инструкции по этапам реализации plugin-system миграции.
- `docs/project_structure.md`: Карта структуры проекта: дерево папок + зоны ответственности файлов.
- `docs/technical_overview.md`: Техническая архитектура, подсистемы и инженерные ограничения.
- `plugin_state.toml`: Локальный runtime-файл пользовательских настроек plugin enable-state; хранится в корне проекта и игнорируется через `.gitignore`.
- `plugins/.gitkeep`: Фиксирует пустой runtime-каталог для packaged plugins; реальные `.fluxplugin` игнорируются через `.gitignore`.
- `plugins_dev/.gitkeep`: Фиксирует пустой runtime-каталог expanded dev plugins; реальные папки плагинов игнорируются через `.gitignore`.
- `src/app/mod.rs`: Сборка Bevy-приложения, stage-2 plugin bootstrap, backend-инициализация и запуск.
- `src/bin/generate_pipe_scenario_saves.rs`: Вспомогательный бинарник, который пересоздаёт стартовые save-slots для пяти эталонных pipe-сценариев через штатный save API.
- `src/bin/gas_perf.rs`: Пайплайн перф-бенчмарка газа (CPU/GPU), parity-gate и отчёты.
- `src/config/hud.rs`: Публичные типы runtime-конфигов HUD, включая substance-контейнеры и режимы видимости по hover, без встроенных entity-label/fallback-конфигов.
- `src/config/config_loader_block.rs`: Внутренняя логика чтения/валидации TOML-конфигов, включая `config/structures/*.toml`.
- `src/config/config_tests_block.rs`: Тесты загрузки и валидации конфигов.
- `src/config/mod.rs`: Публичные конфиг-типы, runtime-реестры base/visual/layout/HUD-метаданных и входная точка загрузки конфигов.
- `src/debug/mod.rs`: Debug-режимы, оверлейные метрики и диагностические ресурсы.
- `src/editor/editor_ui_block.rs`: Runtime-обработка editor UI: tooltip, state sync, панели.
- `src/editor/input_block.rs`: Мышь/кисть/выделение и применение инструментов к миру, unified pipe/structure-сети и мосту.
- `src/editor/main_menu_actions_block.rs`: Обработчики действий меню: save/load/new/exit/confirm, очередь preview-capture и post-save follow-up сценарии.
- `src/editor/main_menu_block.rs`: Композиция логики main menu (escape/actions/ui refresh).
- `src/editor/main_menu_escape_block.rs`: Обработка Esc и переходов состояний меню/инструментов.
- `src/editor/main_menu_save_list_block.rs`: Сборка карточек save/load, загрузка preview PNG в UI и hit-test логика primary-click по всей карточке.
- `src/editor/main_menu_ui_block.rs`: Обновление состояния и видимости элементов меню.
- `src/editor/mod.rs`: Публичные editor-типы/ресурсы и точка сборки editor-систем, включая `Pipe/Vent/Bridge` и состояние поворота моста.
- `src/editor/overlay_setup_block.rs`: Инициализация визуальных editor-оверлеев.
- `src/editor/ui_setup_block.rs`: Сборка editor-UI: панели, кнопки, поля и привязка виджетов.
- `src/editor/ui_setup_debug_panels_block.rs`: Построение debug-панелей и строк параметров.
- `src/editor/ui_setup_menu_button_factory_block.rs`: Фабрика кнопок модального меню.
- `src/editor/ui_setup_setup_fn_block.rs`: Основная функция первичной сборки editor-UI, включая кнопку `Gases` и подпaнель выбора `Pipe/Vent/Bridge`.
- `src/editor/ui_setup_structure_buttons_block.rs`: Вспомогательные фабрики кнопок инструментов/материалов.
- `src/input/camera.rs`: Управление камерой, зум/пан и тесты корректности якоря.
- `src/input/mod.rs`: Плагин подсистемы ввода и wiring систем ввода.
- `src/lib.rs`: Корневой модуль библиотеки и экспорт подсистем, включая новый `plugins`.
- `src/main.rs`: Точка входа бинаря; запускает приложение.
- `src/plugins/abi.rs`: C-compatible ABI stage-1: `FluxUtf8Slice`, `FluxStatus`, host/registrar structs и export names обязательных DLL-функций.
- `src/plugins/diagnostics.rs`: Startup scan packaged archives, дедупликация `PluginId`, resource с результатами проверки и текст для статуса главного меню.
- `src/plugins/id.rs`: Типизированные `PluginId`, `PluginVersion`, `PluginApiVersion` и проверка канонического формата ID.
- `src/plugins/loader.rs`: Чтение packaged/dev plugin-кандидатов, cache-копии runtime-root, загрузка DLL и ABI handshake `create/register/destroy`.
- `src/plugins/manifest.rs`: Парсинг и валидация `manifest.toml` в runtime-структуру `PluginManifest`.
- `src/plugins/mod.rs`: Точка сборки plugin-подсистемы и её публичный re-export API.
- `src/plugins/registry.rs`: Stage-2 bootstrap runtime registry/state, default plugin, source priority, `LoadedPluginRegistry` и `ContentRegistry`.
- `src/plugins/source.rs`: Discovery packaged/dev plugin sources, structured rejected-source diagnostics и resolve plugin layout внутри plugin root.
- `src/plugins/state.rs`: `EnabledPluginSet`, `plugin_state.toml`, runtime plugin statuses и aggregate `PluginRegistryState`.
- `src/render/mod.rs`: Плагин рендера и порядок render-систем, включая pipe visuals.
- `src/render/pipe_highlight_material.rs`: Кастомный `Material2d` и helper-логика для shader-подсветки труб в `F3`.
- `src/render/save_preview.rs`: Offscreen preview pipeline для save-slots: отдельная камера, settle-frame в каноническом `F1`, screenshot capture, PNG-запись и восстановление UI/overlay состояния после кадра.
- `src/render/world_view.rs`: Публичные render-системы world view, config-driven appearance z-order и layer-based pipe/bridge visuals.
- `src/render/world_view_cursor_highlight_block.rs`: Helper отрисовки внутренней белой пунктирной рамки внутри наведённой клетки.
- `src/render/world_view_overlay_block.rs`: Логика overlay-режимов `F1/F2/F3`, курсорной сетки, multi-container pipe gas-square sizing, flow-packet анимации и фильтрации визуального шума для пакетов `< 5` частиц.
- `src/render/world_view_setup_block.rs`: Построение сущностей мира/слоёв, config-driven z-order стен/структур и спавн визуалов из `PlacedStructureMap`.
- `src/render/world_view_tests_block.rs`: Тесты вспомогательной математики рендера.
- `src/save.rs`: Публичный save/load API, типы состояния меню/сессии и queue/event контракты preview-capture.
- `src/save_api_block.rs`: Операции верхнего уровня: list/create/overwrite/load snapshot и canonical preview-path для slot-а.
- `src/save_gas_io_block.rs`: Чтение/запись chunk-ов мира, газа, unified placed-structures и node-based pipe-gas формата текущей save-схемы.
- `src/save_meta_io_block.rs`: Метаданные сейва, валидация единственной поддерживаемой save-схемы и preview-chunk `png_v1`.
- `src/save_tests_block.rs`: Тесты сохранения/загрузки и валидации формата.
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
- `src/simulation/mod.rs`: Плагин симуляции, ресурсы состояния и orchestration тика, включая pipe pre-step и perf-метрики для отдельного времени расчёта труб.
- `src/simulation/parity.rs`: Публичные parity API и сценарии сравнения CPU/GPU.
- `src/simulation/parity_runtime_block.rs`: Runtime parity-метрики, прогоны сценариев и gate-оценка.
- `src/simulation/parity_tests_block.rs`: Тесты parity-порогов, smoke и GPU-регрессий.
- `src/simulation/pipes.rs`: Node-based `PipeGasField`, runtime-only `PipeFluxField`, публичный фасад pipe runtime/visual API и wiring тестов pipe-сети.
- `src/simulation/pipes/pressure.rs`: Helper-ы перевода `particles -> pressure` и форматирования давления для HUD/pipe-рендера.
- `src/simulation/pipes/scenarios.rs`: Общий builder пяти канонических pipe-сценариев для save-утилиты и acceptance-тестов.
- `src/simulation/pipes/solver.rs`: Внутренний semi-implicit pressure+flux solver pipe-сети: component solve, world↔vent budgets, mass-bounded transfers и запись `PipeFlowVisualState`.
- `src/simulation/pipes/tests.rs`: Acceptance/regression тесты новой pipe-модели, включая быстрые `_smoke` проверки для самых долгих сценариев и полные канонические scenario 1..5.
- `src/simulation/runtime_tick_block.rs`: Runtime-шаги симуляции, GPU/CPU подшаги и perf-метрики, включая отдельный замер времени pipe pre-step.
- `src/simulation/simulation_tests_block.rs`: Тесты конфигурации тика и структурных pre-step правил.
- `src/ui/cell_inspector.rs`: Runtime-сборка и позиционирование HUD инспектора клетки как стека отдельных entity-блоков с общей тенью.
- `src/ui/cell_inspector_model.rs`: Модель данных и formatter HUD инспектора клетки, включая config-driven контейнеры, solid-материалы как отдельные блоки и registry-driven отображение состава газа.
- `src/ui/input_field.rs`: Публичные типы text-input и точка сборки input-систем.
- `src/ui/input_field_helpers_block.rs`: Вспомогательная геометрия курсора текста и точный hit-test/каретка через `ComputedTextBlock`.
- `src/ui/input_field_systems_block.rs`: Системы focus/keyboard/render/caret для текстовых полей.
- `src/ui/modal.rs`: Публичные типы reusable modal backdrop subsystem, helper-ы спавна backdrop-слоёв и wiring `ModalPlugin`.
- `src/ui/modal_capture_block.rs`: Snapshot/capture runtime для modal backdrop-ов: offscreen-камера, resize target-а, blur world-snapshot и cache lifecycle.
- `src/ui/modal_runtime_block.rs`: Выбор topmost модалки, cover-layout backdrop-изображений и переключение режимов `PanelFrosted` / `FullscreenBlur`.
- `src/ui/modal_tests_block.rs`: Unit-тесты modal helper-ов, cover-layout и правил refresh/capture для world-snapshot backdrop.
- `src/ui/mod.rs`: UI-плагин и wiring общих UI-систем.
- `src/ui/palette.rs`: Единая палитра цветов UI (панели, меню, текст, input/select, tooltip, HUD и тени HUD).
- `src/ui/panels.rs`: Публичные типы panel-системы и композиция блоков панели.
- `src/ui/panels_manager_block.rs`: Состояние и API PanelManager, hit-rect и управление панелями.
- `src/ui/panels_runtime_block.rs`: Runtime-системы панели: layout, состояние viewport-ов и события заголовка; input scroll делегирован общему `scroll_area`.
- `src/ui/panels_tests_block.rs`: Тесты layout/scroll/stack-поведения панелей.
- `src/ui/scroll_area.rs`: Общий scroll-area runtime для modal/panel viewport-ов: wheel input, drag thumb, click on track, visibility scrollbar и приоритет групп ввода.
- `src/ui/select_field.rs`: Dropdown/select-компонент для UI-панелей и его тесты.
- `src/ui/sim_controls.rs`: UI-контролы симуляции (pause/speed/hotkeys).
- `src/world/grid.rs`: Клеточная сетка мира, материалы, координатные утилиты и тесты.
- `src/world/mod.rs`: Плагин мира и события изменений клеток.
- `src/world/structures.rs`: Unified layer/descriptor-модель структур, `PlacedStructureMap`, rotation, bridge-footprint и pipe-cut state.
- `tmp_size.rs`: Временный локальный вспомогательный Rust-файл для ручных проверок/черновых экспериментов.
