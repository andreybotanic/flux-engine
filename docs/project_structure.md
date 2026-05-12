# РЎС‚СЂСѓРєС‚СѓСЂР° РїСЂРѕРµРєС‚Р° FluxEngine

Р”РѕРєСѓРјРµРЅС‚ С„РёРєСЃРёСЂСѓРµС‚ Р·РѕРЅСѓ РѕС‚РІРµС‚СЃС‚РІРµРЅРЅРѕСЃС‚Рё РїР°РїРѕРє Рё С„Р°Р№Р»РѕРІ СЂРµРїРѕР·РёС‚РѕСЂРёСЏ. РћР±РЅРѕРІР»СЏР№С‚Рµ РµРіРѕ РїСЂРё РёР·РјРµРЅРµРЅРёРё СЃС‚СЂСѓРєС‚СѓСЂС‹ РїСЂРѕРµРєС‚Р°.

## РџР°РїРєРё (ASCII-РґРµСЂРµРІРѕ)

```text
FluxEngine/
|-- .cargo/                  # Р›РѕРєР°Р»СЊРЅС‹Рµ cargo alias-С‹ РїСЂРѕРµРєС‚Р°.
|-- assets/                  # Core-РіСЂР°С„РёРєР° Рё С€РµР№РґРµСЂРЅС‹Рµ СЂРµСЃСѓСЂСЃС‹ РїСЂРёР»РѕР¶РµРЅРёСЏ.
|   |-- fonts/               # UI-С€СЂРёС„С‚С‹, Р·Р°РіСЂСѓР¶Р°РµРјС‹Рµ С‡РµСЂРµР· AssetServer.
|   |-- shaders/             # Core WGSL-С€РµР№РґРµСЂС‹ РІС‹С‡РёСЃР»РµРЅРёР№.
|   `-- sprites/             # РЎРїСЂР°Р№С‚С‹ UI Рё РјРёСЂР°.
|       |-- ui/              # Core UI-СЌР»РµРјРµРЅС‚С‹ РјРµРЅСЋ/СЃРµР»РµРєС‚РѕРІ/РѕР±С‰РёС… РёРЅСЃС‚СЂСѓРјРµРЅС‚РѕРІ.
|       `-- world/           # Core С„РѕРЅРѕРІС‹Рµ С‚РµРєСЃС‚СѓСЂС‹ РјРёСЂР°.
|-- config/                  # Core TOML-РєРѕРЅС„РёРіРё СЃРёРјСѓР»СЏС†РёРё/free-gas РїРѕРІРµРґРµРЅРёСЏ.
|   |-- backups/             # Р РµР·РµСЂРІРЅС‹Рµ РєРѕРїРёРё РєРѕРЅС„РёРіРѕРІ.
|   `-- simulation.toml      # Core runtime-РЅР°СЃС‚СЂРѕР№РєРё Р±РµР· default-plugin content.
|-- crates/                  # РћС‚РґРµР»СЊРЅС‹Рµ workspace-crate-С‹ РїСѓР±Р»РёС‡РЅРѕРіРѕ Plugin SDK Рё РІРЅСѓС‚СЂРµРЅРЅРµРіРѕ ABI.
|   |-- flux_plugin_abi/     # Р’РЅСѓС‚СЂРµРЅРЅРёР№ ABI/glue crate РґР»СЏ runtime DLL handshake Рё dispatch.
|   `-- flux_plugin_sdk/     # РџСѓР±Р»РёС‡РЅС‹Р№ Rust-first SDK РґР»СЏ Р°РІС‚РѕСЂРѕРІ runtime-РїР»Р°РіРёРЅРѕРІ.
|-- docs/                    # РџСЂРѕРµРєС‚РЅР°СЏ РґРѕРєСѓРјРµРЅС‚Р°С†РёСЏ.
|   |-- plugin_sdk/          # mdBook-СЃР°Р№С‚ Plugin SDK СЃ СЂСѓС‡РЅС‹РјРё guide-РіР»Р°РІР°РјРё Рё generated API reference.
|   `-- plans/               # РџР»Р°РЅРѕРІС‹Рµ РґРѕРєСѓРјРµРЅС‚С‹ Р±СѓРґСѓС‰РёС… РєСЂСѓРїРЅС‹С… РёР·РјРµРЅРµРЅРёР№.
|       `-- plugin_system/   # Roadmap Рё СЌС‚Р°РїРЅС‹Рµ РїР»Р°РЅС‹ РїРµСЂРµС…РѕРґР° РЅР° runtime-РїР»Р°РіРёРЅС‹.
|-- plugins/                 # Runtime drop-in РєР°С‚Р°Р»РѕРі packaged plugins (`*.fluxplugin`) СЂСЏРґРѕРј СЃ РёРіСЂРѕР№.
|-- plugins_dev/             # Runtime dev-РєР°С‚Р°Р»РѕРі expanded plugin-РїР°РїРѕРє `plugins_dev/<plugin_id>/`.
|-- src/                     # РСЃС…РѕРґРЅС‹Р№ РєРѕРґ Rust.
|   |-- app/                 # РЎР±РѕСЂРєР° Рё Р·Р°РїСѓСЃРє Bevy-РїСЂРёР»РѕР¶РµРЅРёСЏ.
|   |-- bin/                 # Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Рµ Р±РёРЅР°СЂРЅРёРєРё (РїРµСЂС„, СѓС‚РёР»РёС‚С‹).
|   |-- config/              # Р—Р°РіСЂСѓР·РєР°/РІР°Р»РёРґР°С†РёСЏ РєРѕРЅС„РёРіРѕРІ РІ РєРѕРґРµ.
|   |-- debug/               # Р”РёР°РіРЅРѕСЃС‚РёС‡РµСЃРєРёРµ СЂРµР¶РёРјС‹ Рё РјРµС‚СЂРёРєРё.
|   |-- editor/              # РРЅСЃС‚СЂСѓРјРµРЅС‚С‹ СЂРµРґР°РєС‚РёСЂРѕРІР°РЅРёСЏ РјРёСЂР°, РіР°Р·Р° Рё pipe-СЃРµС‚Рё.
|   |-- input/               # РћР±СЂР°Р±РѕС‚РєР° РїРѕР»СЊР·РѕРІР°С‚РµР»СЊСЃРєРѕРіРѕ РІРІРѕРґР°.
|   |-- plugins/             # Runtime plugin facade РїР»СЋСЃ in-project plugin-РїР°РїРєРё.
|   |   |-- api/              # Core Rust-first Plugin API contracts used by the generated Plugin SDK reference.
|   |   |-- default_plugin/   # Built-in locked `flux.default`: РєРѕРґ, configs, assets Рё pipe-runtime.
|   |   |-- flux_stage1_sample_plugin/        # Tracked sample non-content DLL-РїР»Р°РіРёРЅ РґР»СЏ ABI/e2e-С‚РµСЃС‚РѕРІ.
|   |   `-- flux_stage7_sample_content_plugin/ # Tracked sample content DLL-РїР»Р°РіРёРЅ СЃ Neon gas.
|   |-- render/              # Р’РёР·СѓР°Р»РёР·Р°С†РёСЏ РјРёСЂР°, pipe-layer Рё overlay-СЂРµР¶РёРјРѕРІ.
|   |-- simulation/          # CPU/GPU СЃРёРјСѓР»СЏС†РёСЏ СЃРІРѕР±РѕРґРЅРѕРіРѕ РіР°Р·Р° Рё parity-РёРЅС„СЂР°СЃС‚СЂСѓРєС‚СѓСЂР°.
|   |-- ui/                  # РћР±С‰РёРµ UI-РєРѕРјРїРѕРЅРµРЅС‚С‹ Рё РїР°РЅРµР»Рё.
|   `-- world/               # РљР»РµС‚РѕС‡РЅС‹Р№ РјРёСЂ Рё unified structures.
|-- xtask/                   # Cargo helper crate РґР»СЏ СЃР±РѕСЂРєРё Рё СѓРїР°РєРѕРІРєРё sample runtime-РїР»Р°РіРёРЅРѕРІ.
|-- AGENTS.md                # РџСЂР°РІРёР»Р° СЂР°Р±РѕС‚С‹ Р°РіРµРЅС‚Р°.
|-- Cargo.toml               # РњР°РЅРёС„РµСЃС‚ РїСЂРѕРµРєС‚Р° Рё workspace.
|-- Cargo.lock               # Lock-С„Р°Р№Р» Р·Р°РІРёСЃРёРјРѕСЃС‚РµР№.
`-- tmp_size.rs              # Р›РѕРєР°Р»СЊРЅС‹Р№ РІСЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Р№ С‡РµСЂРЅРѕРІРѕР№ С„Р°Р№Р».
```

## Р¤Р°Р№Р»С‹

- `AGENTS.md`: РџСЂР°РІРёР»Р° СЂР°Р±РѕС‚С‹ Р°РіРµРЅС‚Р° РІ СЌС‚РѕРј СЂРµРїРѕР·РёС‚РѕСЂРёРё.
- `.cargo/config.toml`: Р›РѕРєР°Р»СЊРЅС‹Р№ cargo alias `cargo xtask` РґР»СЏ Р·Р°РїСѓСЃРєР° helper-crate-Р° `xtask`.
- `.gitignore`: РРіРЅРѕСЂРёСЂСѓРµС‚ runtime artifacts Рё РЅРѕРІС‹Рµ `src/plugins/*/` in-project plugin-РїР°РїРєРё; tracked РёСЃРєР»СЋС‡РµРЅРёСЏ вЂ” core `src/plugins/api/`, `default_plugin`, API demo plugins, `flux_stage1_sample_plugin`, `flux_stage7_sample_content_plugin`.
- `assets/fonts/ui_main.ttf`: РћСЃРЅРѕРІРЅРѕР№ UI-С€СЂРёС„С‚ СЃ РїРѕРґРґРµСЂР¶РєРѕР№ РєРёСЂРёР»Р»РёС†С‹ РґР»СЏ РІСЃРµС… С‚РµРєСЃС‚РѕРІС‹С… СЌР»РµРјРµРЅС‚РѕРІ РёРЅС‚РµСЂС„РµР№СЃР°.
- `assets/shaders/gas_solver.wgsl`: GPU-С€РµР№РґРµСЂ РіР°Р·РѕРІРѕРіРѕ С€Р°РіР° (WGSL), СЃРёРЅС…СЂРѕРЅРёР·РёСЂРѕРІР°РЅРЅС‹Р№ СЃ CPU-СЌС‚Р°Р»РѕРЅРѕРј.
- `assets/sprites/ui/main_menu_background.png`: РћС‚РґРµР»СЊРЅС‹Р№ fullscreen-С„РѕРЅ РіР»Р°РІРЅРѕРіРѕ РјРµРЅСЋ.
- `assets/sprites/ui/select_arrow.png`: UI-СЃРїСЂР°Р№С‚ СЃС‚СЂРµР»РєРё РґР»СЏ РІС‹РїР°РґР°СЋС‰РёС… СЃРїРёСЃРєРѕРІ.
- `assets/sprites/ui/tool_build.png`, `tool_erase.png`, `tool_add_gas.png`, `tool_clear_gas.png`: Core UI-СЃРїСЂР°Р№С‚С‹ РѕР±С‰РёС… РёРЅСЃС‚СЂСѓРјРµРЅС‚РѕРІ; content-specific tool icons Р»РµР¶Р°С‚ РІ default plugin assets.
- `assets/sprites/world/backdrop_noise.png`: Core С„РѕРЅРѕРІР°СЏ С‚РµРєСЃС‚СѓСЂР° РјРёСЂР°; default-owned С‚Р°Р№Р»С‹/СЃС‚СЂСѓРєС‚СѓСЂС‹ Р»РµР¶Р°С‚ РІ default plugin assets.
- `Cargo.lock`: Р—Р°С„РёРєСЃРёСЂРѕРІР°РЅРЅС‹Рµ РІРµСЂСЃРёРё Р·Р°РІРёСЃРёРјРѕСЃС‚РµР№ Cargo.
- `Cargo.toml`: РњР°РЅРёС„РµСЃС‚ Rust-РїСЂРѕРµРєС‚Р°, workspace Рё Р·Р°РІРёСЃРёРјРѕСЃС‚Рё; РѕСЃРЅРѕРІРЅРѕР№ crate, `xtask`, `crates/flux_plugin_sdk` Рё `crates/flux_plugin_abi` РІС…РѕРґСЏС‚ РІ workspace, sample plugin crates Р¶РёРІСѓС‚ РїРѕРґ `src/plugins/*` Рё СЃРѕР±РёСЂР°СЋС‚СЃСЏ РѕС‚РґРµР»СЊРЅРѕ С‡РµСЂРµР· `xtask`.
- `crates/flux_plugin_abi/Cargo.toml`: РњР°РЅРёС„РµСЃС‚ РІРЅСѓС‚СЂРµРЅРЅРµРіРѕ ABI crate-Р° РґР»СЏ runtime plugin handshake.
- `crates/flux_plugin_abi/src/events.rs`: Р’РЅСѓС‚СЂРµРЅРЅРёРµ ABI payload-СЃС‚СЂСѓРєС‚СѓСЂС‹ СЃРѕР±С‹С‚РёР№ Рё mapping raw event kind values.
- `crates/flux_plugin_abi/src/ffi.rs`: C-compatible `Flux*` ABI-СЃС‚СЂСѓРєС‚СѓСЂС‹, callback typedef-С‹ Рё export-name РєРѕРЅСЃС‚Р°РЅС‚С‹ РґР»СЏ СЃРєСЂС‹С‚РѕРіРѕ DLL-РєРѕРЅС‚СЂР°РєС‚Р°.
- `crates/flux_plugin_abi/src/lib.rs`: РўРѕС‡РєР° РІС…РѕРґР° РІРЅСѓС‚СЂРµРЅРЅРµРіРѕ ABI crate-Р° Рё re-export РµРіРѕ РјРѕРґСѓР»РµР№.
- `crates/flux_plugin_sdk/Cargo.toml`: РњР°РЅРёС„РµСЃС‚ РїСѓР±Р»РёС‡РЅРѕРіРѕ Rust-first Plugin SDK.
- `crates/flux_plugin_sdk/src/api.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ proxy API РїР»Р°РіРёРЅР° (`WorldApi`, `EntityApi`, `GasApi`, `UiApi`, `OverlayApi`, `SaveApi`, `TimeApi`, `InputApi`, `LoggerApi`).
- `crates/flux_plugin_sdk/src/descriptors.rs`: Typed descriptor-С‹ SDK РґР»СЏ СЃСѓС‰РЅРѕСЃС‚РµР№, РіР°Р·РѕРІ, overlay, tool Рё save chunk.
- `crates/flux_plugin_sdk/src/dispatch_state_builder.rs`: Р’РЅСѓС‚СЂРµРЅРЅРёР№ builder typed dispatch-state РёР· ABI payload РґР»СЏ СЂР°Р±РѕС‚С‹ proxy API РІРѕ РІСЂРµРјСЏ handler-РІС‹Р·РѕРІР°.
- `crates/flux_plugin_sdk/src/error.rs`: `PluginError` Рё Р±Р°Р·РѕРІС‹Рµ РѕС€РёР±РєРё РїСѓР±Р»РёС‡РЅРѕРіРѕ SDK.
- `crates/flux_plugin_sdk/src/events.rs`: Typed runtime events SDK, `PluginEvent` Рё ABI decode logic, СЃРєСЂС‹С‚Р°СЏ РѕС‚ plugin author-Р° Р·Р° trait-СЃР»РѕРµРј.
- `crates/flux_plugin_sdk/src/ids.rs`: Typed identifier wrapper-С‹ SDK Рё Р±Р°Р·РѕРІС‹Рµ РіРµРѕРјРµС‚СЂРёС‡РµСЃРєРёРµ helper-С‚РёРїС‹.
- `crates/flux_plugin_sdk/src/lib.rs`: РџСѓР±Р»РёС‡РЅР°СЏ С‚РѕС‡РєР° РІС…РѕРґР° SDK, re-export-С‹ Рё macro `declare_plugin!`.
- `crates/flux_plugin_sdk/src/plugin.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ `Plugin`, `PluginInit` Рё СЃРєСЂС‹С‚С‹Р№ `PluginRuntime`, РєРѕС‚РѕСЂС‹Р№ СЃРІСЏР·С‹РІР°РµС‚ Rust-РїР»Р°РіРёРЅ СЃ РІРЅСѓС‚СЂРµРЅРЅРёРј ABI dispatch.
- `crates/flux_plugin_sdk/src/registrar.rs`: `Registrar<Self>`, typed `subscribe(...)` Рё РІРЅСѓС‚СЂРµРЅРЅСЏСЏ С‚Р°Р±Р»РёС†Р° Р·Р°СЂРµРіРёСЃС‚СЂРёСЂРѕРІР°РЅРЅС‹С… Rust-РѕР±СЂР°Р±РѕС‚С‡РёРєРѕРІ.
- `crates/flux_plugin_sdk/src/runtime_host.rs`: Internal SDK host-contract (`RuntimeHostBinding`, `RuntimeHostFns`, save/time snapshots) shared by ABI and built-in runtime execution.
- `crates/flux_plugin_sdk/src/scope.rs`: Р’РЅСѓС‚СЂРµРЅРЅРёР№ dispatch scope SDK, РєРѕС‚РѕСЂС‹Р№ РІСЂРµРјРµРЅРЅРѕ РїСЂРёРІСЏР·С‹РІР°РµС‚ proxy API Рє С‚РµРєСѓС‰РµРјСѓ runtime host.
- `config/backups/simulation.toml.pre_tuning_20260503_174021.toml`: Р РµР·РµСЂРІРЅР°СЏ РєРѕРїРёСЏ РєРѕРЅС„РёРіСѓСЂР°С†РёРё СЃРёРјСѓР»СЏС†РёРё РґР»СЏ РѕС‚РєР°С‚Р°/СЃСЂР°РІРЅРµРЅРёСЏ.
- `config/simulation.toml`: Core-РїР°СЂР°РјРµС‚СЂС‹ СЃРёРјСѓР»СЏС†РёРё/free-gas Рё РІРёР·СѓР°Р»РёР·Р°С†РёРё РіР°Р·Р°; pipe-runtime РЅР°СЃС‚СЂРѕР№РєРё default plugin-Р° РІС‹РЅРµСЃРµРЅС‹ РѕС‚РґРµР»СЊРЅРѕ.
- `docs/CHANGELOG.md`: РљСЂР°С‚РєР°СЏ РёСЃС‚РѕСЂРёСЏ РІР°Р¶РЅС‹С… РёР·РјРµРЅРµРЅРёР№ РїСЂРѕРµРєС‚Р°.
- `docs/game_overview.md`: РћРїРёСЃР°РЅРёРµ РёРіСЂРѕРІРѕРіРѕ РїСЂРѕС†РµСЃСЃР° Рё РїРѕР»СЊР·РѕРІР°С‚РµР»СЊСЃРєРёС… РјРµС…Р°РЅРёРє MVP.
- `docs/plugin_sdk/book.toml`: РљРѕРЅС„РёРіСѓСЂР°С†РёСЏ mdBook-СЃР°Р№С‚Р° Plugin SDK; build output РЅР°РїСЂР°РІР»РµРЅ РІ `target/plugin_sdk_docs`, Р° sidebar folding РІРєР»СЋС‡С‘РЅ РґР»СЏ collapsed-by-default generated API РіСЂСѓРїРї.
- `docs/plugin_sdk/src/SUMMARY.md`: Р“РµРЅРµСЂРёСЂСѓРµРјР°СЏ РЅР°РІРёРіР°С†РёСЏ Plugin SDK book: guide-РіР»Р°РІС‹ Рё generated API reference, СЃРіСЂСѓРїРїРёСЂРѕРІР°РЅРЅС‹Р№ РїРѕ СЃС‚СЂСѓРєС‚СѓСЂР°Рј, enum-Р°Рј, РєРѕРЅСЃС‚Р°РЅС‚Р°Рј, РјРµС‚РѕРґР°Рј Рё СЃРѕР±С‹С‚РёСЏРј.
- `docs/plugin_sdk/src/*.md`: Р СѓС‡РЅС‹Рµ guide-РіР»Р°РІС‹ Plugin SDK: РѕР±Р·РѕСЂ, lifecycle, СЃС‚СЂСѓРєС‚СѓСЂР° package, manifest, СЃР±РѕСЂРєР° Рё reload РґР»СЏ SDK v5.
- `docs/plugin_sdk/src/examples/{methods,events,constants}/*.md`: Р’РЅРµС€РЅРёРµ markdown-snippet РїСЂРёРјРµСЂС‹ РґР»СЏ generated Plugin SDK СЃС‚СЂР°РЅРёС†; generated reference РІСЃС‚СЂР°РёРІР°РµС‚ РёС… РєР°Рє `SDK Example`, РµСЃР»Рё С„Р°Р№Р» РґР»СЏ РєРѕРЅРєСЂРµС‚РЅРѕРіРѕ item СЃСѓС‰РµСЃС‚РІСѓРµС‚ Рё РЅРµ СЃРѕРґРµСЂР¶РёС‚ legacy v4 ABI surface, РЅРѕ РѕС‚СЃСѓС‚СЃС‚РІРёРµ snippet-Р° РЅРµ Р»РѕРјР°РµС‚ СЃР±РѕСЂРєСѓ docs.
- `docs/plugin_sdk/src/generated/*.md`: Р”РµС‚РµСЂРјРёРЅРёСЂРѕРІР°РЅРЅРѕ СЃРіРµРЅРµСЂРёСЂРѕРІР°РЅРЅС‹Рµ РёРЅРґРµРєСЃРЅС‹Рµ API-РіР»Р°РІС‹ Plugin SDK РґР»СЏ РіСЂСѓРїРї `Structures`, `Enums`, `Constants`, `Methods` Рё `Events`; РѕР±РЅРѕРІР»СЏСЋС‚СЃСЏ С‡РµСЂРµР· `cargo xtask generate-plugin-sdk-docs`.
- `docs/plugin_sdk/src/generated/{structures,enums,constants,methods,events}/*.md`: Р”РµС‚РµСЂРјРёРЅРёСЂРѕРІР°РЅРЅРѕ СЃРіРµРЅРµСЂРёСЂРѕРІР°РЅРЅС‹Рµ СЃС‚СЂР°РЅРёС†С‹ РєРѕРЅРєСЂРµС‚РЅС‹С… Plugin SDK API-СЃСѓС‰РЅРѕСЃС‚РµР№ СЃ РѕРїРёСЃР°РЅРёСЏРјРё РїРѕР»РµР№, РІР°СЂРёР°РЅС‚РѕРІ, РґРµРєР»Р°СЂР°С†РёР№, Р°СЂРіСѓРјРµРЅС‚РѕРІ, РІРѕР·РІСЂР°С‰Р°РµРјС‹С… Р·РЅР°С‡РµРЅРёР№, СЃСЃС‹Р»РєР°РјРё РЅР° СЃРІСЏР·Р°РЅРЅС‹Рµ SDK-С‚РёРїС‹, СЃРїРёСЃРєР°РјРё РјРµС‚РѕРґРѕРІ СЃС‚СЂСѓРєС‚СѓСЂ Рё РІСЃС‚СЂР°РёРІР°РµРјС‹РјРё external example-snippets.
- `docs/plugin_sdk/theme/sdk.css`: РљР°СЃС‚РѕРјРЅС‹Рµ СЃС‚РёР»Рё РёРЅС‚РµСЂР°РєС‚РёРІРЅС‹С… SDK API-Р±Р»РѕРєРѕРІ, Р±РµР№РґР¶РµР№ Рё С„РёР»СЊС‚СЂР°.
- `docs/plugin_sdk/theme/sdk.js`: РљР°СЃС‚РѕРјРЅР°СЏ РёРЅС‚РµСЂР°РєС‚РёРІРЅРѕСЃС‚СЊ Plugin SDK book: С„РёР»СЊС‚СЂ API items Рё copy-РєРЅРѕРїРєРё РґР»СЏ code blocks.
- `docs/plans/plugin_system/00_roadmap.md`: РћР±С‰РёР№ roadmap Р±СѓРґСѓС‰РµР№ РјРёРіСЂР°С†РёРё FluxEngine РЅР° runtime-РїР»Р°РіРёРЅС‹.
- `docs/plans/plugin_system/*.md`: Р”РµС‚Р°Р»СЊРЅС‹Рµ РёРЅСЃС‚СЂСѓРєС†РёРё РїРѕ СЌС‚Р°РїР°Рј СЂРµР°Р»РёР·Р°С†РёРё plugin-system РјРёРіСЂР°С†РёРё.
- `docs/project_structure.md`: РљР°СЂС‚Р° СЃС‚СЂСѓРєС‚СѓСЂС‹ РїСЂРѕРµРєС‚Р°: РґРµСЂРµРІРѕ РїР°РїРѕРє + Р·РѕРЅС‹ РѕС‚РІРµС‚СЃС‚РІРµРЅРЅРѕСЃС‚Рё С„Р°Р№Р»РѕРІ.
- `docs/technical_overview.md`: РўРµС…РЅРёС‡РµСЃРєР°СЏ Р°СЂС…РёС‚РµРєС‚СѓСЂР°, РїРѕРґСЃРёСЃС‚РµРјС‹ Рё РёРЅР¶РµРЅРµСЂРЅС‹Рµ РѕРіСЂР°РЅРёС‡РµРЅРёСЏ.
- `plugin_state.toml`: Р›РѕРєР°Р»СЊРЅС‹Р№ runtime-С„Р°Р№Р» РїРѕР»СЊР·РѕРІР°С‚РµР»СЊСЃРєРёС… РЅР°СЃС‚СЂРѕРµРє plugin enable-state; С…СЂР°РЅРёС‚СЃСЏ РІ РєРѕСЂРЅРµ РїСЂРѕРµРєС‚Р° Рё РёРіРЅРѕСЂРёСЂСѓРµС‚СЃСЏ С‡РµСЂРµР· `.gitignore`.
- `plugins/.gitkeep`: Р¤РёРєСЃРёСЂСѓРµС‚ РїСѓСЃС‚РѕР№ runtime-РєР°С‚Р°Р»РѕРі РґР»СЏ packaged plugins; СЂРµР°Р»СЊРЅС‹Рµ `.fluxplugin` РёРіРЅРѕСЂРёСЂСѓСЋС‚СЃСЏ С‡РµСЂРµР· `.gitignore`.
- `plugins_dev/.gitkeep`: Р¤РёРєСЃРёСЂСѓРµС‚ РїСѓСЃС‚РѕР№ runtime-РєР°С‚Р°Р»РѕРі expanded dev plugins; СЂРµР°Р»СЊРЅС‹Рµ РїР°РїРєРё РїР»Р°РіРёРЅРѕРІ РёРіРЅРѕСЂРёСЂСѓСЋС‚СЃСЏ С‡РµСЂРµР· `.gitignore`.
- `src/app/mod.rs`: РЎР±РѕСЂРєР° Bevy-РїСЂРёР»РѕР¶РµРЅРёСЏ, plugin bootstrap/config resource, CLI-С„Р»Р°РіРё Р·Р°РїСѓСЃРєР° РІРєР»СЋС‡Р°СЏ `--plugins-dev`, backend-РёРЅРёС†РёР°Р»РёР·Р°С†РёСЏ, Р·Р°РїСѓСЃРє Рё РїРѕРґРєР»СЋС‡РµРЅРёРµ РѕР±С‰РµРіРѕ runtime host-РїСѓС‚Рё РґР»СЏ built-in/DLL plugin dispatch.
- `src/bin/generate_pipe_scenario_saves.rs`: Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Р№ Р±РёРЅР°СЂРЅРёРє, РєРѕС‚РѕСЂС‹Р№ РїРµСЂРµСЃРѕР·РґР°С‘С‚ СЃС‚Р°СЂС‚РѕРІС‹Рµ save-slots РґР»СЏ РїСЏС‚Рё СЌС‚Р°Р»РѕРЅРЅС‹С… pipe-СЃС†РµРЅР°СЂРёРµРІ С‡РµСЂРµР· С€С‚Р°С‚РЅС‹Р№ save API.
- `src/bin/gas_perf.rs`: РџР°Р№РїР»Р°Р№РЅ РїРµСЂС„-Р±РµРЅС‡РјР°СЂРєР° РіР°Р·Р° (CPU/GPU), parity-gate Рё РѕС‚С‡С‘С‚С‹.
- `src/config/hud.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ С‚РёРїС‹ runtime-РєРѕРЅС„РёРіРѕРІ HUD, РІРєР»СЋС‡Р°СЏ substance-РєРѕРЅС‚РµР№РЅРµСЂС‹ Рё СЂРµР¶РёРјС‹ РІРёРґРёРјРѕСЃС‚Рё РїРѕ hover, Р±РµР· РІСЃС‚СЂРѕРµРЅРЅС‹С… entity-label/fallback-РєРѕРЅС„РёРіРѕРІ.
- `src/config/config_loader_block.rs`: Р’РЅСѓС‚СЂРµРЅРЅСЏСЏ Р»РѕРіРёРєР° С‡С‚РµРЅРёСЏ/РІР°Р»РёРґР°С†РёРё core/default-plugin TOML-РєРѕРЅС„РёРіРѕРІ Рё РїРѕРґРєР»СЋС‡РµРЅРёРµ gas substances РёР· Р°РєС‚РёРІРЅРѕРіРѕ `ContentRegistry`.
- `src/config/config_tests_block.rs`: РўРµСЃС‚С‹ Р·Р°РіСЂСѓР·РєРё Рё РІР°Р»РёРґР°С†РёРё РєРѕРЅС„РёРіРѕРІ.
- `src/config/mod.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ РєРѕРЅС„РёРі-С‚РёРїС‹, compatibility `GasRegistry` РїРѕРІРµСЂС… plugin-owned substance registry, runtime-СЂРµРµСЃС‚СЂС‹ base/visual/layout/HUD-РјРµС‚Р°РґР°РЅРЅС‹С… Рё РІС…РѕРґРЅР°СЏ С‚РѕС‡РєР° Р·Р°РіСЂСѓР·РєРё РєРѕРЅС„РёРіРѕРІ.
- `src/debug/mod.rs`: Debug-СЂРµР¶РёРјС‹, РѕРІРµСЂР»РµР№РЅС‹Рµ РјРµС‚СЂРёРєРё Рё РґРёР°РіРЅРѕСЃС‚РёС‡РµСЃРєРёРµ СЂРµСЃСѓСЂСЃС‹.
- `src/editor/editor_ui_block.rs`: Runtime-РѕР±СЂР°Р±РѕС‚РєР° editor UI: tooltip, state sync, РїР°РЅРµР»Рё.
- `src/editor/input_block.rs`: РњС‹С€СЊ/РєРёСЃС‚СЊ/РІС‹РґРµР»РµРЅРёРµ Рё РїСЂРёРјРµРЅРµРЅРёРµ РёРЅСЃС‚СЂСѓРјРµРЅС‚РѕРІ Рє РјРёСЂСѓ, unified pipe/structure-СЃРµС‚Рё Рё РјРѕСЃС‚Сѓ.
- `src/editor/main_menu_actions_block.rs`: РћР±СЂР°Р±РѕС‚С‡РёРєРё РґРµР№СЃС‚РІРёР№ РјРµРЅСЋ: save/load/new/exit/plugins/reload/confirm, РѕС‡РµСЂРµРґСЊ preview-capture Рё post-save follow-up СЃС†РµРЅР°СЂРёРё.
- `src/editor/main_menu_block.rs`: РљРѕРјРїРѕР·РёС†РёСЏ Р»РѕРіРёРєРё main menu (escape/actions/ui refresh).
- `src/editor/main_menu_escape_block.rs`: РћР±СЂР°Р±РѕС‚РєР° Esc Рё РїРµСЂРµС…РѕРґРѕРІ СЃРѕСЃС‚РѕСЏРЅРёР№ РјРµРЅСЋ/РёРЅСЃС‚СЂСѓРјРµРЅС‚РѕРІ, РІРєР»СЋС‡Р°СЏ РІРѕР·РІСЂР°С‚ РёР· `Plugins` Рє root screen.
- `src/editor/main_menu_plugins_block.rs`: РЎР±РѕСЂРєР° Рё in-place СЃРёРЅС…СЂРѕРЅРёР·Р°С†РёСЏ СЃРїРёСЃРєР° runtime-РїР»Р°РіРёРЅРѕРІ РґР»СЏ СЌРєСЂР°РЅР° `Plugins`, РїСЂР°РІРёР»Р° РґРѕСЃС‚СѓРїРЅРѕСЃС‚Рё toggle/reload Рё safe registry rebuild РїРѕСЃР»Рµ РёР·РјРµРЅРµРЅРёСЏ `EnabledPluginSet`.
- `src/editor/main_menu_save_list_block.rs`: РћР±С‰Р°СЏ РѕС‚РїСЂР°РІРєР° action-РёРІРµРЅС‚РѕРІ РєРЅРѕРїРѕРє РіР»Р°РІРЅРѕРіРѕ РјРµРЅСЋ, СЃР±РѕСЂРєР° РєР°СЂС‚РѕС‡РµРє save/load, Р·Р°РіСЂСѓР·РєР° preview PNG РІ UI Рё hit-test Р»РѕРіРёРєР° primary-click РїРѕ РІСЃРµР№ РєР°СЂС‚РѕС‡РєРµ.
- `src/editor/main_menu_ui_block.rs`: РћР±РЅРѕРІР»РµРЅРёРµ СЃРѕСЃС‚РѕСЏРЅРёСЏ Рё РІРёРґРёРјРѕСЃС‚Рё СЌР»РµРјРµРЅС‚РѕРІ РјРµРЅСЋ, РІРєР»СЋС‡Р°СЏ СЌРєСЂР°РЅС‹ save/load/confirm/plugins.
- `src/editor/mod.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ editor-С‚РёРїС‹/СЂРµСЃСѓСЂСЃС‹ Рё С‚РѕС‡РєР° СЃР±РѕСЂРєРё editor-СЃРёСЃС‚РµРј, РІРєР»СЋС‡Р°СЏ `Pipe/Vent/Bridge` Рё СЃРѕСЃС‚РѕСЏРЅРёРµ РїРѕРІРѕСЂРѕС‚Р° РјРѕСЃС‚Р°.
- `src/editor/overlay_setup_block.rs`: РРЅРёС†РёР°Р»РёР·Р°С†РёСЏ РІРёР·СѓР°Р»СЊРЅС‹С… editor-РѕРІРµСЂР»РµРµРІ.
- `src/editor/ui_setup_block.rs`: РЎР±РѕСЂРєР° editor-UI: РїР°РЅРµР»Рё, РєРЅРѕРїРєРё, РїРѕР»СЏ Рё РїСЂРёРІСЏР·РєР° РІРёРґР¶РµС‚РѕРІ.
- `src/editor/ui_setup_debug_panels_block.rs`: РџРѕСЃС‚СЂРѕРµРЅРёРµ debug-РїР°РЅРµР»РµР№ Рё СЃС‚СЂРѕРє РїР°СЂР°РјРµС‚СЂРѕРІ.
- `src/editor/ui_setup_menu_button_factory_block.rs`: Р¤Р°Р±СЂРёРєР° РєРЅРѕРїРѕРє РјРѕРґР°Р»СЊРЅРѕРіРѕ РјРµРЅСЋ.
- `src/editor/ui_setup_setup_fn_block.rs`: РћСЃРЅРѕРІРЅР°СЏ С„СѓРЅРєС†РёСЏ РїРµСЂРІРёС‡РЅРѕР№ СЃР±РѕСЂРєРё editor-UI, РІРєР»СЋС‡Р°СЏ РєРЅРѕРїРєСѓ `Gases`, РїРѕРґРїaРЅРµР»СЊ РІС‹Р±РѕСЂР° `Pipe/Vent/Bridge` Рё РєРѕРЅС‚РµР№РЅРµСЂС‹ СЌРєСЂР°РЅРѕРІ РіР»Р°РІРЅРѕРіРѕ РјРµРЅСЋ.
- `src/editor/ui_setup_structure_buttons_block.rs`: Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Рµ С„Р°Р±СЂРёРєРё РєРЅРѕРїРѕРє РёРЅСЃС‚СЂСѓРјРµРЅС‚РѕРІ/РјР°С‚РµСЂРёР°Р»РѕРІ.
- `src/input/camera.rs`: РЈРїСЂР°РІР»РµРЅРёРµ РєР°РјРµСЂРѕР№, Р·СѓРј/РїР°РЅ Рё С‚РµСЃС‚С‹ РєРѕСЂСЂРµРєС‚РЅРѕСЃС‚Рё СЏРєРѕСЂСЏ.
- `src/input/mod.rs`: РџР»Р°РіРёРЅ РїРѕРґСЃРёСЃС‚РµРјС‹ РІРІРѕРґР° Рё wiring СЃРёСЃС‚РµРј РІРІРѕРґР°.
- `src/lib.rs`: РљРѕСЂРЅРµРІРѕР№ РјРѕРґСѓР»СЊ Р±РёР±Р»РёРѕС‚РµРєРё Рё СЌРєСЃРїРѕСЂС‚ РїРѕРґСЃРёСЃС‚РµРј, РІРєР»СЋС‡Р°СЏ РЅРѕРІС‹Р№ `plugins`.
- `src/main.rs`: РўРѕС‡РєР° РІС…РѕРґР° Р±РёРЅР°СЂСЏ; Р·Р°РїСѓСЃРєР°РµС‚ РїСЂРёР»РѕР¶РµРЅРёРµ.
- `src/plugins/abi.rs`: Engine-side wrapper РЅР°Рґ `crates/flux_plugin_abi`: СЃР±РѕСЂРєР° host/registrar payload РґР»СЏ loader/runtime Рё mapping ABI event kinds РІ РІРЅСѓС‚СЂРµРЅРЅРёРµ engine events.
- `src/plugins/api/mod.rs`: Engine-side shared plugin API module root Рё re-exports РґР»СЏ СЃРѕР±С‹С‚РёР№, runtime registry, render/UI/save contracts, РєРѕС‚РѕСЂС‹Рµ РёСЃРїРѕР»СЊР·СѓРµС‚ С…РѕСЃС‚ plugin-СЃРёСЃС‚РµРјС‹.
- `src/plugins/api/events.rs`: Plugin event kinds and payloads, including simulation lifecycle, save lifecycle, low-level mouse cell input and keyboard events.
- `src/plugins/api/render_api.rs`: Overlay render contract with `OverlayRenderPolicy`, `OverlayFrame`, per-cell/per-structure/per-gas styles and draw commands.
- `src/plugins/api/ui_api.rs`: Declarative plugin UI descriptors for tools, panels, HUD blocks and simple UI node trees.
- `src/plugins/api/save_api.rs`: In-memory plugin save chunk store and chunk payload contracts.
- `src/plugins/api/runtime.rs`: Runtime registry for plugin event subscriber groups, tool descriptors, overlay descriptors and save chunk descriptors.
- `src/plugins/content.rs`: Content registry runtime-РјРѕРґРµР»СЊ: stable `ContentId`, provider plugins, descriptors РєР»РµС‚РѕРє/СЃС‚СЂСѓРєС‚СѓСЂ/overlay, HUD metadata Рё registered substances.
- `src/plugins/default_plugin/mod.rs`: Built-in locked `flux.default` content/runtime: default descriptor registration, generic ID facade, legacy numeric save adapters Рё wiring built-in SDK runtime + pipe-runtime support.
- `src/plugins/default_plugin/runtime_sdk.rs`: In-process SDK plugin type РґР»СЏ `flux.default`: С…СЂР°РЅРёС‚ typed proxy API, РїРѕРґРїРёСЃС‹РІР°РµС‚СЃСЏ РЅР° lifecycle/simulation/HUD СЃРѕР±С‹С‚РёСЏ Рё РІС‹РїРѕР»РЅСЏРµС‚ built-in runtime-Р»РѕРіРёРєСѓ С‡РµСЂРµР· РѕР±С‰РёР№ plugin dispatch.
- `src/plugins/default_plugin/descriptors_block.rs`: Р’РЅСѓС‚СЂРµРЅРЅРёР№ Р±Р»РѕРє СЃР±РѕСЂРєРё descriptors default plugin-Р°: layer/collision rules, footprint, rotations, sprite metadata Рё HUD blocks.
- `src/plugins/default_plugin/ids.rs`: Stable IDs `flux.default` РґР»СЏ cells/structures/plugin overlays/substances, typed wrapper helpers Рё asset/config root helpers default plugin-Р°.
- `src/plugins/default_plugin/tests.rs`: Unit-С‚РµСЃС‚С‹ С„Р°СЃР°РґР° default plugin-Р°: legacy ID roundtrip, РїРѕР»РЅРѕС‚Р° registry Рё РїРѕСЂСЏРґРѕРє HUD-Р±Р»РѕРєРѕРІ.
- `src/plugins/default_plugin/assets/ui/tool_*.png`: Content-specific UI-РёРєРѕРЅРєРё default plugin-Р° РґР»СЏ РјР°С‚РµСЂРёР°Р»РѕРІ Рё СЃС‚СЂСѓРєС‚СѓСЂ.
- `src/plugins/default_plugin/assets/shaders/pipe_highlight_material.wgsl`: Plugin-owned WGSL-С€РµР№РґРµСЂ `Material2d` РґР»СЏ СЏСЂРєРѕР№ РїРѕРґСЃРІРµС‚РєРё С‚СЂСѓР± РІ `F3/Pipes`.
- `src/plugins/default_plugin/assets/world/pipe_mask_*.png`: Р¤Р°Р№Р»РѕРІС‹Рµ СЃРїСЂР°Р№С‚С‹ С‚СЂСѓР± РґР»СЏ РІСЃРµС… connection-mask РІР°СЂРёР°РЅС‚РѕРІ, Р·Р°РіСЂСѓР¶Р°РµРјС‹Рµ С‡РµСЂРµР· `flux_default://world/...`.
- `src/plugins/default_plugin/assets/world/pipe_silhouette_mask_*.png`: Р¤Р°Р№Р»РѕРІС‹Рµ silhouette-СЃРїСЂР°Р№С‚С‹ С‚СЂСѓР± РґР»СЏ ghost-preview.
- `src/plugins/default_plugin/assets/world/bridge*.png`, `gas_*.png`, `silhouette_*.png`, `tile_*.png`: World-СЃРїСЂР°Р№С‚С‹ default plugin-Р° РґР»СЏ СЃС‚РµРЅ, СЃС‚СЂСѓРєС‚СѓСЂ, РјРѕСЃС‚РѕРІ Рё pipe overlay.
- `src/plugins/default_plugin/config/cell_types.toml`: РќР°СЃС‚СЂРѕР№РєРё РІРёР·СѓР°Р»Р°/РїР°СЂР°РјРµС‚СЂРѕРІ default-РєР»РµС‚РѕРє Рё HUD-РєРѕРЅС„РёРі world-РєР»РµС‚РєРё РґР»СЏ СЃРІРѕР±РѕРґРЅРѕРіРѕ РіР°Р·Р°.
- `src/plugins/default_plugin/config/gases/*.toml`: Optional data-РєРѕРЅС„РёРіРё default plugin gas substances; РїСЂРё РїСѓСЃС‚РѕР№ РїР°РїРєРµ Р±Р°Р·РѕРІС‹Рµ `H2/O2/CO2` Р±РµСЂСѓС‚СЃСЏ РёР· built-in default plugin definitions.
- `src/plugins/default_plugin/config/pipe_runtime.toml`: Runtime-РЅР°СЃС‚СЂРѕР№РєРё pipe pressure/flux/vent solver default plugin-Р°.
- `src/plugins/default_plugin/config/structures/*.toml`: РљРѕРЅС„РёРіРё appearance Рё HUD-РјРµС‚Р°РґР°РЅРЅС‹С… РІСЃС‚СЂРѕРµРЅРЅС‹С… СЃС‚РµРЅ Рё СЃС‚СЂСѓРєС‚СѓСЂ (`label`, `draw_priority`, `size_in_cells`, `hud.sort_order` Рё substance-РєРѕРЅС‚РµР№РЅРµСЂС‹).
- `src/plugins/flux_stage1_sample_plugin/Cargo.toml`: РћС‚РґРµР»СЊРЅС‹Р№ `cdylib` crate РјРёРЅРёРјР°Р»СЊРЅРѕРіРѕ non-content sample plugin-Р° РЅР° `flux_plugin_sdk`.
- `src/plugins/flux_stage1_sample_plugin/package_template/manifest.toml`: РЁР°Р±Р»РѕРЅ packaged plugin manifest РґР»СЏ sample DLL, РёСЃРїРѕР»СЊР·СѓРµРјС‹Р№ РїРѕР·РёС‚РёРІРЅС‹Рј e2e-С‚РµСЃС‚РѕРј.
- `src/plugins/flux_stage1_sample_plugin/package_template/config/sample.toml`: РњРёРЅРёРјР°Р»СЊРЅС‹Р№ config-С„Р°Р№Р» sample plugin package.
- `src/plugins/flux_stage1_sample_plugin/package_template/assets/placeholder.txt`: РњРёРЅРёРјР°Р»СЊРЅС‹Р№ asset-С„Р°Р№Р» sample plugin package.
- `src/plugins/flux_stage1_sample_plugin/src/lib.rs`: РњРёРЅРёРјР°Р»СЊРЅС‹Р№ sample runtime-РїР»Р°РіРёРЅ РЅР° `Plugin` + `declare_plugin!`, РёСЃРїРѕР»СЊР·СѓРµРјС‹Р№ smoke/e2e workflow-РѕРј СЃР±РѕСЂРєРё.
- `src/plugins/flux_stage7_sample_content_plugin/Cargo.toml`: РћС‚РґРµР»СЊРЅС‹Р№ `cdylib` crate sample content plugin-Р° stage-7.
- `src/plugins/flux_stage7_sample_content_plugin/package_template/manifest.toml`: РЁР°Р±Р»РѕРЅ packaged plugin manifest РґР»СЏ sample content plugin-Р° СЃ `content = true`.
- `src/plugins/flux_stage7_sample_content_plugin/package_template/config/sample.toml`: РњРёРЅРёРјР°Р»СЊРЅС‹Р№ config-С„Р°Р№Р» sample content plugin package.
- `src/plugins/flux_stage7_sample_content_plugin/package_template/assets/placeholder.txt`: РњРёРЅРёРјР°Р»СЊРЅС‹Р№ asset-С„Р°Р№Р» sample content plugin package.
- `src/plugins/flux_stage7_sample_content_plugin/src/lib.rs`: Sample content plugin РЅР° `flux_plugin_sdk`, СЂРµРіРёСЃС‚СЂРёСЂСѓСЋС‰РёР№ РІРЅРµС€РЅРёР№ РіР°Р· `flux.sample_content.substance.neon`.
- `src/plugins/diagnostics.rs`: Startup scan packaged archives, РґРµРґСѓРїР»РёРєР°С†РёСЏ `PluginId`, resource СЃ СЂРµР·СѓР»СЊС‚Р°С‚Р°РјРё РїСЂРѕРІРµСЂРєРё Рё С‚РµРєСЃС‚ РґР»СЏ СЃС‚Р°С‚СѓСЃР° РіР»Р°РІРЅРѕРіРѕ РјРµРЅСЋ.
- `src/plugins/id.rs`: РўРёРїРёР·РёСЂРѕРІР°РЅРЅС‹Рµ `PluginId`, `PluginVersion`, `PluginApiVersion` Рё РїСЂРѕРІРµСЂРєР° РєР°РЅРѕРЅРёС‡РµСЃРєРѕРіРѕ С„РѕСЂРјР°С‚Р° ID.
- `src/plugins/loader.rs`: Р§С‚РµРЅРёРµ packaged/dev plugin-РєР°РЅРґРёРґР°С‚РѕРІ, cache-РєРѕРїРёРё runtime-root, Р·Р°РіСЂСѓР·РєР° DLL, ABI handshake `create/register/dispatch/destroy`, fingerprint source-Р° Рё СЃР±РѕСЂ runtime registration/subscription-РјРѕРґРµР»Рё.
- `src/plugins/manifest.rs`: РџР°СЂСЃРёРЅРі Рё РІР°Р»РёРґР°С†РёСЏ `manifest.toml` РІ runtime-СЃС‚СЂСѓРєС‚СѓСЂСѓ `PluginManifest`.
- `src/plugins/mod.rs`: РўРѕС‡РєР° СЃР±РѕСЂРєРё plugin-РїРѕРґСЃРёСЃС‚РµРјС‹ Рё РµС‘ РїСѓР±Р»РёС‡РЅС‹Р№ re-export API; РїРѕРґРєР»СЋС‡Р°РµС‚ РѕР±С‰РёР№ runtime layer РґР»СЏ DLL Рё built-in endpoint-РѕРІ.
- `src/plugins/reload.rs`: РђС‚РѕРјР°СЂРЅС‹Р№ manual reload/rescan runtime-РїР»Р°РіРёРЅРѕРІ Р±РµР· Р·Р°РіСЂСѓР¶РµРЅРЅРѕРіРѕ РјРёСЂР°: rebuild registry, РїРµСЂРµСЃР±РѕСЂРєР° gas registry, generation/report Рё СЃСЂР°РІРЅРµРЅРёРµ source fingerprints.
- `src/plugins/registration.rs`: Runtime-СЃС‚СЂСѓРєС‚СѓСЂР° СЂРµР·СѓР»СЊС‚Р°С‚Р° ABI-СЂРµРіРёСЃС‚СЂР°С†РёРё plugin capabilities/content, РІРєР»СЋС‡Р°СЏ СЃСѓС‰РЅРѕСЃС‚Рё, РіР°Р·С‹, РёРЅСЃС‚СЂСѓРјРµРЅС‚С‹, overlay, save chunks Рё event subscriptions Р±РµР· `handler_name`.
- `src/plugins/runtime_builtin.rs`: Built-in runtime endpoint-С‹ РЅР° Р±Р°Р·Рµ `flux_plugin_sdk::BuiltinPluginRuntime`, conversion engine events -> typed SDK events Рё СЃР±РѕСЂРєР° runtime registration РґР»СЏ `flux.default`.
- `src/plugins/runtime_dll.rs`: РћР±С‰РёР№ runtime executor РІРµСЂС…РЅРµРіРѕ СѓСЂРѕРІРЅСЏ: С…СЂР°РЅРёС‚ unified plugin endpoint registry, DLL host callbacks РґР»СЏ entity/gas/UI/overlay/save/time APIs Рё dispatch РєР°Рє РІ live DLL-РїР»Р°РіРёРЅС‹, С‚Р°Рє Рё РІ built-in `flux.default`.
- `src/plugins/runtime_dll_events.rs`: Typed runtime event dispatch РґР»СЏ SDK v5: РєРѕРґРёСЂСѓРµС‚ ABI payload Рё РІС‹Р·С‹РІР°РµС‚ РµРґРёРЅС‹Р№ `flux_plugin_dispatch` Сѓ РєР°Р¶РґРѕРіРѕ РїРѕРґРїРёСЃР°РЅРЅРѕРіРѕ DLL-РїР»Р°РіРёРЅР°.
- `src/plugins/runtime_host_binding.rs`: Engine-side Р°РґР°РїС‚РµСЂ `RuntimeHostContext -> flux_plugin_sdk::__private::RuntimeHostBinding`, РєРѕС‚РѕСЂС‹Р№ РґР°С‘С‚ built-in SDK runtime С‚РѕС‚ Р¶Рµ host-contract, С‡С‚Рѕ Рё ABI-РїР»Р°РіРёРЅС‹.
- `src/plugins/flux_api_cell_demo_plugin/`: Runtime DLL fixture РЅР° `flux_plugin_sdk`, РґРµРјРѕРЅСЃС‚СЂРёСЂСѓСЋС‰РёР№ entity/tool/input path РЅРѕРІРѕРіРѕ SDK.
- `src/plugins/flux_api_tick_demo_plugin/`: Runtime DLL fixture РЅР° `flux_plugin_sdk`, РґРµРјРѕРЅСЃС‚СЂРёСЂСѓСЋС‰РёР№ simulation pre-step handler Р±РµР· named ABI exports.
- `src/plugins/flux_api_temperature_overlay_plugin/`: Runtime DLL fixture РЅР° `flux_plugin_sdk`, СЂРµРіРёСЃС‚СЂРёСЂСѓСЋС‰РёР№ plugin-controlled overlay Рё РїСЂРёСЃС‹Р»Р°СЋС‰РёР№ РіРѕС‚РѕРІС‹Р№ frame С‡РµСЂРµР· unified dispatch.
- `src/plugins/flux_api_ui_save_demo_plugin/`: Runtime DLL fixture РЅР° `flux_plugin_sdk`, РґРµРјРѕРЅСЃС‚СЂРёСЂСѓСЋС‰РёР№ HUD, save chunk Рё input-driven runtime path.
- `src/plugins/registry.rs`: Bootstrap runtime registry/state, default plugin source priority, `LoadedPluginRegistry` Рё rebuild-helper РґР»СЏ menu toggle; content registry СЃРѕР·РґР°С‘С‚СЃСЏ РёР· default descriptors РїР»СЋСЃ runtime registration РІРєР»СЋС‡С‘РЅРЅС‹С… content-РїР»Р°РіРёРЅРѕРІ.
- `src/plugins/source.rs`: Discovery packaged/dev plugin sources, structured rejected-source diagnostics, source fingerprint Рё resolve plugin layout РІРЅСѓС‚СЂРё plugin root.
- `src/plugins/state.rs`: `EnabledPluginSet`, `plugin_state.toml`, runtime plugin statuses Рё aggregate `PluginRegistryState`.
- `src/plugins/substances.rs`: Generic plugin-owned substance contract: `SubstanceId`, `SubstanceDefinition`, `SubstanceFlags` Рё deterministic `SubstanceRegistry` РґР»СЏ compact runtime indices.
- `src/render/mod.rs`: РџР»Р°РіРёРЅ СЂРµРЅРґРµСЂР° Рё РїРѕСЂСЏРґРѕРє render-СЃРёСЃС‚РµРј, РІРєР»СЋС‡Р°СЏ pipe visuals.
- `src/render/pipe_highlight_material.rs`: РљР°СЃС‚РѕРјРЅС‹Р№ `Material2d` Рё helper-Р»РѕРіРёРєР° РґР»СЏ shader-РїРѕРґСЃРІРµС‚РєРё С‚СЂСѓР± РІ `F3`.
- `src/render/save_preview.rs`: Offscreen preview pipeline РґР»СЏ save-slots: РѕС‚РґРµР»СЊРЅР°СЏ РєР°РјРµСЂР°, settle-frame РІ РєР°РЅРѕРЅРёС‡РµСЃРєРѕРј `F1`, screenshot capture, PNG-Р·Р°РїРёСЃСЊ Рё РІРѕСЃСЃС‚Р°РЅРѕРІР»РµРЅРёРµ UI/overlay СЃРѕСЃС‚РѕСЏРЅРёСЏ РїРѕСЃР»Рµ РєР°РґСЂР°.
- `src/render/world_view.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ render-СЃРёСЃС‚РµРјС‹ world view, config-driven appearance z-order Рё layer-based pipe/bridge visuals.
- `src/render/world_view_cursor_highlight_block.rs`: Helper РѕС‚СЂРёСЃРѕРІРєРё РІРЅСѓС‚СЂРµРЅРЅРµР№ Р±РµР»РѕР№ РїСѓРЅРєС‚РёСЂРЅРѕР№ СЂР°РјРєРё РІРЅСѓС‚СЂРё РЅР°РІРµРґС‘РЅРЅРѕР№ РєР»РµС‚РєРё.
- `src/render/world_view_overlay_block.rs`: Р›РѕРіРёРєР° core overlay `F1/F2` Рё plugin overlay `F3/Pipes`, РєСѓСЂСЃРѕСЂРЅРѕР№ СЃРµС‚РєРё, multi-container pipe gas-square sizing, flow-packet Р°РЅРёРјР°С†РёРё Рё С„РёР»СЊС‚СЂР°С†РёРё РІРёР·СѓР°Р»СЊРЅРѕРіРѕ С€СѓРјР° РґР»СЏ РїР°РєРµС‚РѕРІ `< 5` С‡Р°СЃС‚РёС†.
- `src/render/world_view_setup_block.rs`: РџРѕСЃС‚СЂРѕРµРЅРёРµ СЃСѓС‰РЅРѕСЃС‚РµР№ РјРёСЂР°/СЃР»РѕС‘РІ, config-driven z-order СЃС‚РµРЅ/СЃС‚СЂСѓРєС‚СѓСЂ Рё СЃРїР°РІРЅ РІРёР·СѓР°Р»РѕРІ РёР· `PlacedStructureMap`.
- `src/render/world_view_tests_block.rs`: РўРµСЃС‚С‹ РІСЃРїРѕРјРѕРіР°С‚РµР»СЊРЅРѕР№ РјР°С‚РµРјР°С‚РёРєРё СЂРµРЅРґРµСЂР°.
- `src/save.rs`: РџСѓР±Р»РёС‡РЅС‹Р№ save/load API, С‚РёРїС‹ СЃРѕСЃС‚РѕСЏРЅРёСЏ РјРµРЅСЋ/СЃРµСЃСЃРёРё, plugin menu screen state Рё queue/event РєРѕРЅС‚СЂР°РєС‚С‹ preview-capture.
- `src/save_api_block.rs`: РћРїРµСЂР°С†РёРё РІРµСЂС…РЅРµРіРѕ СѓСЂРѕРІРЅСЏ: list/create/overwrite/load snapshot Рё canonical preview-path РґР»СЏ slot-Р°.
- `src/save_content_gate_block.rs`: РЎР±РѕСЂ required plugin content IDs РґР»СЏ save-meta Рё load-gate РїСЂРѕРІРµСЂРєР° РґРѕСЃС‚СѓРїРЅРѕСЃС‚Рё content РїРµСЂРµРґ С‡С‚РµРЅРёРµРј world chunks.
- `src/save_content_gate_tests_block.rs`: РўРµСЃС‚С‹ required-content meta Рё load-gate СЃС†РµРЅР°СЂРёРµРІ plugin-compatible save schema.
- `src/save_format_tests_block.rs`: РўРµСЃС‚С‹ РѕС‚РєР°Р·Р° СЃС‚Р°СЂС‹С…/Р±РёС‚С‹С… save schema Рё mapping edge cases РґР»СЏ gas chunks.
- `src/save_gas_io_block.rs`: Р§С‚РµРЅРёРµ/Р·Р°РїРёСЃСЊ chunk-РѕРІ РјРёСЂР°, РіР°Р·Р°, unified placed-structures Рё node-based pipe-gas С„РѕСЂРјР°С‚Р° save schema `6`; world/structure/pipe chunks С…СЂР°РЅСЏС‚ stable content IDs, gas chunks РјР°РїСЏС‚СЃСЏ РјРµР¶РґСѓ saved stable substance IDs/legacy aliases Рё С‚РµРєСѓС‰РёРјРё compact indices.
- `src/save_meta_io_block.rs`: РњРµС‚Р°РґР°РЅРЅС‹Рµ СЃРµР№РІР°, required content, РґРёР°РіРЅРѕСЃС‚РёС‡РµСЃРєРёР№ СЃРїРёСЃРѕРє enabled plugins, РІР°Р»РёРґР°С†РёСЏ РµРґРёРЅСЃС‚РІРµРЅРЅРѕР№ РїРѕРґРґРµСЂР¶РёРІР°РµРјРѕР№ save-СЃС…РµРјС‹ Рё preview-chunk `png_v1`.
- `src/save_pipe_gas_io_block.rs`: Р§С‚РµРЅРёРµ/Р·Р°РїРёСЃСЊ node-based pipe-gas chunk v2 СЃРѕ stable pipe-container content IDs Рё mapping saved substance IDs РІ С‚РµРєСѓС‰РёР№ compact registry.
- `src/save_tests_block.rs`: РћСЃРЅРѕРІРЅС‹Рµ С‚РµСЃС‚С‹ СЃРѕС…СЂР°РЅРµРЅРёСЏ/Р·Р°РіСЂСѓР·РєРё, roundtrip, preview meta Рё shared helpers РґР»СЏ save test blocks.
- `src/simulation/backend.rs`: РљРѕРЅС„РёРі backend Рё РїР°СЂР°РјРµС‚СЂС‹ СЂР°Р·РјРµСЂР° РјРёСЂР° РґР»СЏ СЃРёРјСѓР»СЏС†РёРё.
- `src/simulation/discrete_step.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ РєРѕРЅС‚СЂР°РєС‚С‹ РґРёСЃРєСЂРµС‚РЅРѕРіРѕ CPU-С€Р°РіР° РіР°Р·Р°.
- `src/simulation/discrete_step_helpers_block.rs`: Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Рµ С„СѓРЅРєС†РёРё РґРёСЃРєСЂРµС‚РЅРѕРіРѕ С€Р°РіР° (kernel/RNG/СѓС‚РёР»РёС‚С‹).
- `src/simulation/discrete_step_step_block.rs`: РћСЃРЅРѕРІРЅРѕР№ Р°Р»РіРѕСЂРёС‚Рј РґРёСЃРєСЂРµС‚РЅРѕРіРѕ С€Р°РіР° CPU СЃРёРјСѓР»СЏС†РёРё.
- `src/simulation/gas.rs`: РџСѓР±Р»РёС‡РЅР°СЏ РјРѕРґРµР»СЊ GasField Рё СЃРІСЏР·РєР° CPU/GPU СЃРѕСЃС‚РѕСЏРЅРёСЏ.
- `src/simulation/gas_core_block.rs`: РћСЃРЅРѕРІРЅР°СЏ Р»РѕРіРёРєР° РѕРїРµСЂР°С†РёР№ GasField РІ runtime.
- `src/simulation/gas_test_support_block.rs`: Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Рµ test-only С„СѓРЅРєС†РёРё РґР»СЏ buoyancy/reachability.
- `src/simulation/gas_tests_block.rs`: РќР°Р±РѕСЂ С‚РµСЃС‚РѕРІ GasField/РїРѕРІРµРґРµРЅРёСЏ СЃРёРјСѓР»СЏС†РёРё Рё СЂРµРіСЂРµСЃСЃРёР№.
- `src/simulation/gpu_solver.rs`: РџСѓР±Р»РёС‡РЅС‹Р№ РёРЅС‚РµСЂС„РµР№СЃ GPU solver Рё РёРЅРёС†РёР°Р»РёР·Р°С†РёСЏ СЂРµСЃСѓСЂСЃРѕРІ wgpu.
- `src/simulation/gpu_solver_helpers_block.rs`: Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Рµ С„СѓРЅРєС†РёРё Р±СѓС„РµСЂРѕРІ, bind-РіСЂСѓРїРї Рё dispatch.
- `src/simulation/gpu_solver_impl_core_block.rs`: Core-РёРЅРёС†РёР°Р»РёР·Р°С†РёСЏ/Р·Р°РіСЂСѓР·РєР° СЃРѕСЃС‚РѕСЏРЅРёСЏ GPU solver.
- `src/simulation/gpu_solver_impl_exec_block.rs`: РСЃРїРѕР»РЅРµРЅРёРµ С€Р°РіР° GPU, readback Рё РіРµРЅРµСЂР°С†РёСЏ РїР°СЂР°РјРµС‚СЂРѕРІ.
- `src/simulation/mod.rs`: РџР»Р°РіРёРЅ core-СЃРёРјСѓР»СЏС†РёРё СЃРІРѕР±РѕРґРЅРѕРіРѕ РіР°Р·Р°, СЂРµСЃСѓСЂСЃС‹ СЃРѕСЃС‚РѕСЏРЅРёСЏ, schedule sets, CPU/GPU backend orchestration, reset GPU solver state Рё РѕР±С‰РёРµ perf-РјРµС‚СЂРёРєРё.
- `src/simulation/parity.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ parity API Рё СЃС†РµРЅР°СЂРёРё СЃСЂР°РІРЅРµРЅРёСЏ CPU/GPU.
- `src/simulation/parity_runtime_block.rs`: Runtime parity-РјРµС‚СЂРёРєРё, РїСЂРѕРіРѕРЅС‹ СЃС†РµРЅР°СЂРёРµРІ Рё gate-РѕС†РµРЅРєР°.
- `src/simulation/parity_tests_block.rs`: РўРµСЃС‚С‹ parity-РїРѕСЂРѕРіРѕРІ, smoke Рё GPU-СЂРµРіСЂРµСЃСЃРёР№.
- `src/plugins/default_plugin/pipe_runtime.rs`: Node-based `PipeGasField`, runtime-only `PipeFluxField`, РїСѓР±Р»РёС‡РЅС‹Р№ С„Р°СЃР°Рґ pipe runtime/visual API, support-plugin Рё wiring С‚РµСЃС‚РѕРІ pipe-СЃРµС‚Рё; СЃР°Рј pre-step dispatch С‚РµРїРµСЂСЊ РёРґС‘С‚ С‡РµСЂРµР· built-in SDK runtime.
- `src/plugins/default_plugin/pipe_runtime/pressure.rs`: Helper-С‹ РїРµСЂРµРІРѕРґР° `particles -> pressure` Рё С„РѕСЂРјР°С‚РёСЂРѕРІР°РЅРёСЏ РґР°РІР»РµРЅРёСЏ РґР»СЏ HUD/pipe-СЂРµРЅРґРµСЂР°.
- `src/plugins/default_plugin/pipe_runtime/scenarios.rs`: РћР±С‰РёР№ builder РїСЏС‚Рё РєР°РЅРѕРЅРёС‡РµСЃРєРёС… pipe-СЃС†РµРЅР°СЂРёРµРІ РґР»СЏ save-СѓС‚РёР»РёС‚С‹ Рё acceptance-С‚РµСЃС‚РѕРІ.
- `src/plugins/default_plugin/pipe_runtime/solver.rs`: Р’РЅСѓС‚СЂРµРЅРЅРёР№ semi-implicit pressure+flux solver pipe-СЃРµС‚Рё: component solve, world-vent budgets, mass-bounded transfers Рё Р·Р°РїРёСЃСЊ `PipeFlowVisualState`.
- `src/plugins/default_plugin/pipe_runtime/tests.rs`: Acceptance/regression С‚РµСЃС‚С‹ РЅРѕРІРѕР№ pipe-РјРѕРґРµР»Рё, РІРєР»СЋС‡Р°СЏ Р±С‹СЃС‚СЂС‹Рµ `_smoke` РїСЂРѕРІРµСЂРєРё РґР»СЏ СЃР°РјС‹С… РґРѕР»РіРёС… СЃС†РµРЅР°СЂРёРµРІ Рё РїРѕР»РЅС‹Рµ РєР°РЅРѕРЅРёС‡РµСЃРєРёРµ scenario 1..5.
- `src/simulation/runtime_tick_block.rs`: Runtime-С€Р°РіРё core-СЃРёРјСѓР»СЏС†РёРё СЃРІРѕР±РѕРґРЅРѕРіРѕ РіР°Р·Р°, GPU/CPU РїРѕРґС€Р°РіРё Рё perf-РјРµС‚СЂРёРєРё; pipe pre-step РІС‹РїРѕР»РЅСЏРµС‚СЃСЏ default plugin runtime-РѕРј РґРѕ СЌС‚РѕРіРѕ С€Р°РіР°.
- `src/simulation/simulation_tests_block.rs`: РўРµСЃС‚С‹ РєРѕРЅС„РёРіСѓСЂР°С†РёРё С‚РёРєР° Рё СЃС‚СЂСѓРєС‚СѓСЂРЅС‹С… pre-step РїСЂР°РІРёР».
- `src/ui/cell_inspector.rs`: Runtime-СЃР±РѕСЂРєР° Рё РїРѕР·РёС†РёРѕРЅРёСЂРѕРІР°РЅРёРµ HUD РёРЅСЃРїРµРєС‚РѕСЂР° РєР»РµС‚РєРё РєР°Рє СЃС‚РµРєР° РѕС‚РґРµР»СЊРЅС‹С… entity-Р±Р»РѕРєРѕРІ СЃ РѕР±С‰РµР№ С‚РµРЅСЊСЋ; plugin HUD С‚РµРїРµСЂСЊ СЃРѕР±РёСЂР°РµС‚СЃСЏ С‡РµСЂРµР· РѕР±С‰РёР№ `BuildHudForCell` dispatch, РІРєР»СЋС‡Р°СЏ built-in `flux.default`.
- `src/ui/cell_inspector_model.rs`: РњРѕРґРµР»СЊ РґР°РЅРЅС‹С… Рё formatter HUD РёРЅСЃРїРµРєС‚РѕСЂР° РєР»РµС‚РєРё, РІРєР»СЋС‡Р°СЏ config-driven РєРѕРЅС‚РµР№РЅРµСЂС‹, solid-РјР°С‚РµСЂРёР°Р»С‹ РєР°Рє РѕС‚РґРµР»СЊРЅС‹Рµ Р±Р»РѕРєРё Рё registry-driven РѕС‚РѕР±СЂР°Р¶РµРЅРёРµ СЃРѕСЃС‚Р°РІР° РіР°Р·Р°.
- `src/ui/input_field.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ С‚РёРїС‹ text-input Рё С‚РѕС‡РєР° СЃР±РѕСЂРєРё input-СЃРёСЃС‚РµРј.
- `src/ui/input_field_helpers_block.rs`: Р’СЃРїРѕРјРѕРіР°С‚РµР»СЊРЅР°СЏ РіРµРѕРјРµС‚СЂРёСЏ РєСѓСЂСЃРѕСЂР° С‚РµРєСЃС‚Р° Рё С‚РѕС‡РЅС‹Р№ hit-test/РєР°СЂРµС‚РєР° С‡РµСЂРµР· `ComputedTextBlock`.
- `src/ui/input_field_systems_block.rs`: РЎРёСЃС‚РµРјС‹ focus/keyboard/render/caret РґР»СЏ С‚РµРєСЃС‚РѕРІС‹С… РїРѕР»РµР№.
- `src/ui/modal.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ С‚РёРїС‹ reusable modal backdrop subsystem, helper-С‹ СЃРїР°РІРЅР° backdrop-СЃР»РѕС‘РІ Рё wiring `ModalPlugin`.
- `src/ui/modal_capture_block.rs`: Snapshot/capture runtime РґР»СЏ modal backdrop-РѕРІ: offscreen-РєР°РјРµСЂР°, resize target-Р°, blur world-snapshot Рё cache lifecycle.
- `src/ui/modal_runtime_block.rs`: Р’С‹Р±РѕСЂ topmost РјРѕРґР°Р»РєРё, cover-layout backdrop-РёР·РѕР±СЂР°Р¶РµРЅРёР№ Рё РїРµСЂРµРєР»СЋС‡РµРЅРёРµ СЂРµР¶РёРјРѕРІ `PanelFrosted` / `FullscreenBlur`.
- `src/ui/modal_tests_block.rs`: Unit-С‚РµСЃС‚С‹ modal helper-РѕРІ, cover-layout Рё РїСЂР°РІРёР» refresh/capture РґР»СЏ world-snapshot backdrop.
- `src/ui/mod.rs`: UI-РїР»Р°РіРёРЅ, wiring РѕР±С‰РёС… UI-СЃРёСЃС‚РµРј Рё exports РїРµСЂРµРёСЃРїРѕР»СЊР·СѓРµРјС‹С… UI-РєРѕРјРїРѕРЅРµРЅС‚РѕРІ.
- `src/ui/palette.rs`: Р•РґРёРЅР°СЏ РїР°Р»РёС‚СЂР° С†РІРµС‚РѕРІ UI (РїР°РЅРµР»Рё, РјРµРЅСЋ, С‚РµРєСЃС‚, input/select, tooltip, HUD Рё С‚РµРЅРё HUD).
- `src/ui/panels.rs`: РџСѓР±Р»РёС‡РЅС‹Рµ С‚РёРїС‹ panel-СЃРёСЃС‚РµРјС‹ Рё РєРѕРјРїРѕР·РёС†РёСЏ Р±Р»РѕРєРѕРІ РїР°РЅРµР»Рё.
- `src/ui/panels_manager_block.rs`: РЎРѕСЃС‚РѕСЏРЅРёРµ Рё API PanelManager, hit-rect Рё СѓРїСЂР°РІР»РµРЅРёРµ РїР°РЅРµР»СЏРјРё.
- `src/ui/panels_runtime_block.rs`: Runtime-СЃРёСЃС‚РµРјС‹ РїР°РЅРµР»Рё: layout, СЃРѕСЃС‚РѕСЏРЅРёРµ viewport-РѕРІ Рё СЃРѕР±С‹С‚РёСЏ Р·Р°РіРѕР»РѕРІРєР°; input scroll РґРµР»РµРіРёСЂРѕРІР°РЅ РѕР±С‰РµРјСѓ `scroll_area`.
- `src/ui/panels_tests_block.rs`: РўРµСЃС‚С‹ layout/scroll/stack-РїРѕРІРµРґРµРЅРёСЏ РїР°РЅРµР»РµР№.
- `src/ui/scroll_area.rs`: РћР±С‰РёР№ scroll-area runtime РґР»СЏ modal/panel viewport-РѕРІ: wheel input, drag thumb, click on track, visibility scrollbar Рё РїСЂРёРѕСЂРёС‚РµС‚ РіСЂСѓРїРї РІРІРѕРґР°.
- `src/ui/select_field.rs`: Dropdown/select-РєРѕРјРїРѕРЅРµРЅС‚ РґР»СЏ UI-РїР°РЅРµР»РµР№, РґРёРЅР°РјРёС‡РµСЃРєР°СЏ РїРµСЂРµСЂРёСЃРѕРІРєР° option buttons РїСЂРё СЃРјРµРЅРµ СЃРїРёСЃРєР° Рё РµРіРѕ С‚РµСЃС‚С‹.
- `src/ui/sim_controls.rs`: UI-РєРѕРЅС‚СЂРѕР»С‹ СЃРёРјСѓР»СЏС†РёРё (pause/speed/hotkeys).
- `src/ui/toggle_switch.rs`: РџРµСЂРµРёСЃРїРѕР»СЊР·СѓРµРјС‹Р№ РґРІСѓС…РїРѕР·РёС†РёРѕРЅРЅС‹Р№ toggle-switch UI-РєРѕРјРїРѕРЅРµРЅС‚ РґР»СЏ РІРєР»СЋС‡РµРЅРёСЏ/РІС‹РєР»СЋС‡РµРЅРёСЏ РЅР°СЃС‚СЂРѕРµРє.
- `src/world/grid.rs`: РљР»РµС‚РѕС‡РЅР°СЏ СЃРµС‚РєР° РјРёСЂР°, generic material ID wrapper, РєРѕРѕСЂРґРёРЅР°С‚РЅС‹Рµ СѓС‚РёР»РёС‚С‹ Рё С‚РµСЃС‚С‹.
- `src/world/mod.rs`: РџР»Р°РіРёРЅ РјРёСЂР° Рё СЃРѕР±С‹С‚РёСЏ РёР·РјРµРЅРµРЅРёР№ РєР»РµС‚РѕРє.
- `src/world/structures.rs`: Unified layer/descriptor-РјРѕРґРµР»СЊ СЃС‚СЂСѓРєС‚СѓСЂ, generic structure/layer ID wrapper-С‹, `PlacedStructureMap`, rotation, bridge-footprint compatibility helpers Рё pipe-cut state.
- `xtask/Cargo.toml`: РњР°РЅРёС„РµСЃС‚ helper-crate-Р° РґР»СЏ СЃР±РѕСЂРєРё/СѓРїР°РєРѕРІРєРё runtime-РїР»Р°РіРёРЅРѕРІ Рё РіРµРЅРµСЂР°С†РёРё Plugin SDK РґРѕРєСѓРјРµРЅС‚Р°С†РёРё.
- `xtask/src/lib.rs`: Р РµР°Р»РёР·Р°С†РёСЏ РєРѕРјР°РЅРґ `build-plugin`, `build-plugin --dev`, `pack-plugin`, `build-all-plugins`, Plugin SDK docs РєРѕРјР°РЅРґ, discovery plugin projects, СѓСЃС‚Р°РЅРѕРІРєР° expanded output РІ `plugins_dev/<plugin_id>` Рё Р±РµР·РѕРїР°СЃРЅР°СЏ СѓРїР°РєРѕРІРєР° `.fluxplugin`.
- `xtask/src/plugin_sdk_docs.rs`: Orchestration-РјРѕРґСѓР»СЊ Plugin SDK docs РєРѕРјР°РЅРґ: СЃРѕР±РёСЂР°РµС‚ generated Markdown, stale-check Рё mdBook build.
- `xtask/src/plugin_sdk_docs/collector.rs`: РЎР±РѕСЂ Plugin SDK API-СЃСѓС‰РЅРѕСЃС‚РµР№ РёР· Rust AST С‡РµСЂРµР· `syn`: СЃС‚СЂСѓРєС‚СѓСЂС‹, РјРµС‚РѕРґС‹, callback-С‚РёРїС‹, РєРѕРЅСЃС‚Р°РЅС‚С‹ Рё СЃРѕР±С‹С‚РёСЏ, exclude-С„РёР»СЊС‚СЂР°С†РёСЏ РІРЅСѓС‚СЂРµРЅРЅРёС… helper-РѕРІ, mapping `PluginEvent -> typed payload` С‡РµСЂРµР· `AbiEventPayload`, fallback-РѕРїРёСЃР°РЅРёСЏ РїРѕР»РµР№/РІР°СЂРёР°РЅС‚РѕРІ Рё Р·Р°РіСЂСѓР·РєР° optional external example-snippets СЃ РїСЂРѕРїСѓСЃРєРѕРј legacy v4 ABI РїСЂРёРјРµСЂРѕРІ.
- `xtask/src/plugin_sdk_docs/model.rs`: РћР±С‰РёРµ РјРѕРґРµР»Рё generated Plugin SDK reference: РіСЂСѓРїРїС‹ API, item docs, РїРѕР»СЏ, Р°СЂРіСѓРјРµРЅС‚С‹, РІР°СЂРёР°РЅС‚С‹, source metadata Рё СЃС…РµРјР° РїСѓС‚РµР№ РґР»СЏ external examples.
- `xtask/src/plugin_sdk_docs/parser.rs`: РџР°СЂСЃРёРЅРі SDK-facing Rustdoc С‡РµСЂРµР· `syn`, РёР·РІР»РµС‡РµРЅРёРµ summary/section-Р±Р»РѕРєРѕРІ Рё РїРѕРґРґРµСЂР¶РєР° `#[doc(hidden)]` РґР»СЏ РёСЃРєР»СЋС‡РµРЅРёСЏ РІРЅСѓС‚СЂРµРЅРЅРёС… SDK helper-РѕРІ РёР· generated reference.
- `xtask/src/plugin_sdk_docs/render.rs`: Р РµРЅРґРµСЂ generated API items РІ Markdown/HTML-Р±Р»РѕРєРё mdBook, РІРєР»СЋС‡Р°СЏ cross-links РЅР° РґРѕРєСѓРјРµРЅС‚РёСЂРѕРІР°РЅРЅС‹Рµ SDK-С‚РёРїС‹, СЃРїРёСЃРєРё РјРµС‚РѕРґРѕРІ СЃС‚СЂСѓРєС‚СѓСЂ Рё РїРѕРґРєР»СЋС‡РµРЅРёРµ external example-snippets.
- `xtask/src/main.rs`: CLI entrypoint, РєРѕС‚РѕСЂС‹Р№ Р·Р°РїСѓСЃРєР°РµС‚ `xtask::run_from_env()` Рё РІРѕР·РІСЂР°С‰Р°РµС‚ non-zero exit code РїСЂРё РѕС€РёР±РєРµ.
- `tmp_size.rs`: Р’СЂРµРјРµРЅРЅС‹Р№ Р»РѕРєР°Р»СЊРЅС‹Р№ РІСЃРїРѕРјРѕРіР°С‚РµР»СЊРЅС‹Р№ Rust-С„Р°Р№Р» РґР»СЏ СЂСѓС‡РЅС‹С… РїСЂРѕРІРµСЂРѕРє/С‡РµСЂРЅРѕРІС‹С… СЌРєСЃРїРµСЂРёРјРµРЅС‚РѕРІ.



