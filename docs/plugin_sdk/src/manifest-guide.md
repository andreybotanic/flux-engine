# Manifest

Минимальный `manifest.toml`:

```toml
id = "flux.example"
display_name = "Example Plugin"
version = "0.1.0"
api_version = 3
dll = "bin/example.dll"
configs = "config"
assets = "assets"
content = false
description = "Small runtime plugin example."
```

`id` должен быть canonical lowercase identifier. `version` должен быть semver. `api_version` должен точно совпадать с версией ABI, поддерживаемой движком.

`content = true` означает, что plugin может быть required content provider для save/load gate. Non-content plugins не блокируют загрузку мира при отсутствии.
