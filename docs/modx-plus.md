# Yamaha MODX+

## Source

All mappings were verified against the **MODX+ Data List** published by Yamaha Corporation (`modx_plus_en_dl_a0.pdf`, 223 pages, dated 2022-08-29), available from the [Yamaha download site](https://data.yamaha.com/files/download/other_assets/5/1583935/modx_plus_en_dl_a0.pdf). The PDF is not redistributed in this repository.

Page numbers refer to the page numbers printed in the Data List.

## Message format

Parameter Change, MIDI Data Format section (3-5-3-1), pages 187–188:

```text
F0 43 1n 7F 1C 07 ah am al dd … dd F7
```

| Bytes | Meaning |
| --- | --- |
| `F0` | Exclusive status |
| `43` | Yamaha ID |
| `1n` | Parameter change, `n` = device number (`0`–`F`) |
| `7F 1C` | Group ID high and low |
| `07` | Model ID |
| `ah am al` | Address high, mid and low |
| `dd …` | Data, one byte per unit of the parameter's size |
| `F7` | End of exclusive |

Parameters with a size of 2 are sent as two 7-bit bytes, most significant first ("1st bit6-0 → bit13-7, 2nd bit6-0 → bit6-0").

SynthPilot sends device number `0`. The Data List does not state how `n` relates to the MIDI Device Number setting on the instrument, so set that setting to **All** to make sure the MODX+ accepts the messages.

## Receiving over USB

The MIDI Parameter Change Table (System) on page 194 lists **MIDI IN/OUT** (`00 00 18`, values MIDI or USB, default MIDI). Set it to USB on the instrument when connecting with a USB cable.

Known erratum: the binary column of the format table shows the Model ID as `00000010`, while the hex column and every message diagram on page 187 show `07`. SynthPilot follows `07`.

## Reading the current sound

When a connection opens, SynthPilot sends a Dump Request (section 3-5-5, page 188) for every Part of the Performance edit buffer and for its Common block (MIDI Parameter Change Table (Bulk Control), page 193):

```text
F0 43 20 7F 1C 07 0E 52 nn F7    Part nn (00–0F)
F0 43 20 7F 1C 07 0E 53 00 F7    Performance Common
```

The MODX+ replies with a Bulk Dump (section 3-5-4, page 188) for each block, framed by a Bulk Header (`0E 52 nn`) and a Bulk Footer (`0F 52 nn`):

```text
F0 43 0n 7F 1C bh bl 07 ah am al dd … dd cc F7
```

`bh bl` is a byte count in seven-bit groups and `cc` is a checksum. The Data List describes the checksum as covering the byte count, address and data (page 188), and the Bulk Dump Block table (page 192) lists only the data size of each block. A MODX+ running firmware for this Data List was observed sending this Bulk Header:

```text
F0 43 00 7F 1C 00 04 07 0E 52 00 19 F7
```

Here the byte count (`00 04`) counts the Model ID, the address and the data, and the checksum makes the lower seven bits of the sum of those same bytes plus the checksum zero (`07 + 0E + 52 + 00 + 19 = 80`). SynthPilot reads both this observed form and the documented one, and ignores messages that match neither. If nothing readable arrives, the app shows the first bytes the keyboard sent.

The blocks that arrive tell SynthPilot what each Part is: `41 ep`/`42 ep` for AWM2, `48 0p`/`49 op` for FM-X and `5p kk` for Drum. The Part name is bytes `00`–`13` of the Performance Part block (`31 0p`, page 207) and the Performance name is bytes `00`–`13` of the Performance Common block (`30 40 00`, page 197).

The Data List links the Bulk Dump Request to the Bulk Receive switch (page 187), so Bulk Receive must be On for the MODX+ to reply. Parameter Changes the instrument transmits while you edit on it are decoded with the same tables.

## Loading a preset sound into a Part

SynthPilot can put any of the 2227 preset Performances of the Performance List (pages 2–17) into a Part, to fill an empty Part or replace a Part's sound. The list, with each Performance's Main and Sub Category, is in `crates/synthpilot-core/src/devices/modx_plus/performances.tsv`.

It sends a Bank Select and Program Change on the Part's MIDI channel (Part 1 on channel 1 … Part 16 on channel 16):

```text
Bn 00 3F    Bank Select MSB 63: Performance (Single Part)
Bn 20 ll    Bank Select LSB ll = (number - 1) / 128
Cn pp       Program Change pp = (number - 1) % 128
```

- The MIDI Data Format (page 187) gives Bank Select MSB 63 with LSB 0–31 for the Single Part Performance banks Preset 1–32, 128 Programs each, and the Bulk Control table (page 193) splits the presets into the same banks of 128 (Performance PRE 1–32). That gives the LSB and Program of each number of the Performance List: for example 955 Gamelan is LSB `07`, Program `3A`.
- The Data List does not say what the instrument does with a Single Part bank on a Part's channel. On Yamaha's community forum, users explain that the MODX places the first Part of that Performance in the Part on that channel, and that for a Performance with several Parts only the first one loads. SynthPilot therefore reads the Performance again about a second later and reports whether the Part took the sound.
- It needs MIDI I/O Mode set to Multi (System `00 00 30` = `00`, the default, page 194), so that each Part listens on its own channel, and the Bank Select and Program Change receive switches on.
- Parts 9 to 16 only play over MIDI: Keyboard Control is fixed to Off for them (page 207). Use Parts 1 to 8 for sounds you play from the keys.
- Undo does not bring back the previous sound of a Part.

## Address blocks

From the Parameter Base Address table on page 191. `p` is the part number (`0`–`F`), `e` the element number (`0`–`7`) and `o` the operator number (`0`–`7`).

| Block | Address | Used by |
| --- | --- | --- |
| Performance Part | `31 0p 00` | Every Part |
| Normal Part Element Group 1: Oscillator, Amplitude, Pitch | `41 ep 00` | AWM2 |
| Normal Part Element Group 2: Filter, EQ, LFO | `42 ep 00` | AWM2 |
| FM Part Common | `48 0p 00` | FM-X |
| FM Part Operator | `49 op 00` | FM-X |
| Reverb | `30 41 00` | Performance effects |
| Variation | `30 42 00` | Performance effects |
| Insertion A | `31 2p 00` | Every Part |
| Insertion B | `31 3p 00` | Every Part |

## Implemented parameters

### Every Part: MIDI Parameter Change Table (Performance Part)

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Volume | `31 0p 24` | 1 | `00`–`7F` | 0 – 127 | 100 | 207 |
| Pan | `31 0p 25` | 1 | `01`–`7F` | L63 – C – R63 | C | 207 |
| Velocity Sensitivity Depth | `31 0p 22` | 1 | `00`–`7F` | 0 – 127 | 64 | 207 |
| Velocity Sensitivity Offset | `31 0p 23` | 1 | `00`–`7F` | 0 – 127 | 64 | 207 |
| Note Shift | `31 0p 2D` | 1 | `28`–`58` | -24 – +24 semitones | 0 | 207 |
| AEG Attack Time | `31 0p 46` | 1 | `00`–`7F` | -64 – +63 | 0 | 208 |
| AEG Decay Time | `31 0p 47` | 1 | `00`–`7F` | -64 – +63 | 0 | 208 |
| AEG Release Time | `31 0p 49` | 1 | `00`–`7F` | -64 – +63 | 0 | 208 |
| Filter Cutoff Frequency | `31 0p 50` | 1 | `00`–`7F` | -64 – +63 | 0 | 208 |
| Filter Resonance/Width | `31 0p 51` | 1 | `00`–`7F` | -64 – +63 | 0 | 208 |

Note Shift and AEG Release Time are not available for Drum Parts, so Drum Parts do not list them.

### Every Part: Part Common settings (MIDI Parameter Change Table (Performance Part))

These are the settings of the instrument's Edit > Part Common > General and Pitch screens.

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Part Category Main | `31 0p 14` | 1 | `00`–`10` | Piano … Ethnic, No Assign (Performance Category List, page 18) | No Assign | 207 |
| Velocity Limit Low and High | `31 0p 1C`–`1D` | 1 | `01`–`7F` | 1 – 127 | 1, 127 | 207 |
| Note Limit Low and High | `31 0p 1E`–`1F` | 1 | `00`–`7F` | C-2 – G8 | C-2, G8 | 207 |
| Pitch Bend Range Upper | `31 0p 20` | 1 | `10`–`58` | -48 – +24 semitones | +2 | 207 |
| Pitch Bend Range Lower | `31 0p 21` | 1 | `10`–`58` | -48 – +24 semitones | -2 | 207 |
| Portamento Switch | `31 0p 31` | 1 | `00`–`01` | Off, On | Off | 207 |
| Portamento Time | `31 0p 32` | 1 | `00`–`7F` | 0 – 127 | 64 | 207 |
| Portamento Mode | `31 0p 33` | 1 | `00`–`01` | Fingered, Full-time | Full-time | 207 |
| Key Assign Mode | `31 0p 35` | 1 | `00`–`01` | Single, Multi | Multi | 207 |
| Mono/Poly Mode | `31 0p 36` | 1 | `00`–`01` | Mono, Poly | Poly | 207 |
| Part Output Select | `31 0p 3F` | 1 | `00`, `09`–`0C`, `42`–`49`, `7D`, `7F` | MainL&R, USB1&2 … USB7&8, USB1 … USB8, Off, Drum | MainL&R | 207 |
| Element Pan Switch | `31 0p 45` | 1 | `00`–`01` | Off, On | On | 208 |
| Arp Play Only | `31 6p 03` | 1 | `00`–`01` | Off, On | Off | 210 |
| Part Switch | `31 0p 16` | 1 | `00`–`01` | Off, On | On | 207 |
| Keyboard Control Switch | `31 0p 17` | 1 | `00`–`01` | Off, On (fixed to Off for Part 9 or later) | Off | 207 |
| Mute Switch | `31 0p 18` | 1 | `00`–`01` | Off, On | Off | 207 |

Portamento, Key Assign and Mono/Poly are not available for Drum Parts, so Drum Parts do not list them. Part Output Select offers Drum (`7F`) only on Drum Parts, as the Data List notes. The Main Category numbers follow the order of the Performance Category List, with `10` (the default) as No Assign.

### Every Part: effects routing and EQ (MIDI Parameter Change Table (Performance Part))

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Insertion FX Switch | `31 0p 26` | 1 | `00`–`03` | Both off, A on, B on, A and B on | A and B on | 207 |
| Reverb Send | `31 0p 29` | 1 | `00`–`7F` | 0 – 127 | 12 | 207 |
| Variation Send | `31 0p 2A` | 1 | `00`–`7F` | 0 – 127 | 0 | 207 |
| Dry Level | `31 0p 2C` | 1 | `00`–`7F` | 0 – 127 | 127 | 207 |
| Insertion Connection Type | `31 0p 3B` | 1 | `00`–`02` | Parallel, Ins A->B, Ins B->A | Parallel | 207 |
| 3-band EQ Low Gain | `31 0p 55` | 1 | `20`–`60` | -12.00 dB – +12.00 dB | 0.00 dB | 208 |
| 3-band EQ Mid Gain | `31 0p 59` | 1 | `20`–`60` | -12.00 dB – +12.00 dB | 0.00 dB | 208 |
| 3-band EQ High Gain | `31 0p 5D` | 1 | `20`–`60` | -12.00 dB – +12.00 dB | 0.00 dB | 208 |
| Insertion A Type | `31 2p 00` | 2 | Effect Type List | THRU and 88 effect types | THRU | 209 |
| Insertion B Type | `31 3p 00` | 2 | Effect Type List | THRU and 83 effect types | THRU | 209 |

The Data List notes that Parallel is not available for FM Parts, so FM-X Parts offer only Ins A->B and Ins B->A.

The EQ gain table gives only the end points of the range. SynthPilot shows the 64 steps between `20` and `60` as equal steps of 0.375 dB, which matches both end points and `40` = 0 dB.

Effect types are two bytes, MSB first, taken from the Type (HEX) MSB and LSB columns of the Effect Type List (pages 144–145). The editor lists the types of the matching column: REV for Reverb, VAR for Variation, InsA and InsB for the insertions. The InsB column has no number for VINYL BREAK and the four BEAT REPEAT types, so Insertion B offers 83 types and numbers VCM EQ 501 as 71, while Insertion A offers all 88. The dropdowns group the types under the Category headings of the list (Reverb, Delay, Chorus, Flanger, Phaser, Tremolo & Rotary, Distortion, Compressor, Wah, Lo-Fi, Tech, Misc), as the instrument does.

Every type name and every MSB/LSB pair in SynthPilot was compared one by one with pages 144–145: the 12 reverbs and 88 effect types all match.

### AWM2: effect routing (MIDI Parameter Change Table (Normal Part Element))

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Element Connection Switch | `41 ep 17` | 1 | `00`–`02` | Thru, InsA, InsB | InsA | 217 |

### Performance effects: MIDI Parameter Change Table (Performance Common)

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Reverb Type | `30 41 00` | 2 | Effect Type List, REV column | NO EFFECT and 12 reverb types | REV-X HALL (`01 00`) | 197 |
| Reverb Return | `30 41 44` | 1 | `00`–`7F` | 0 – 127 | 64 | 198 |
| Reverb Pan | `30 41 45` | 1 | `01`–`7F` | L63 – C – R63 | C | 198 |
| Reverb Switch | `30 41 46` | 1 | `00`–`01` | Off, On | On | 198 |
| Variation Type | `30 42 00` | 2 | Effect Type List, VAR column | NO EFFECT and 88 effect types | G CHORUS (`03 00`) | 198 |
| Variation Return | `30 42 44` | 1 | `00`–`7F` | 0 – 127 | 96 | 198 |
| Variation Pan | `30 42 45` | 1 | `01`–`7F` | L63 – C – R63 | C | 198 |
| Send Variation To Reverb | `30 42 46` | 1 | `00`–`7F` | 0 – 127 | 0 | 198 |
| Variation Switch | `30 42 47` | 1 | `00`–`01` | Off, On | On | 198 |

Reverb and Variation arrive with the Performance Common bulk dump, so they are read with the rest of the sound.

### AWM2: MIDI Parameter Change Table (Normal Part Element)

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Element Switch | `41 ep 00` | 1 | `00`–`01` | Off, On | On for Element 1, Off otherwise | 217 |
| XA Control | `41 ep 0C` | 1 | `00`–`07` | Normal … A.SW2 On | Normal | 217 |
| Element Pan | `41 ep 08` | 1 | `01`–`7F` | L63 – C – R63 | C | 217 |
| Element Level | `41 ep 28` | 1 | `00`–`7F` | 0 – 127 | 127 | 217 |
| Level Velocity Sensitivity | `41 ep 29` | 1 | `00`–`7F` | -64 – +63 | 0 | 217 |
| AEG Attack Time | `41 ep 2D` | 1 | `00`–`7F` | 0 – 127 | 0 | 217 |
| AEG Decay 1 Time | `41 ep 2E` | 1 | `00`–`7F` | 0 – 127 | 64 | 217 |
| AEG Decay 2 Time | `41 ep 2F` | 1 | `00`–`7F` | 0 – 127 | 64 | 217 |
| AEG Release Time | `41 ep 31` | 1 | `00`–`7F` | 0 – 127 | 50 | 217 |
| Coarse Tune | `41 ep 49` | 1 | `10`–`70` | -48 – +48 | 0 | 217 |
| Fine Tune | `41 ep 4A` | 1 | `00`–`7F` | -64 – +63 | 0 | 217 |
| Filter Cutoff Frequency | `42 ep 01` | 2 | `00 00`–`01 7F` | 0 – 255 | 160 | 218 |
| Filter Resonance/Width | `42 ep 05` | 1 | `00`–`7F` | 0 – 127 | 0 | 218 |

### AWM2: Waves, envelopes, filter EG and LFO (MIDI Parameter Change Table (Normal Part Element))

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Wave Bank (Wave Select) | `41 ep 01` | 1 | `00`–`09` | Preset, User, Library 1–8 | Preset | 217 |
| Wave (Wave Number) | `41 ep 03` | 2 | `00 01`–`31 4B` | 1 – 6347, named from the Waveform List | 1 | 217 |
| Note Limit Low and High | `41 ep 0D`–`0E` | 1 | `00`–`7F` | C-2 – G8 | C-2, G8 | 217 |
| Velocity Limit Low and High | `41 ep 0F`–`10` | 1 | `01`–`7F` | 1 – 127 | 1, 127 | 217 |
| Velocity Cross Fade | `41 ep 11` | 1 | `00`–`7F` | 0 – 127 | 0 | 217 |
| AEG Time Velocity Segment | `41 ep 37` | 1 | `00`–`04` | Attack, Atk+Dcy, Decay, Atk+Rls, All | All | 217 |
| AEG Time Velocity Sensitivity | `41 ep 38` | 1 | `00`–`7F` | -64 – +63 | 0 | 217 |
| AEG Time Key Follow Sensitivity | `41 ep 39` | 1 | `00`–`7F` | -64 – +63 | 0 | 217 |
| AEG Initial Level | `41 ep 32` | 1 | `00`–`7F` | 0 – 127 | 0 | 217 |
| AEG Attack Level | `41 ep 33` | 1 | `00`–`7F` | 0 – 127 | 127 | 217 |
| AEG Decay 1 Level | `41 ep 34` | 1 | `00`–`7F` | 0 – 127 | 127 | 217 |
| AEG Decay 2 Level | `41 ep 35` | 1 | `00`–`7F` | 0 – 127 | 127 | 217 |
| Pitch Velocity Sensitivity | `41 ep 4B` | 1 | `00`–`7F` | -64 – +63 | 0 | 217 |
| Filter Cutoff Velocity Sensitivity | `42 ep 03` | 1 | `00`–`7F` | -64 – +63 | 0 | 218 |
| FEG Hold, Attack, Decay 1, Decay 2 and Release Time | `42 ep 0E`–`12` | 1 | `00`–`7F` | 0 – 127 | 0, 0, 64, 64, 80 | 218 |
| FEG Hold, Attack, Decay 1, Decay 2 and Release Level | `42 ep 13`, `15`, `17`, `19`, `1B` | 2 | `00 00`–`01 7F` | -128 – +127 | 0, +127, +127, +127, 0 | 218 |
| FEG Depth | `42 ep 1D` | 1 | `00`–`7F` | -64 – +63 | +40 | 218 |
| FEG Depth Velocity Sensitivity | `42 ep 20` | 1 | `00`–`7F` | -64 – +63 | 0 | 218 |
| LFO Wave | `42 ep 3A` | 1 | `00`–`02` | Saw, Triangle, Square | Triangle | 218 |
| LFO Delay Time | `42 ep 3C` | 1 | `00`–`7F` | 0 – 127 | 0 | 218 |
| LFO Speed | `42 ep 3D` | 1 | `00`–`3F` | 0 – 63 | 38 | 218 |
| LFO Amplitude, Pitch and Filter Modulation | `42 ep 3E`–`40` | 1 | `00`–`7F` | 0 – 127 | 0 | 218 |
| LFO Fade In Time | `42 ep 41` | 1 | `00`–`7F` | 0 – 127 | 0 | 218 |

Wave names come from the Waveform List (pages 62–81), which numbers the 6347 Preset Waves with their main and sub category. The names apply to the Preset bank; with a User or Library bank the number refers to that bank and the Preset name shown next to it does not apply.

Known erratum: the Data Range column of Wave Number shows `00`–`1F` for the first byte, which only reaches 4095, while the Description column and the Waveform List go up to 6347. SynthPilot follows the Description and sends the number as two 7-bit groups (`1st bit6-0 → bit13-7`), so Wave 6347 is `31 4B`. The Default `00 01` confirms that the value is the Wave number itself. Reading a Performance that uses a Wave above 4095 shows which form the instrument uses.

### FM-X: MIDI Parameter Change Table (FM Part Common)

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Filter Cutoff Frequency | `48 0p 0C` | 2 | `00 00`–`01 7F` | 0 – 255 | 255 | 219 |
| Filter Resonance/Width | `48 0p 0F` | 1 | `00`–`7F` | 0 – 127 | 10 | 219 |
| Algorithm Number | `48 0p 4F` | 1 | `00`–`57` | 1 – 88 | 1 | 220 |
| Feedback Level | `48 0p 50` | 1 | `00`–`07` | 0 – 7 | 0 | 220 |

### FM-X: MIDI Parameter Change Table (FM Part Operator)

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Oscillator Frequency Mode | `49 op 03` | 1 | `00`–`01` | Ratio, Fixed | Ratio | 220 |
| Tune Coarse | `49 op 04` | 1 | `00`–`1F` | 0 – 31 | 1 | 220 |
| Tune Fine | `49 op 05` | 1 | `00`–`7F` | 0 – 127 | 0 | 220 |
| Detune | `49 op 06` | 1 | `00`–`1E` | -15 – +15 | 0 | 220 |
| AEG Attack Time | `49 op 14` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| AEG Decay 1 Time | `49 op 15` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| AEG Release Time | `49 op 17` | 1 | `00`–`63` | 0 – 99 | 40 | 220 |
| Operator Level | `49 op 1A` | 1 | `00`–`63` | 0 – 99 | 99 for Operator 1, 0 otherwise | 220 |
| Level Velocity Sensitivity | `49 op 20` | 1 | `00`–`0E` | -7 – +7 | 0 | 220 |

### FM-X: filter EG, velocity and 2nd LFO (MIDI Parameter Change Table (FM Part Common))

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Pitch Velocity Sensitivity | `48 0p 06` | 1 | `00`–`7F` | -64 – +63 | 0 | 219 |
| Filter Cutoff Velocity Sensitivity | `48 0p 0E` | 1 | `00`–`7F` | -64 – +63 | 0 | 219 |
| FEG Hold, Attack, Decay 1, Decay 2 and Release Time | `48 0p 17`–`1B` | 1 | `00`–`7F` | 0 – 127 | 0 | 219 |
| FEG Hold, Attack, Decay 1, Decay 2 and Release Level | `48 0p 1C`, `1E`, `20`, `22`, `24` | 2 | `00 00`–`01 7F` | -128 – +127 | 0 | 219 |
| FEG Depth | `48 0p 26` | 1 | `00`–`7F` | -64 – +63 | +40 | 219 |
| FEG Depth Velocity Sensitivity | `48 0p 29` | 1 | `00`–`7F` | -64 – +63 | 0 | 219 |
| 2nd LFO Wave | `48 0p 47` | 1 | `00`–`05` | Triangle, Saw Down, Saw Up, Square, Sine, S/H | Triangle | 220 |
| 2nd LFO Speed | `48 0p 48` | 1 | `00`–`63` | 0 – 99 | 30 | 220 |
| 2nd LFO Delay Time | `48 0p 4A` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| 2nd LFO Pitch, Amplitude and Filter Modulation | `48 0p 4C`–`4E` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |

### FM-X: spectral form, envelopes and scaling (MIDI Parameter Change Table (FM Part Operator))

| Parameter | Address | Size | Data | Display | Default | Page |
| --- | --- | --- | --- | --- | --- | --- |
| Spectral Form | `49 op 09` | 1 | `00`–`06` | Sine, All 1, All 2, Odd 1, Odd 2, Res 1, Res 2 | Sine | 220 |
| Spectral Skirt | `49 op 0A` | 1 | `00`–`07` | 0 – 7 | 0 | 220 |
| Spectral Resonance | `49 op 0B` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| AEG Attack, Decay 1 and Decay 2 Level | `49 op 10`–`12` | 1 | `00`–`63` | 0 – 99 | 99 | 220 |
| AEG Release Level | `49 op 13` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| AEG Decay 2 Time | `49 op 16` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| AEG Hold Time | `49 op 18` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| AEG Time Key Follow | `49 op 19` | 1 | `00`–`07` | 0 – 7 | 0 | 220 |
| Level Scaling Low and High Depth | `49 op 1C`–`1D` | 1 | `00`–`63` | 0 – 99 | 0 | 220 |
| LFO Pitch and Amplitude Sensitivity | `49 op 21`–`22` | 1 | `00`–`07` | 0 – 7 | 3 | 220 |

### Effect parameters

Reverb (`30 41 03`–`31`), Variation (`30 42 03`–`31`), Insertion A (`31 2p 03`–`31`) and Insertion B (`31 3p 03`–`31`) each have 24 two-byte parameter slots (pages 197–198 and 209). What each slot means depends on the effect type, so SynthPilot names and limits them from the Effect Parameter List (pages 146–158): `crates/synthpilot-core/src/devices/modx_plus/effect_parameters.tsv` holds, for each effect type, the slot number, name, value range and the range shown on the instrument. Slots a type does not use are hidden, and edits outside a slot's range are rejected.

- Values are the numbers in the Value column. The Range column (for example `0.3s – 30.0s`) is shown next to them; the step tables it refers to (Tbl No.) are not modelled, so in-between values are not converted to seconds or hertz.
- Where the Effect Parameter List says "Same as the parameters shaded in gray in CROSS DELAY", the slots take CROSS DELAY's shaded EQ parameters (13–16) in order.
- Range texts that wrap onto a second line in the Data List end in "…".
- Spiralizer P and F, Tempo Spiralizer P and F, Slice, the four Beat Repeat types, VCM Mini Filter and VCM Mini Booster have gaps in their tables in the Data List, so their parameters stay hidden.

After an effect type changes, SynthPilot reads the Performance again, so the editor shows the values the instrument loaded for the new type.

## Listed but not implemented

| Parameter | Reason |
| --- | --- |
| AWM2 and FM-X Filter Type | The data range is `00`–`15` (22 values) but the table names 19 filter types, so the mapping cannot be verified. |
| FM-X Level Scaling Break Point | The Data List does not spell out how `00`–`63` maps onto the note names A-1 to C8. |
| Drum Key parameters | The Drum Voice Key table (page 219) is not modelled yet. |
| Parameters of the effect types listed above as having gaps | The Effect Parameter List leaves some of their slots undefined. |
| Effect Preset Number | The preset numbers depend on the effect type (Effect Preset List), which is not modelled yet. |
| 3-band EQ frequencies and Mid Q | The Data List gives only the end points in Hz and Q, not the value of each step. |
| Part Category Sub (`31 0p 15`) | The data range is `00`–`07` (8 values) with default `00` shown as "--", but the Performance Category List names up to 9 Sub Categories per Main Category, including No Assign, so the numbering cannot be verified. Reading a preset Performance with a known Sub Category (for example CFX Concert, Piano/Acoustic) shows which number the instrument uses. |
| Effect Category and Preset (template) | The instrument picks an effect by Category, then Type, then Preset. SynthPilot groups the types by Category; the presets come from the Effect Preset List, which is not modelled yet. |

## Not modelled yet

The Data List has no Part type parameter, and a Bulk Dump that includes blocks unrelated to a Part is rejected (page 192). SynthPilot therefore never changes a Part's engine on the instrument: it reads the engine from the keyboard.
