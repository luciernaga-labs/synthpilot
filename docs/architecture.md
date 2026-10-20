# Architecture

SynthPilot is a Cargo workspace with three crates. All device knowledge lives in `synthpilot-core`. `synthpilot-midi` moves bytes to a MIDI port. The desktop application in `synthpilot` only draws what the core describes and hands the resulting messages to the transport.

```text
synthpilot (app) ──▶ domain ◀── devices/modx_plus ──▶ protocol
        │
        └──────────▶ synthpilot-midi ──▶ midir (CoreMIDI, WinMM, ALSA)
```

| Layer | Crate and path | Responsibility | Must not depend on |
| --- | --- | --- | --- |
| Protocol | `synthpilot-core/src/protocol` | SysEx framing, the generic Yamaha parameter change format and 7-bit data encoding | Devices, UI, MIDI transport |
| Domain | `synthpilot-core/src/domain` | Identifiers, parameter definitions, value ranges and formats, validation, the `DeviceProfile` and `EngineProfile` traits, the `Sound` model | Any specific manufacturer, UI |
| Devices | `synthpilot-core/src/devices` | The device catalog and one module per instrument. Addresses, model IDs and value mappings stay here | UI |
| Transport | `synthpilot-midi` | Listing MIDI ports, opening an input and output on the same port, and moving bytes through the `Connection` trait | Devices, UI |
| App | `synthpilot/src/app` | Device selector, sound editor, SysEx preview and connection controls built with eframe/egui. Logic lives in `state.rs`, `controller.rs` and `midi.rs`; the view modules only draw and collect actions | Manufacturer-specific modules |

`synthpilot-core` has no GUI or MIDI dependency, so it can be reused from a command-line tool, tests or a future plugin.

## Core concepts

- **`DeviceProfile`**: a synthesizer model. It exposes its identity, part count and engines.
- **`EngineProfile`**: a synthesis engine of a device, such as AWM2 or FM-X. It publishes its `ParameterDefinition`s, its `Layout` (parts and elements or operators) and encodes a validated `ParameterChange` into a `SysExMessage`.
- **`ParameterDefinition`**: stable ID, display name, category, scope (whole part or element), value range, default, unit, display format, support status and optional help text shown as a tooltip. `Support::Implemented` carries a `SourceReference` to the document page the mapping was verified against.
- **`Sound`**: an in-memory sound with a name and one slot per Part. `Sound::apply` validates a change, encodes it and only then stores the new value; `Sound::store` stores a validated change received from the instrument.
- **`DeviceProfile::read_requests` and `DeviceProfile::decode`**: how a device asks for its current sound and turns incoming bytes into `Received` events (parts, names, values).
- **`SoundReader`**: collects `Received` events and builds the `Sound` once the instrument has answered.

Encoding and decoding are pure functions on bytes. Moving bytes is a separate step: the app's controller sends the read requests when a connection opens, feeds incoming bytes through the device's decoder into a `SoundReader`, and forwards each edit to the MIDI link. `RecordingConnection` implements `Connection` in memory, so the whole read-and-edit flow is tested without hardware.

## Data-driven catalogs

Each engine declares a constant table of entries. An entry is either:

- `Entry::verified(definition, mapping)`: a parameter whose address, data size and raw range were checked against the documentation. The mapping also records the page number.
- `Entry::unverified(definition, reason)`: a parameter that exists on the instrument but cannot be encoded yet. It is shown in the UI and never produces a message.

Constructors are `const fn` and assert their invariants. A mismatch between the display range and the raw data range, a default outside its range or a raw value that does not fit in the data size fails to compile.

## Adding a synthesizer

1. Create `crates/synthpilot-core/src/devices/<model>/` with a type implementing `DeviceProfile` and one `EngineProfile` per engine.
2. Keep every address, model ID and value mapping inside that module, and cite the source page for each verified parameter.
3. Add a `DeviceDescriptor` for it to the catalog in `crates/synthpilot-core/src/devices/mod.rs`.
4. Add golden SysEx tests under `crates/synthpilot-core/tests/`.

The application picks up the new device, its engines and their parameters without changes.

## Adding an engine to an existing device

Implement `EngineProfile`, add it to the device's engine list and declare its parameter table. The editor groups parameters by category and scope, and labels elements with `EngineProfile::element_label`, so no UI code is needed.
