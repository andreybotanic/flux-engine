# Overlay graph pipeline

В SDK v6 plugin-controlled overlay рендерится только через declarative graph.

## Как это работает

1. Плагин регистрирует overlay с `render_policy = PluginControlled` через `Registrar::register_overlay`.
2. На событии `PluginEvent::RenderOverlay` плагин проверяет `RenderOverlayEvent.overlay_id`.
3. Для своего overlay-id плагин собирает `OverlayGraph` и отправляет его через `OverlayApi::submit_graph`.
4. Движок валидирует граф (`OverlayGraph::validate`) и исполняет его как DAG в compositor-е overlay graph runtime.

## Какие узлы использовать

- `RenderEntitiesNode` — выбор и отрисовка сущностей через selector.
- `RenderFreeGasNode` — слой визуализации свободного газа.
- `RenderImageNode` — image instances (`Rgba8`, `SpriteAsset`) с `OverlayPlacement`.
- `BlendNode` — композиция слоёв.
- `MaterialNode` — shader/material-проход по результату зависимостей.

## Важные правила

- Legacy frame-based API удалён из рабочего SDK/runtime API.
- На один `RenderOverlay` нужно отправлять итог через `submit_graph`.
- Плагин обязан игнорировать чужой `overlay_id`, чтобы не перетирать другие overlay-режимы.
- Пустые image-слои (`RenderImageNode` с пустым `instances`) валидны и не должны использоваться как повод для fallback.

## Минимальный шаблон обработчика

```rust
fn on_render_overlay(&mut self, event: &RenderOverlayEvent) -> Result<(), PluginError> {
    if event.overlay_id.as_str() != self.overlay_id.as_str() {
        return Ok(());
    }
    let root = OverlayNodeId::parse("overlay.root").map_err(PluginError::message)?;
    self.overlays.submit_graph(OverlayGraph {
        nodes: vec![OverlayNode {
            id: root.clone(),
            depends_on: Vec::new(),
            kind: OverlayNodeKind::RenderImage(RenderImageNode {
                instances: Vec::new(),
            }),
        }],
        output: root,
    })
}
```