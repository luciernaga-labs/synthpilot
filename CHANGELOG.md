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
- Effects for the MODX+: Reverb and Variation type, return, pan and switch for the whole sound; Insertion A/B type, Insertion FX Switch, Insertion Connection Type, Reverb Send, Variation Send, Dry Level and 3-band EQ gains per Part; Element Connection Switch per AWM2 Element.
- Performance-wide parameters in the domain model (`Scope::Performance`, `Location::performance`, `DeviceProfile::common`).
- `.env.example` with the settings for connecting an AI model (provider, model, API key and optional base URL). Local `.env` files are ignored by git.
- Optional AI assistant (`synthpilot-assistant` crate and Assistant panel): describe a sound and an OpenAI, Anthropic or local OpenAI-compatible model configured in `.env` proposes changes to the selected Part and the Performance effects, which you review, apply and undo.
- Wave selection for AWM2 Elements by number and name from the 6347 Preset Waves, with a searchable picker.
- AWM2 Element amplitude envelope levels, pitch and filter velocity sensitivity, filter envelope and LFO; FM-X filter envelope, velocity sensitivity, 2nd LFO, spectral form, operator envelope levels, level scaling depths and LFO sensitivity.
- Effect parameters for 89 effect types, named and limited by the selected Reverb, Variation or Insertion type. SynthPilot reads the Performance again after an effect type changes.
- The AI assistant can look up Wave groups and effect type parameters before proposing changes, and validates effect parameters against the type it chooses.
- Part Common settings from the Performance Part table: Part Category Main (Piano, Pad/Choir, Chromatic Perc and the rest), Mono/Poly, Key Assign, Portamento, Pitch Bend Range, Velocity and Note Limit, Part Output, Element Pan and Arp Play Only.
- Effect type dropdowns are grouped by the Effect Type List categories, as on the instrument.
- The AI assistant works on the whole Performance: it sees every Part (the open one in full, the others summarized, and any of them in full on request), and each proposed change names the Part it edits, so it can balance, layer and split Parts. It can look things up up to three times before answering.
- Loading any of the 2227 preset sounds of the MODX+ into a Part, from a searchable list by name and category: fill an empty Part or replace a Part's sound. SynthPilot reads the Part back to confirm it loaded. The AI assistant can propose sounds to load too.
- Part Switch, Keyboard Control and Mute per Part.

### Fixed

- Insertion B no longer offers VINYL BREAK and the four BEAT REPEAT types, which the Effect Type List makes available on Insertion A only, and numbers the types after them as the InsB column does.
- Velocity and note ranges, velocity cross fade and velocity-dependent envelope times for AWM2 Elements, and Velocity Sensitivity Depth and Offset per Part, so layered sounds respond to playing strength. The AI assistant follows layering and attack guidance and always answers in English.

### Changed

- Interface text rewritten for musicians: consistent terms (keyboard, Performance, Part, Common, read, store), error messages that say what happened and what to do with the MODX+ menu path, and "Not editable yet" instead of "Not implemented yet".
- Editor layout separates what changes the whole Part from what changes one Element or Operator, with colored sections and Part offsets marked with "±".
- Element and Operator tabs show whether each layer is on and how loud it is, warn when the selected one is off or very quiet, and open on a layer you can hear.
- Sliders show their value, accept typed values, mark the center of bipolar ranges and offer a right-click reset to the keyboard value or the default.
- Parameters that are not editable yet are grouped in a collapsed list instead of filling the editor.
- Reading progress, empty Parts and errors are shown in the editor, and the editor is locked while the keyboard is being read.
- The SysEx panel is now a one-line status bar with a "Show MIDI details" toggle, and the top bar has a MIDI activity light.
- Larger text, higher contrast and keyboard shortcuts: `[` and `]` or Ctrl/Cmd+number switch layers, Enter connects on the start screen.
