# Этап 5: substances и gas core

## Проверка предпосылок

Перед началом этапа убедиться:
- этап 4 завершён;
- default plugin регистрирует текущий content через IDs;
- текущие структуры и overlay работают без изменения поведения;
- registry умеет различать content и non-content plugin.

Если default plugin ещё не является источником текущего content, этот этап не начинать.

## Будущие зоны изменений

Ожидаемые зоны кода:
- `src/config/mod.rs`;
- `src/simulation/gas.rs`;
- `src/simulation/discrete_step*.rs`;
- `src/simulation/gpu_solver*.rs`;
- `assets/shaders/gas_solver.wgsl`;
- `src/ui/cell_inspector_model.rs`;
- `src/render/world_view*.rs`;
- `src/save*.rs` только если нужен промежуточный snapshot adapter;
- `docs/technical_overview.md`;
- `docs/project_structure.md`.

## Что реализовать

1. Ввести `SubstanceId` и `SubstanceDefinition`.

2. Перенести текущие газы в default plugin:
   - `flux.default.substance.h2`;
   - `flux.default.substance.o2`;
   - `flux.default.substance.co2`.

3. Оставить физику газа в ядре:
   - particle storage;
   - buoyancy;
   - CPU step;
   - GPU step;
   - pipe pressure helpers.

4. Plugin substance может задавать только данные:
   - label;
   - molecular mass;
   - visual color;
   - поддерживаемые substance flags.

5. `GasRegistry` заменить или обернуть так, чтобы runtime order строился из `SubstanceId`, но CPU/GPU буферы продолжали работать по compact index.

6. Сохранить инвариант: CPU-алгоритм является единственным эталоном. Любые изменения данных, которые влияют на GPU, должны передаваться в WGSL без самостоятельных алгоритмических отличий.

7. HUD и render должны брать labels/colors из substance registry, а не из hardcoded gas definitions.

## Edge cases

- Плагин добавляет газ с duplicate `SubstanceId`.
- Плагин добавляет substance без molecular mass.
- Molecular mass не finite или `<= 0`.
- Цвет не в диапазоне `0..=1`.
- Порядок compact indices изменился между сохранениями.
- GPU получает registry с количеством газов больше текущих тестовых значений.

## Тесты этапа

Сначала запускать узкие тесты:
- substance registry validation;
- `GasField::from_registry`;
- HUD composition formatting через plugin substances;
- save-mapping helper, если он затронут.

Если менялись CPU/GPU буферы или shader:
- запустить связанные CPU gas tests;
- запустить быстрые parity smoke tests;
- только после успешного smoke запускать полные нужные parity scenarios.

После этапа выполнить `cargo build --release`, запустить release-версию и проверить логи.

## Заметка для заказчика

Запусти игру с включённым тестовым плагином, который добавляет новый газ, например `Neon`. В `New Game` открой инструмент `Add Gas` или `Gas Source`: новый газ должен появиться в выпадающем списке рядом с `H2/O2/CO2`. Добавь его в мир, переключись в `F2` и наведи HUD на область газа: название, цвет и состав должны идти из plugin substance registry. Затем отключи тестовый газовый плагин, запусти игру снова и убедись, что базовые `H2/O2/CO2` из default plugin всё ещё работают.

## Критерии успешности

- Текущие `H2/O2/CO2` приходят из default plugin.
- Добавление нового газа через registry не требует правок HUD/render списков.
- CPU/GPU parity не ухудшилась.
- Визуально `F1/F2` и HUD работают как до этапа.
