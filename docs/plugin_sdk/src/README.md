# FluxEngine Plugin SDK

Эта документация описывает runtime Plugin SDK FluxEngine. Отдельного SDK-crate пока нет: внешний плагин подключается как DLL через стабильный C-compatible ABI, а Rust-first API внутри движка используется как каноническая модель типов, событий и контрактов.

Сгенерированные reference-разделы обновляются командой:

```powershell
cargo xtask generate-plugin-sdk-docs
```

Генератор строгий. Он завершится ошибкой, если у SDK-facing item:

- нет doc-comment;
- нет обязательной секции вроде `# Fields`, `# Variants` или `# SDK Notes`;
- у публичного поля структуры или варианта enum нет явного описания;
- отсутствует внешний example-snippet для метода, callback, функции, константы или события.

Примеры хранятся отдельно от Rustdoc в каталоге [`examples/`](examples/):

- `examples/methods/*.md`
- `examples/events/*.md`
- `examples/constants/*.md`

Сайт документации собирается командой:

```powershell
cargo xtask build-plugin-sdk-docs
```
