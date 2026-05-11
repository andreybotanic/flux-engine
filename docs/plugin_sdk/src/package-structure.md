# Структура plugin package

Expanded plugin root содержит:

```text
<plugin_id>/
|-- manifest.toml
|-- bin/
|   `-- plugin.dll
|-- config/
`-- assets/
```

Packaged plugin — это ZIP-архив с расширением `.fluxplugin` и тем же содержимым внутри архива.

Пути в manifest и archive entries должны быть относительными и безопасными: запрещены absolute roots, `..`, `.`-сегменты и Windows drive prefix.
