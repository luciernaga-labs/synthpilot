# SynthPilot

[![CI](https://github.com/luciernaga-labs/synthpilot/actions/workflows/ci.yml/badge.svg)](https://github.com/luciernaga-labs/synthpilot/actions/workflows/ci.yml)
[![Coverage gate: 90%](https://img.shields.io/badge/coverage%20gate-90%25-brightgreen.svg)](CONTRIBUTING.md#continuous-integration)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Open-source MIDI synthesizer editor written in Rust, starting with the Yamaha MODX+.

SynthPilot connects to your synthesizer, reads the Performance it is playing and lets you edit it with readable names, ranges and values. You never type SysEx addresses. Every edit shows the exact System Exclusive message it produces, together with the page of the official documentation it comes from, and you hear the change on the keyboard as you make it.

> [!NOTE]
> SynthPilot needs the synthesizer connected over MIDI. It reads the Performance from the keyboard and sends every change back to it.

## Features

- Device selector, starting with the **Yamaha MODX+**.
- **Reads the current Performance from the keyboard**: the Performance name, and every Part with its name, engine (AWM2, FM-X or Drum) and values.
- Part controls that work on any Performance (Volume, Pan, Note Shift and offsets for the amplitude envelope and filter), plus per-Element (AWM2) and per-Operator (FM-X) parameters.
- Every edit is sent to the keyboard as a parameter change, and edits made on the keyboard show up in the app.
- Validated values: out-of-range input is rejected before any bytes are produced.
- SysEx preview split into header, address and data, with a source reference for each parameter.
- Parameters that exist on the instrument but are not verified yet are listed and clearly marked **Not editable yet**.
- Tooltips that explain what each parameter does and how it sounds.

## Getting started

Install Rust with [rustup](https://rustup.rs), then:

```sh
git clone https://github.com/luciernaga-labs/synthpilot.git
cd synthpilot
cargo run --release
```

SynthPilot builds on macOS, Windows and Linux. On Linux you need the ALSA development package (`libasound2-dev` on Debian or Ubuntu) and may need the usual windowing packages (`libxkbcommon-dev`, `libwayland-dev`, `libgl1-mesa-dev`).

## Hearing your edits on a Yamaha MODX+

1. Connect the MODX+ **USB TO HOST** port to your computer with a USB cable.
2. On the MODX+, go to **Utility > Settings > MIDI I/O** and set **MIDI IN/OUT** to **USB**, **Device Number** to **All** and **Bulk Receive** to **On**.
3. Select the Performance you want to edit on the keyboard.
4. Start SynthPilot, check that **Yamaha MODX+** and **MODX Port 1** are selected and select **Connect and read Performance**. The app reads the Performance and lists its Parts.
5. Pick a Part, play the keyboard and move the controls. Select **Read from keyboard** after choosing another Performance on the instrument.

Changes go to the keyboard's edit buffer. Store the Performance on the MODX+ to keep them.

## Project layout

```text
crates/
├── synthpilot-core   domain model, device profiles and SysEx encoding (no GUI or MIDI dependencies)
│   └── src/
│       ├── domain/      devices, engines, parameters, validation, sounds
│       ├── protocol/    SysEx framing and the Yamaha parameter change format
│       └── devices/     device catalog and the Yamaha MODX+ profile
├── synthpilot-midi   MIDI output transport built on midir
└── synthpilot        desktop application built with eframe/egui
```

See [docs/architecture.md](docs/architecture.md) for how the pieces fit together and how to add a new synthesizer or engine.

## Supported parameters

The implemented MODX+ parameters, their addresses and the Data List pages they were verified against are listed in [docs/modx-plus.md](docs/modx-plus.md).

## Current limitations

- SynthPilot reads the Performance edit buffer but never writes bulk dumps, stores Performances or touches firmware.
- Drum keys, effects, arpeggios, Motion Sequences and most Performance-level settings are not modelled yet.
- The SysEx device number is fixed to `0`.
- Sounds live only in memory; there is no file import or export yet.
- Only a verified subset of parameters is editable.
- SynthPilot is a manual editor. It does not generate or suggest sounds.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Every protocol mapping has to cite the page of the official documentation it was verified against.

## License

Licensed under the [MIT License](LICENSE).

SynthPilot is an independent project and is not affiliated with or endorsed by Yamaha Corporation. Yamaha, MODX, AWM2 and FM-X are trademarks of Yamaha Corporation.
