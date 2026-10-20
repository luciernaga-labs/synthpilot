# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace with `synthpilot-core` and the `synthpilot` desktop app.
- Device catalog with a Yamaha MODX+ profile declaring the AWM2 and FM-X engines.
- Verified AWM2 Element and FM-X Common and Operator parameter mappings from the MODX+ Data List.
- Yamaha parameter change SysEx encoding with validation.
- In-memory sounds with sixteen Parts.
- Desktop editor with a SysEx preview, opened from a single screen that picks the synthesizer and its MIDI port.
- `synthpilot-midi` crate with MIDI input and output on the same port.
- Reading the current sound from the MODX+ over Bulk Dump: Performance name, Part names, engines and values.
- Part parameters from the Performance Part table that work on any sound, and a Drum engine with Part-level controls.
- Live editing: edits are sent as parameter changes, and parameter changes sent by the keyboard update the editor.
- Parameter tooltips that explain what each Part, Element and Operator parameter does.
- Pull request checks for unit tests, a 90% line coverage gate and an autofix workflow that applies `cargo fmt` and `cargo clippy --fix`.

### Changed

- Interface text rewritten for musicians: consistent terms (keyboard, Performance, Part, Common, read, store), error messages that say what happened and what to do with the MODX+ menu path, and "Not editable yet" instead of "Not implemented yet".
- Editor layout separates what changes the whole Part from what changes one Element or Operator, with colored sections and Part offsets marked with "±".
- Element and Operator tabs show whether each layer is on and how loud it is, warn when the selected one is off or very quiet, and open on a layer you can hear.
- Sliders show their value, accept typed values, mark the center of bipolar ranges and offer a right-click reset to the keyboard value or the default.
- Parameters that are not editable yet are grouped in a collapsed list instead of filling the editor.
- Reading progress, empty Parts and errors are shown in the editor, and the editor is locked while the keyboard is being read.
- The SysEx panel is now a one-line status bar with a "Show MIDI details" toggle, and the top bar has a MIDI activity light.
- Larger text, higher contrast and keyboard shortcuts: `[` and `]` or Ctrl/Cmd+number switch layers, Enter connects on the start screen.
