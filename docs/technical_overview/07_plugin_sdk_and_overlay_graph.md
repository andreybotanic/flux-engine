## Plugin SDK v5

- Текущий публичный контракт runtime-плагинов живёт в `crates/flux_plugin_sdk`. Это Rust-first SDK: автор плагина реализует `Plugin`, получает `PluginInit`, регистрирует content и подписки через `Registrar<Self>`, а ABI glue генерируется макросом `declare_plugin!(Type)`.
- Внутренний ABI слой полностью вынесен в `crates/flux_plugin_abi`. Все `extern "C"`, `#[repr(C)]`, `Flux*`-структуры, export names и dispatch glue скрыты от пользовательского кода плагина.
- Для стабильности уже собранных runtime DLL любое расширение `#[repr(C)]` host/registrar структур делается только append-only (новые поля добавляются в хвост), иначе старые `.fluxplugin` могут падать из-за смещения callback offsets.
- DLL по-прежнему экспортирует обязательные `flux_plugin_api_version`, `flux_plugin_create`, `flux_plugin_register`, `flux_plugin_dispatch` и `flux_plugin_destroy`, но эти entrypoints создаются SDK автоматически. Плагин больше не экспортирует пользовательские named handlers.
- Loader работает по пайплайну `api_version -> create -> register -> dispatch -> destroy`. Во время `register` движок получает зарегистрированный content и список `PluginSubscriptionRegistration`, строит subscriber map по `PluginEvent`, а в runtime вызывает только подписанные плагины.
- Помимо DLL-path у SDK теперь есть built-in execution backend: `flux.default` поднимается через `flux_plugin_sdk::BuiltinPluginRuntime`, но для ядра это тот же runtime endpoint с теми же subscriptions, typed events и host APIs, что и у внешних ABI-плагинов.
- В отличие от v4, подписка не хранит `handler_name`. На стороне ABI существует один общий `dispatch`, а маршрутизация к конкретному Rust-методу выполняется внутри SDK по сохранённой таблице `event_kind -> handler`.
- Обработчики событий имеют форму `fn(&mut self, &TypedEvent) -> Result<(), PluginError>`. Объект события обязателен, а `FluxPluginHandle` и `FluxRuntimeHost` больше не участвуют в публичной сигнатуре.
- Игровые API доступны через сам объект плагина. `PluginInit` выдаёт долгоживущие proxy-объекты `WorldApi`, `EntityApi`, `GasApi`, `UiApi`, `OverlayApi`, `SaveApi`, `TimeApi`, `InputApi` и `LoggerApi`, которые плагин хранит у себя в полях.
- Эти proxy API работают через скрытый dispatch scope. Перед вызовом обработчика SDK привязывает текущий runtime host, после завершения обработчика очищает scope. Любой вызов API вне разрешённого runtime scope возвращает `PluginError::ApiUnavailable`.
- `WorldApi` теперь отвечает только за чтение мира и координатные helper-ы. Из публичного контракта удалена модель `set_cell_material`: твёрдые клетки и структуры описываются как сущности и редактируются через `EntityApi`.
- `EntityApi` объединяет placement/removal/mutation placeable-объектов, включая `brick`, `metal`, `boundary`, трубы, вентиляции, мосты, `gas source` и `gas sink`. Внутри движка это пока адаптируется к legacy split `WorldGrid + PlacedStructureMap`, но наружу этот split больше не течёт.
- `GasApi` покрывает только операции над свободным газом клетки; `UiApi` ограничен HUD/tool-интеграцией без plugin-owned панелей и без системы уведомлений; `TimeApi` теперь не только читает состояние симуляции, но и умеет менять pause/speed.
- `RuntimeDllPlugin` больше не хранит таблицу named export handlers. Вместо этого на плагин кэшируется один `FluxPluginDispatchFn`, а `runtime_dll_events.rs` только кодирует typed payload и вызывает единый dispatch entrypoint.
- При создании live runtime-экземпляра движок теперь выполняет не только `create`, но и повторный `register` на уже созданном plugin handle. Это нужно, потому что SDK строит свою внутреннюю таблицу `event_kind -> handler` именно во время `register`; без этого плагин считался загруженным, но фактически не реагировал ни на одно событие.
- Runtime host теперь пробрасывает `write_log_fn` и для event-dispatch. Из-за этого `LoggerApi` и сообщения об ошибках из обработчиков больше не теряются во время игры и попадают в stderr-лог с plugin id и уровнем сообщения.
- Sample plugin crates `src/plugins/flux_api_*`, `flux_stage7_sample_content_plugin` и `flux_stage1_sample_plugin` мигрированы на `flux_plugin_sdk`; пользовательский код этих плагинов больше не содержит ручного ABI.
- `xtask` и ручная документация Plugin SDK должны рассматривать `crates/flux_plugin_sdk/src/*` как главный источник user-facing контракта. `src/plugins/abi.rs` в ядре теперь является только engine-side wrapper-слоем над внутренним ABI crate.

