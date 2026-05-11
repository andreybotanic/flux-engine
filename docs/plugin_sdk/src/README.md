# FluxEngine Plugin SDK

Эта документация описывает runtime plugin API FluxEngine. Настоящего отдельного SDK crate пока нет: внешний плагин подключается как DLL через стабильный C-compatible ABI, а Rust-first API в коде движка служит канонической моделью типов, событий и контрактов.

API-разделы генерируются командой:

```powershell
cargo xtask generate-plugin-sdk-docs
```

Генератор строгий: если у SDK-facing item нет doc-comment или обязательной секции вроде `# SDK Example`, генерация завершается ошибкой.

Сайт собирается командой:

```powershell
cargo xtask build-plugin-sdk-docs
```
