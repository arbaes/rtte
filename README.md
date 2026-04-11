<div align="center"><pre>
     ____  __________________  
   • / __ \/_  __/_  __/ ____/ • 
  • / /_/ / / /   / / / __/   •  
 • / _, _/ / /   / / / /___  •   
• /_/ |_| /_/   /_/ /_____/ •    
</pre></div>

# *R*usted *T*erminal *T*ext *E*ffects
Rust reimplementation of [TerminalTextEffects (TTE)](https://github.com/ChrisBuilds/terminaltexteffects) by [ChrisBuilds](https://github.com/ChrisBuilds).

[![CI](https://github.com/arbaes/rtte/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/arbaes/rtte/actions/workflows/ci.yml)
[![TTE Effect Parity](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/arbaes/edf5217236148a1aa1923659ac1302eb/raw/tte-effect-parity.json)](https://github.com/arbaes/rtte/actions/workflows/effect-parity.yml)
[![Cargo Audit](https://github.com/arbaes/rtte/actions/workflows/security-audit.yml/badge.svg?branch=master)](https://github.com/arbaes/rtte/actions/workflows/security-audit.yml)
---

## Disclaimer

This is my first Rust project, built primarily for my own needs. I will not lie to you, I used AI's assistance a lot in this. This was mostly a fun experiment to see how TTE's effects could be implemented in Rust, and to learn Rust in the process.

**All credit for the original effects, design, and concept goes to [ChrisBuilds](https://github.com/ChrisBuilds/terminaltexteffects) and all [TTE contributors](https://github.com/ChrisBuilds/terminaltexteffects/graphs/contributors).** This project is a reimplementation attempt, not an original work.

The whole project was only tested on my machine (Arch Linux, x86_64), so there may be platform-specific bugs or performance issues. Contributions to fix those are welcome, but I won't be able to personally verify every platform, nor do I plan to maintain this long-term for other than my personal uses. **Use at your own risk**, and expect that it may be a bit rough around the edges.

---

## What is it?

`rtte` takes text from stdin (or a file) and animates it in the terminal using one of many visual effects. It's a drop-in replacement for the `tte` CLI from [TerminalTextEffects](https://github.com/ChrisBuilds/terminaltexteffects), but compiled to a native Rust binary.

The main objective was to get better performance when you're piping output from `toilet` or similar, and use some shader effect in terminals like `cool-retro-term`, on big 4K screens.

```sh
echo "Hello, world!" | rtte wipe
echo "Hello, world!" | toilet -f "DOS Rebel" | rtte --random-effect
```

---

## Effects

<details>
<summary>Show all effects</summary>

| Effect | Description |
|---|---|
| `beams` | Beams of light sweep across the text |
| `binarypath` | Binary data streams converge into characters |
| `blackhole` | Characters spiral into a black hole |
| `bouncyballs` | Characters arrive as bouncing balls |
| `bubbles` | Characters float up as bubbles |
| `burn` | Text burns away from the edges |
| `colorshift` | Gradient color shifts across the text |
| `crumble` | Text crumbles into particles |
| `decrypt` | Cipher animation resolves to plaintext |
| `errorcorrect` | Error-correction reveals the final text |
| `expand` | Characters expand from the center |
| `fireworks` | Characters launch as fireworks |
| `highlight` | A highlight sweeps across the text |
| `laseretch` | A laser etches the text onto the screen |
| `matrix` | Digital rain resolves into the text |
| `middleout` | Text expands outward from the middle |
| `orbittingvolley` | Characters orbit before landing |
| `overflow` | Text overflows and resolves |
| `pour` | Characters pour into place from above |
| `print` | Typewriter-style character-by-character print |
| `rain` | Characters fall like rain |
| `randomsequence` | Characters randomize then resolve |
| `rings` | Concentric rings expand outward |
| `scattered` | Characters scatter then reassemble |
| `slice` | Text is sliced and reassembled |
| `slide` | Characters slide in from the edges |
| `smoke` | Characters drift in as smoke particles |
| `spotlights` | Moving spotlights reveal the text |
| `spray` | Characters spray in from a point |
| `swarm` | Characters swarm before settling |
| `sweep` | A sweep line reveals the text |
| `synthgrid` | Synthwave-style grid animation |
| `thunderstorm` | Lightning strikes reveal the text |
| `unstable` | Glitchy, unstable reveal |
| `vhstape` | VHS tape degradation effect |
| `waves` | Wave motion ripples through the text |
| `wipe` | Diagonal wipe reveals the text |

</details>

---

## Requirements

| Requirement | When |
|---|---|
| [Rust stable](https://rustup.rs/) | Build only |
| `git` | Build only |
| `glibc` | Runtime |

---

## Installation

### Arch Linux

```sh
git clone https://github.com/arbaes/rtte.git
cd rtte
makepkg -si
```


### Manual

```sh
git clone https://github.com/arbaes/rtte.git
cd rtte
cargo build --release
sudo install -Dm755 target/release/rtte /usr/local/bin/rtte
```

---

## Usage

```sh
# Basic usage
echo "Hello" | rtte print

# Pick a random effect
echo "Hello" | rtte --random-effect

# Specify frame rate (default: 60)
echo "Hello" | rtte --frame-rate 30 matrix

# Read from a file
rtte wipe -i mytext.txt

# Random effect, excluding some
echo "Hello" | rtte --random-effect --exclude-effects matrix rain

# Random effect from a specific list
echo "Hello" | rtte --random-effect --include-effects wipe expand slide
```

### With [toilet](https://github.com/cacalabs/toilet) (ASCII art text generator)

```sh
echo "RTTE" | toilet -f "DOS Rebel" | rtte wipe
echo "$(date +%H:%M)" | toilet -f future | rtte --random-effect
```

---

## Tests

```sh
cargo test
```

Tests cover:
- Core engine (grid creation, ANSI stripping)
- Gradient and easing math
- Convergence for all effects (each must complete within a frame budget on a small grid)
- Final state invariants (characters visible, content preserved)

---

## Development

```sh
# Build
cargo build --release

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Test
cargo test
```

---

## License

 [See LICENSE.md](LICENSE).

By respect for the original TerminalTextEffects project's [License](https://github.com/ChrisBuilds/terminaltexteffects/blob/main/LICENSE), which is also MIT licensed, this project is also under the MIT License.

---

## Contributing

Issues and PRs are welcome. If you're fixing a bug or adding a missing feature from TTE, that's the most useful kind of contribution. I don't plan to add new effects or features beyond what TTE has.