### Overlay scene graph foundation

- В SDK добавлен foundation для named DAG-контракта plugin-owned overlays: граф задаёт узлы, зависимости и детерминированный порядок исполнения без special-case веток под конкретный content.
- `OverlayDescriptor` может хранить optional `OverlayGraph`; `Registrar::register_overlay` валидирует graph до регистрации, а engine-side `RuntimeOverlayDescriptor` сохраняет graph для in-process SDK plugins.
- `CoreWorld` не знает о конкретных сущностях вроде pipes/bridges/vents; выбор render-кандидатов идёт через plugin-facing селекторы (`ContentId` + `ContentTag`) и выражения `And/Or/Not`.
- Узлы `RenderSpaceNode`, `RenderTintFieldNode` и отдельный `BackgroundNode` не используются.
- Базовый набор узлов на этапе миграции: `RenderEntitiesNode`, `RenderFreeGasNode`, `RenderImageNode`, `BlendNode`, `MaterialNode`.
- `RenderImageNode` служит универсальным источником plugin-графики (иконки, батчи инстансов, крупные grid-aligned raster-слои вроде температурной карты) и поддерживает placement в координатах клеточной сетки без ручной screen-space математики через камеру.
- Runtime compositor добавлен в `src/render/overlay_graph_runtime.rs`: plugin overlay graph исполняется в детерминированном топологическом порядке, material/blend узлы собирают слойный render-plan, а финальный результат публикуется как активный overlay без knowledge о gameplay-смыслах `pipes`/`temperature`.
- Host API использует только `submit_overlay_graph` (SDK runtime binding + ABI callback), поэтому `RenderOverlay` отправляет declarative graph вместо `RGBA8 full-frame`; frame-based submit path удалён из рабочего runtime/SDK-контракта.
- Пустые image-слои в graph (`RenderImageNode` с `instances.is_empty()`) считаются валидными: это нужно для режимов вроде `F3`, где отдельные слои (например, движущиеся пакеты газа на паузе) могут временно отсутствовать, но сам graph-пайплайн должен оставаться активным без переключения на отдельный frame-path.
- Default plugin `F3/Pipes` мигрирован на graph producer: `flux.default` на событии `RenderOverlay` строит graph с dim-фоном, selector-слоями сущностей, `MaterialNode` для pipe highlight, image-слоями moving flow-content/порт-иконок и отправляет его через `OverlayApi::submit_graph`.
- Temperature demo overlay переведён на graph path: `RenderImageNode` передаёт одну большую `GridLocal` RGBA8 temperature map, а `RenderEntitiesNode` накладывает силуэты solid-сущностей через selector по `ContentTag`.
- Подробный пошаговый план реализации вынесен в `docs/plans/plugin_system/09_overlay_scene_graph_and_compositor.md`.
- В graph-path `F3/Pipes` flow-packets рендерятся через `OverlayPlacement::GridLocal`, поэтому world-space позиции переводятся в grid-local координаты относительно `world_origin()` перед отправкой в graph.
- Фаза анимации flow-packets в `flux.default` берётся напрямую из `PipeFlowVisualState::flow_progress()`: на паузе состояние не очищается, поэтому текущие moving-контейнеры остаются видимыми в той же фазе между сегментами до resume.
- При активном graph-оверлее plugin-режима `GasMainOverlaySprite` скрывается, чтобы core free-gas слой не подмешивался поверх `F3/Pipes` во время анимации.
- `MaterialNode` с `flux.default.overlay.material.pipe_highlight` в graph-композиторе применяется не только к обычным трубам, но и к мостам (`GasPipeBridge`) через собственный world-спрайт моста (`bridge`), без отдельной сегментированной mask-подсветки по трём клеткам.
- Для pipe-структур в graph-композиторе используется не статичный `pipe_mask_00`, а динамический выбор `pipe_mask_##` по `pipe_connection_mask`, чтобы визуальные соединения трубы в `F3` совпадали с фактической топологией соседей (включая сценарии с трубой под мостом).
- `MaterialNode` pipe-highlight также применяется к `Vent` и `GasPipeBridge` через их собственные world-спрайты (`tile_vent`, `bridge`), а не через дополнительные mask-сегменты в клетке пересечения; это сохраняет корректный вид трубы под мостом без ложного визуального объединения в `pipe_mask_15`.
- Для `F3/Pipes` базовая цветовая схема слоя мира теперь выбирается как pipe-режим независимо от legacy-path, а при активном graph `F3` базовые legacy-спрайты мира (`board/backdrop/walls/pipes/vents`) скрываются, чтобы итоговую картинку полностью определял plugin overlay graph.
- В режиме `F3/Pipes` legacy world-слои теперь не участвуют в финальном кадре независимо от paused/running состояния; это исключает влияние `F1`-пайплайна на tint/фон/порядок отрисовки во время симуляции.
- Локальные z-смещения graph-сущностей ограничены внутри бюджета одного слоя (`OVERLAY_LAYER_STEP_Z`), а `gas_in_out` иконки получают дополнительный front-z bias, поэтому порт-иконки стабильно остаются на самом переднем плане `F3`.

