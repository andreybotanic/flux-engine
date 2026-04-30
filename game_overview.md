# Игра FluxEngine (название временное)

## Концепт

Жанры: песочница, симулятор, строительство, выживание.

Игровой мир наполнен разными газами, жидкостями, ископаемыми породами, минералами, растениями иживотными.

Игра 2D, вид "сбоку", т.е. экранные направления вверх/вниз/влево/вправо соответствуют аналогичным игровым направлениям. Ось вперед/назад отсутствует.

## Симуляция

Распространение жидкостей и газов реализуется клеточными автоматами. Реализация клеточных автоматов должна использовать максимальный параллелизм.

Весь игровой мир накладывается на квадратную сетку, в рамках которой происходит вся симуляция.

### Текущий статус MVP

На текущем этапе реализуется первый рабочий каркас игры на Rust + Bevy + wgpu/WGSL.

В MVP реализованы:
- мир размером 102x102 клетки
- непроницаемая рамка по периметру мира
- базовый интерфейс просмотра мира
- панорамирование и масштабирование камеры
- компактный полупрозрачный HUD состояния клетки рядом с курсором
- управление скоростью симуляции (x1, x2, x5) и пауза из UI и с клавиатуры
- отдельный режим отображения газов
- полупрозрачные линии сетки (поверх всех слоёв, z=1.5, alpha≈0.09)
- отладочный режим (клавиша `` ` ``): авто-пауза, пошаговое выполнение клавишей Enter, зелёная рамка (активная клетка) и синяя рамка (выбранный сосед) для следующего шага диффузии

Симуляция в MVP выполняется с фиксированным шагом 30 Гц.

Поверх фиксированного шага добавлен управляемый множитель скорости:
- x1, x2, x5
- переключение скоростей кнопками интерфейса и клавишами < и >
- пауза кнопкой интерфейса и клавишей Space
- старт игры по умолчанию в состоянии паузы

## Газы

Газы имеют следующие характеристики:
- атомная масса
- цвет (может быть бесцветным)

Симуляция газов должна предусматривать смешивание разных газов в одной клетке.

Газу должны вести себя как в реальности:
- заполнять всё предоставленное им пространство
- не проходить через твердые непроничаемые для газа клетки
- легкие газы должна подниматься вверх, а тяжелые - опускаться вниз

Должны быть предусмотрены особые клетки, взаимодействующие с газом:
- источники газа: добавляют определенное количество газа в клетку
- потребители газа: удаляют определенное количество газа из клетки

### Текущий статус MVP газов

На первом шаге реализуется только один газ: водород.

Для первого рабочего прототипа используется только диффузия газа по окрестности фон Неймана порядка 1.

На текущем шаге MVP диффузия водорода переведена на целочисленную модель частиц:
- количество газа хранится как число условных частиц в каждой клетке
- используется недетерминированный блочно-синхронный шаг 3x3:
	на substep выбирается один глобальный индекс 0..8, активируется одна позиция во всех блоках,
	и каждая активная клетка выбирает одного случайного соседа фон Неймана, после чего обмен идет в обе стороны между выбранной парой клеток
- для каждой из двух клеток пары по целочисленному коэффициенту (числитель/знаменатель) вычисляется переносимая доля;
	эта доля передается соседу, а остаток остается в исходной клетке (коэффициент инвертирован относительно прошлой версии)
- переносимая доля округляется до ближайшего целого по стандартным правилам математики
- для малых концентраций, когда ожидаемый перенос меньше 1 частицы, применяется стохастика:
	переносится 1 частица с вероятностью p = diffusionCoefficient * cellParticlesAmount, иначе переносится 0
- коэффициент диффузии задается отдельными целочисленными параметрами (числитель/знаменатель), чтобы в будущем легко настроить его отдельно для каждого газа
- начальное состояние: одна центральная клетка содержит 10000 частиц водорода
- для размеров мира, не кратных 3, разбиение блоков обрабатывается через циклический сдвиг фаз 3x3 по шагам для выравнивания вероятностей активации клеток

Подъем легких газов вверх и опускание тяжелых газов вниз остаются обязательной частью общей модели газов, но будут добавлены следующим этапом.

Визуализация MVP газов:
- основной слой F1: обычный вид мира, водород визуально прозрачный
- слой газов F2: мир и фон отображаются через серый фильтр, концентрация газа показывается насыщенностью цвета клетки; текстура overlay на текущем этапе синхронизируется напрямую из CPU-состояния газа, слабые концентрации усилены визуальной кривой

Техническая фиксация для корректного обновления overlay:
- текстуры газа создаются с `MAIN_WORLD | RENDER_WORLD`, чтобы изменения цвета из main world корректно попадали в рендер
- добавлены unit-тесты на инициализацию газа, блочно-синхронную активацию, сохранение суммарного количества частиц и равномерность активации при циклическом сдвиге


## Жидкости
Not yet

## Ископаемые породы и минералы
Not yet

## Растения
Not yet

## Животные
Not yet
## Changelog
- 2026-04-29: Added `.gitignore` for the Rust/Bevy project to ignore build artifacts (`target/`), IDE folders, and OS-generated temporary files.
- 2026-04-29: Implemented world editing toolbars and editor workflows (bottom main toolbar with Build/Erase, top debug toolbar with Add Gas/Clear and toggle visibility, right gas settings panel with amount/replace), drag-based solid painting/erasing, rectangle gas apply/clear, blueprint and selection overlays, and erase cursor marker. Solid cells are now truly impermeable for gas simulation and wall visuals update dynamically when grid cells change.
- 2026-04-29: Fixed Bevy runtime system-param conflict (`B0001`) in editor UI systems by separating mutable `Visibility` and `Text` accesses with `ParamSet`, including cursor overlay updates, eliminating startup panic.
- 2026-04-29: Improved editor UX: made Build blueprint more transparent, added per-cell destroy highlight for Erase tool, replaced gas amount +/- controls with a direct numeric input field (keyboard digits + Backspace + Enter/Escape), and configured Bevy `AssetPlugin` to use absolute `<project>/assets` path to remove runtime shader path errors when launching release binaries outside project root.
- 2026-04-30: Refined editor interaction: Build blueprint now appears only while LMB is not pressed and uses lower transparency; Erase now highlights the exact target cell under cursor; gas amount control switched to keyboard numeric input with visible caret/focus state; gas overlay now refreshes immediately after editor gas mutations by bumping `SimulationStep`; asset loading remains pinned to absolute project `assets` path to avoid release-run shader path errors.
- 2026-04-30: Fixed gas amount input caret rendering in editor UI: while focused, the field now explicitly shows a visible text cursor (`|`) at the input end.
- 2026-04-30: Reworked gas amount entry into a reusable full-featured text input subsystem (`ui/input_field.rs`): supports cursor placement by mouse click, ArrowLeft/ArrowRight/Home/End navigation, Backspace/Delete editing, per-field allowed-character policy, and pluggable parser to internal value (`String` or `u32` range). Editor gas amount now uses this shared input field component.
- 2026-04-30: Fixed `B0001` panic in reusable text input system by splitting conflicting `TextInputField` queries in `focus_text_input_on_click` into a `ParamSet` (read query for click detection + separate mutable query for focus/cursor updates).
- 2026-04-30: Replaced symbolic text-input caret with a real UI caret entity in ui/input_field.rs: caret is now rendered between characters (not as part of the text), supports mouse placement between glyphs and arrow-key movement, and blinks like a standard input control.
