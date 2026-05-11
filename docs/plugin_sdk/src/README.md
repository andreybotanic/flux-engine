# FluxEngine Plugin SDK

Эта документация описывает единственный публичный runtime Plugin SDK FluxEngine.
Для автора DLL-плагина каноническая поверхность API состоит из:

- `FluxHostApi` для `flux_plugin_create`
- `FluxRegistrar` для `flux_plugin_register`
- `FluxRuntimeHost` для runtime-событий
- typed event payload structs и `FluxEventKind`
- descriptor structs, export names и ABI-совместимые базовые типы

Важно: generated reference намеренно не документирует внутренние или legacy-параллели вроде `WorldApi`, `WorldApiMut` и raw host callback aliases. Если у ABI-структуры есть wrapper-метод, публичный SDK показывает именно этот метод.

Сгенерированные reference-разделы обновляются командой:

```powershell
cargo xtask generate-plugin-sdk-docs
```

Генератор строгий. Он завершится ошибкой, если у SDK-facing item:

- нет doc-comment;
- нет обязательной секции вроде `# Fields`, `# Variants` или `# SDK Notes`;
- у публичного поля структуры или варианта enum нет явного описания;
- отсутствует внешний example-snippet для метода, функции, константы или события.

Примеры хранятся отдельно от Rustdoc в каталоге [`examples/`](examples/):

- `examples/methods/*.md`
- `examples/events/*.md`
- `examples/constants/*.md`

Сайт документации собирается командой:

```powershell
cargo xtask build-plugin-sdk-docs
```