### Runtime overlay graph pipeline (актуальное состояние)

- Событие `PluginRuntimeEvent::RenderOverlay` диспатчится в `src/plugins/runtime_dll.rs::dispatch_plugin_overlay_render`; перед dispatch очищается только `PluginOverlayGraphStore`, frame-store в runtime больше не существует.
- Runtime host для DLL и built-in плагинов отдаёт только graph submit callback: старый frame-slot в ABI оставлен как reserved-only для бинарной совместимости, но не используется runtime-логикой.
- Для packaged-плагинов путь `RenderOverlay -> submit_graph` зафиксирован регрессионными тестами в `src/plugins/runtime_dll.rs` (архивный `flux.api_temperature_overlay.fluxplugin`): тест проверяет submit графа на собственный `overlay_id` и отсутствие submit на чужой `overlay_id`.
- Плагин отправляет `OverlayGraph` через `OverlayApi::submit_graph`; graph валидируется в SDK до отправки (`OverlayGraph::validate`) и сериализуется в JSON для ABI callback.
- Engine сохраняет присланный graph в `PluginOverlayGraphStore.graph` на кадр, после чего `sync_overlay_graph_visuals` в `src/render/overlay_graph_runtime.rs`:
  - выбирает graph из `RuntimeOverlayDescriptor.graph` или из `PluginOverlayGraphStore`;
  - строит topological execution order;
  - сводит узлы в слойный `LayerPlan` (`RenderEntities` / `RenderFreeGas` / `RenderImage` / `Blend` / `Material`);
  - материализует результат спавном overlay-entity с жёстким layer budget по `z`.
- Если graph отсутствует или невалиден, отдельного frame-fallback больше нет: `PluginOverlaySprite` принудительно скрывается, и оверлей плагина не рисуется.
