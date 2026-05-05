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
|   `-- gases/               # Конфиги отдельных газов.
|-- docs/                    # Проектная документация.
|-- src/                     # Исходный код Rust.
|   |-- app/                 # Сборка и запуск Bevy-приложения.
|   |-- bin/                 # Вспомогательные бинарники (перф, утилиты).
|   |-- config/              # Загрузка/валидация конфигов в коде.
|   |-- debug/               # Диагностические режимы и метрики.
|   |-- editor/              # Инструменты редактирования мира и UI редактора.
|   |-- input/               # Обработка пользовательского ввода.
|   |-- render/              # Визуализация мира и overlay-режимов.
|   |-- simulation/          # CPU/GPU симуляция газа и parity-инфраструктура.
|   |-- ui/                  # Общие UI-компоненты и панели.
|   `-- world/               # Клеточный мир и структуры Source/Sink.
|-- AGENTS.md                # Правила работы агента.
|-- Cargo.toml               # Манифест проекта.
|-- Cargo.lock               # Lock-файл зависимостей.
`-- tmp_size.rs              # Локальный вспомогательный черновой файл.
```

## Файлы

- `AGENTS.md`: Правила работы агента в этом репозитории.
- `assets/fonts/ui_main.ttf`: Основной UI-шрифт с поддержкой кириллицы для всех текстовых элементов интерфейса.
- `assets/shaders/gas_solver.wgsl`: GPU-шейдер газового шага (WGSL), синхронизированный с CPU-эталоном.
- `assets/sprites/ui/main_menu_background.png`: Отдельный fullscreen-фон главного меню.
- `assets/sprites/ui/select_arrow.png`: UI-спрайт стрелки для выпадающих списков.
- `assets/sprites/ui/silhouette_*.png`: UI-спрайты силуэтов предпросмотра.
- `assets/sprites/ui/tool_*.png`: UI-спрайты иконок инструментов.
- `assets/sprites/world/backdrop_*.png`: Фоновые текстуры мира.
- `assets/sprites/world/tile_*.png`: Спрайты тайлов мира.
- `Cargo.lock`: Зафиксированные версии зависимостей Cargo.
- `Cargo.toml`: Манифест Rust-проекта и зависимости.
- `config/backups/simulation.toml.pre_tuning_20260503_174021.toml`: Резервная копия конфигурации симуляции для отката/сравнения.
- `config/cell_types.toml`: Настройки визуала/параметров типов клеток.
- `config/gases/*.toml`: Конфиги отдельных газов (физические и визуальные параметры).
- `config/simulation.toml`: Основные параметры симуляции и runtime-настройки.
- `docs/CHANGELOG.md`: Краткая история важных изменений проекта.
- `docs/game_overview.md`: Описание игрового процесса и пользовательских механик MVP.
- `docs/project_structure.md`: Карта структуры проекта: дерево папок + зоны ответственности файлов.
- `docs/technical_overview.md`: Техническая архитектура, подсистемы и инженерные ограничения.
- `src/app/mod.rs`: Сборка Bevy-приложения, плагины, backend-инициализация и запуск.
- `src/bin/gas_perf.rs`: Пайплайн перф-бенчмарка газа (CPU/GPU), parity-gate и отчёты.
- `src/config/config_loader_block.rs`: Внутренняя логика чтения/валидации TOML-конфигов.
- `src/config/config_tests_block.rs`: Тесты загрузки и валидации конфигов.
- `src/config/mod.rs`: Публичные конфиг-типы и входная точка загрузки конфигов.
- `src/debug/mod.rs`: Debug-режимы, оверлейные метрики и диагностические ресурсы.
- `src/editor/editor_ui_block.rs`: Runtime-обработка editor UI: tooltip, state sync, панели.
- `src/editor/input_block.rs`: Мышь/кисть/выделение и применение инструментов к миру/газу.
- `src/editor/main_menu_actions_block.rs`: Обработчики действий меню: save/load/new/exit/confirm.
- `src/editor/main_menu_block.rs`: Композиция логики main menu (escape/actions/ui refresh).
- `src/editor/main_menu_escape_block.rs`: Обработка Esc и переходов состояний меню/инструментов.
- `src/editor/main_menu_ui_block.rs`: Обновление состояния и видимости элементов меню.
- `src/editor/mod.rs`: Публичные editor-типы/ресурсы и точка сборки editor-систем.
- `src/editor/overlay_setup_block.rs`: Инициализация визуальных editor-оверлеев.
- `src/editor/ui_setup_block.rs`: Сборка editor-UI: панели, кнопки, поля и привязка виджетов.
- `src/editor/ui_setup_debug_panels_block.rs`: Построение debug-панелей и строк параметров.
- `src/editor/ui_setup_menu_button_factory_block.rs`: Фабрика кнопок модального меню.
- `src/editor/ui_setup_setup_fn_block.rs`: Основная функция первичной сборки editor-UI.
- `src/editor/ui_setup_structure_buttons_block.rs`: Вспомогательные фабрики кнопок инструментов/материалов.
- `src/input/camera.rs`: Управление камерой, зум/пан и тесты корректности якоря.
- `src/input/mod.rs`: Плагин подсистемы ввода и wiring систем ввода.
- `src/lib.rs`: Корневой модуль библиотеки и экспорт подсистем.
- `src/main.rs`: Точка входа бинаря; запускает приложение.
- `src/render/mod.rs`: Плагин рендера и порядок render-систем.
- `src/render/world_view.rs`: Публичные render-системы world view и переключение overlay.
- `src/render/world_view_overlay_block.rs`: Логика overlay-режимов, курсорной сетки и визуальных sync.
- `src/render/world_view_setup_block.rs`: Построение сущностей мира/слоёв и спавн спрайтов.
- `src/render/world_view_tests_block.rs`: Тесты вспомогательной математики рендера.
- `src/save.rs`: Публичный save/load API и типы состояния меню/сессии.
- `src/save_api_block.rs`: Операции верхнего уровня: list/create/overwrite/load snapshot.
- `src/save_gas_io_block.rs`: Чтение/запись gas/gas-structures chunk и маппинг по registry.
- `src/save_meta_io_block.rs`: Метаданные сейва и chunk I/O для мира/служебных структур.
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
- `src/simulation/mod.rs`: Плагин симуляции, ресурсы состояния и orchestration тика.
- `src/simulation/parity.rs`: Публичные parity API и сценарии сравнения CPU/GPU.
- `src/simulation/parity_runtime_block.rs`: Runtime parity-метрики, прогоны сценариев и gate-оценка.
- `src/simulation/parity_tests_block.rs`: Тесты parity-порогов, smoke и GPU-регрессий.
- `src/simulation/runtime_tick_block.rs`: Runtime-шаги симуляции, GPU/CPU подшаги и perf-метрики.
- `src/simulation/simulation_tests_block.rs`: Тесты конфигурации тика и структурных pre-step правил.
- `src/ui/cell_inspector.rs`: Панель инспектора клетки под курсором.
- `src/ui/input_field.rs`: Публичные типы text-input и точка сборки input-систем.
- `src/ui/input_field_helpers_block.rs`: Парсинг и вспомогательная геометрия курсора текста.
- `src/ui/input_field_systems_block.rs`: Системы focus/keyboard/render/caret для текстовых полей.
- `src/ui/mod.rs`: UI-плагин и wiring общих UI-систем.
- `src/ui/palette.rs`: Единая палитра цветов UI (панели, меню, текст, input/select, tooltip, HUD).
- `src/ui/panels.rs`: Публичные типы panel-системы и композиция блоков панели.
- `src/ui/panels_manager_block.rs`: Состояние и API PanelManager, hit-rect и управление панелями.
- `src/ui/panels_runtime_block.rs`: Runtime-системы панели: layout, scroll, события заголовка.
- `src/ui/panels_tests_block.rs`: Тесты layout/scroll/stack-поведения панелей.
- `src/ui/select_field.rs`: Dropdown/select-компонент для UI-панелей и его тесты.
- `src/ui/sim_controls.rs`: UI-контролы симуляции (pause/speed/hotkeys).
- `src/world/gas_structures.rs`: Source/Sink структуры, snapshot и операции размещения.
- `src/world/grid.rs`: Клеточная сетка мира, материалы, координатные утилиты и тесты.
- `src/world/mod.rs`: Плагин мира и события изменений клеток.
- `tmp_size.rs`: Временный локальный вспомогательный Rust-файл для ручных проверок/черновых экспериментов.
