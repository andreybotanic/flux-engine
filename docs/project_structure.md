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
|   `-- structures/          # Конфиги appearance-метаданных стен и структур.
|-- docs/                    # Проектная документация.
|-- src/                     # Исходный код Rust.
|   |-- app/                 # Сборка и запуск Bevy-приложения.
|   |-- bin/                 # Вспомогательные бинарники (перф, утилиты).
|   |-- config/              # Загрузка/валидация конфигов в коде.
|   |-- debug/               # Диагностические режимы и метрики.
|   |-- editor/              # Инструменты редактирования мира, газа и pipe-сети.
|   |-- input/               # Обработка пользовательского ввода.
|   |-- render/              # Визуализация мира, pipe-layer и overlay-режимов.
|   |-- simulation/          # CPU/GPU симуляция газа, pipe pre-step и parity-инфраструктура.
|   |   `-- pipes/           # Внутренние модули pressure/fixtures/solver/test-инфраструктуры труб.
|   |-- ui/                  # Общие UI-компоненты и панели.
|   `-- world/               # Клеточный мир, unified structures и legacy-модули миграции.
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
- `config/cell_types.toml`: Настройки визуала/параметров типов клеток.
- `config/gases/*.toml`: Конфиги отдельных газов (физические и визуальные параметры).
- `config/structures/*.toml`: Конфиги appearance-метаданных встроенных стен и структур (`draw_priority`, `size_in_cells`).
- `config/simulation.toml`: Основные параметры симуляции и runtime-настройки, включая секцию `[pipe]` для pressure-driven труб.
- `docs/CHANGELOG.md`: Краткая история важных изменений проекта.
- `docs/game_overview.md`: Описание игрового процесса и пользовательских механик MVP.
- `docs/project_structure.md`: Карта структуры проекта: дерево папок + зоны ответственности файлов.
- `docs/technical_overview.md`: Техническая архитектура, подсистемы и инженерные ограничения.
- `src/app/mod.rs`: Сборка Bevy-приложения, плагины, backend-инициализация и запуск.
- `src/bin/generate_pipe_scenario_saves.rs`: Вспомогательный бинарник, который пересоздаёт стартовые save-slots для пяти эталонных pipe-сценариев через штатный save API.
- `src/bin/gas_perf.rs`: Пайплайн перф-бенчмарка газа (CPU/GPU), parity-gate и отчёты.
- `src/bin/migrate_save_previews.rs`: Временный служебный бинарник для миграции старых save-slots: прогоняет штатный load + offscreen preview capture и дозаписывает `preview.png`/meta schema `5`.
- `src/config/config_loader_block.rs`: Внутренняя логика чтения/валидации TOML-конфигов, включая `config/structures/*.toml`.
- `src/config/config_tests_block.rs`: Тесты загрузки и валидации конфигов.
- `src/config/mod.rs`: Публичные конфиг-типы, runtime-реестры visual/layout-метаданных и входная точка загрузки конфигов.
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
- `src/lib.rs`: Корневой модуль библиотеки и экспорт подсистем.
- `src/main.rs`: Точка входа бинаря; запускает приложение.
- `src/render/mod.rs`: Плагин рендера и порядок render-систем, включая pipe visuals.
- `src/render/pipe_highlight_material.rs`: Кастомный `Material2d` и helper-логика для shader-подсветки труб в `F3`.
- `src/render/save_preview.rs`: Offscreen preview pipeline для save-slots: отдельная камера, screenshot capture, PNG-запись и восстановление UI/overlay состояния после кадра.
- `src/render/world_view.rs`: Публичные render-системы world view, config-driven appearance z-order и layer-based pipe/bridge visuals.
- `src/render/world_view_overlay_block.rs`: Логика overlay-режимов `F1/F2/F3`, курсорной сетки, multi-container pipe gas-square sizing, flow-packet анимации и фильтрации визуального шума для пакетов `< 5` частиц.
- `src/render/world_view_setup_block.rs`: Построение сущностей мира/слоёв, config-driven z-order стен/структур и спавн визуалов из `PlacedStructureMap`.
- `src/render/world_view_tests_block.rs`: Тесты вспомогательной математики рендера.
- `src/save.rs`: Публичный save/load API, типы состояния меню/сессии и queue/event контракты preview-capture.
- `src/save_api_block.rs`: Операции верхнего уровня: list/create/overwrite/load snapshot и canonical preview-path для slot-а.
- `src/save_gas_io_block.rs`: Чтение/запись gas, unified placed-structures и node-based pipe-gas chunk, плюс миграция schema `3`.
- `src/save_meta_io_block.rs`: Метаданные сейва, версия схемы, preview-chunk `png_v1` и точечный patch helper для preview-миграции.
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
- `src/ui/cell_inspector.rs`: Панель инспектора клетки под курсором, включая world-gas, pressure HUD и список pipe-контейнеров для труб/моста.
- `src/ui/input_field.rs`: Публичные типы text-input и точка сборки input-систем.
- `src/ui/input_field_helpers_block.rs`: Вспомогательная геометрия курсора текста и точный hit-test/каретка через `ComputedTextBlock`.
- `src/ui/input_field_systems_block.rs`: Системы focus/keyboard/render/caret для текстовых полей.
- `src/ui/mod.rs`: UI-плагин и wiring общих UI-систем.
- `src/ui/palette.rs`: Единая палитра цветов UI (панели, меню, текст, input/select, tooltip, HUD).
- `src/ui/panels.rs`: Публичные типы panel-системы и композиция блоков панели.
- `src/ui/panels_manager_block.rs`: Состояние и API PanelManager, hit-rect и управление панелями.
- `src/ui/panels_runtime_block.rs`: Runtime-системы панели: layout, состояние viewport-ов и события заголовка; input scroll делегирован общему `scroll_area`.
- `src/ui/panels_tests_block.rs`: Тесты layout/scroll/stack-поведения панелей.
- `src/ui/scroll_area.rs`: Общий scroll-area runtime для modal/panel viewport-ов: wheel input, drag thumb, click on track, visibility scrollbar и приоритет групп ввода.
- `src/ui/select_field.rs`: Dropdown/select-компонент для UI-панелей и его тесты.
- `src/ui/sim_controls.rs`: UI-контролы симуляции (pause/speed/hotkeys).
- `src/world/gas_structures.rs`: Legacy Source/Sink grid и snapshot schema `2/3`, сохранённый для backward-compatible загрузки и старых unit-тестов.
- `src/world/grid.rs`: Клеточная сетка мира, материалы, координатные утилиты и тесты.
- `src/world/mod.rs`: Плагин мира и события изменений клеток.
- `src/world/pipes.rs`: Legacy `PipeGrid`/`PipeLayoutSnapshot`, сохранённые для schema `3` миграции и регрессионных тестов.
- `src/world/structures.rs`: Unified layer/descriptor-модель структур, `PlacedStructureMap`, rotation, bridge-footprint и pipe-cut state.
- `tmp_size.rs`: Временный локальный вспомогательный Rust-файл для ручных проверок/черновых экспериментов.
