# Show Me Your Skin! for Horizon

![icon](showmeyourskin-icon.png)

A Horizon native port of [enjarai's Show Me Your Skin!](https://modrinth.com/mod/show-me-your-skin). Hide or fade armour, elytra, hats, shields and name tags, separately for yourself and for other players.

## Status

| | |
|---|---|
| Version | 2.0.4 |
| Horizon | Nightly 2026-11-13 (`9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`) |
| Game | 26.3 |
| Tested | **No.** 2.0.4 only moves the pin from Nightly 2026-11-05 to 2026-11-13. It hasn't been built or tested on the new nightly yet. |

The bundle pins a single Horizon commit because armour, elytra, head items, shields and the pumpkin overlay are drawn by game functions that the named hook catalog doesn't cover.

## Install

Download `showmeyourskin-2.0.4.hnmod` from Releases and add it in the Horizon launcher. Open the settings from the native mods menu.

## Layout

- `horizon-native.json`: bundle manifest
- `mixins/showmeyourskin/horizon-mixin.toml`: hooks and injections
- `mixins/showmeyourskin/glue.rs`: the mod's logic and settings screen
- `mixins/showmeyourskin/h_*.rs`: per-hook helpers

## Credits and licence

Original mod © 2022 enjarai, MIT. Horizon port © 2026 Horizon port contributors, MIT. See [LICENSE](LICENSE).
