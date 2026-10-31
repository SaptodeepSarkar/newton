# Newton

A local, voice-only personal assistant. One ring on screen, no text, no cloud.
It reads your Gmail and Google Calendar, tells you about your day, and remembers
you in a local graph.

**Status: scaffold.** This repository currently contains the ring and its state
machine. There is no model, no Google connection, no memory store and no audio
yet. See the roadmap in the project PRD for the order things arrive in.

## What is here

```
src/            Qt6 app: the orb, its state machine, the QML module
  orbstate.*    five states + one "working" flag, no Qt Quick, testable headless
  Ring.qml      the orb; Theme.qml holds every number; Main.qml is the window
tests/          Qt Test + a headless QML smoke test
core/           Rust crate: the tool-call boundary (not linked in yet)
docs/           PRD, brief, security audit, UI spec, canvases — mirror of the
                Obsidian vault. Read AGENTS.md before editing anything here.
```

## Build

Needs Qt 6.5 or newer (developed against 6.12, CI pins 6.8), CMake 3.24+, a
C++20 compiler, and Ninja.

```sh
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build build
./build/src/newton
```

Space cycles the five states. Escape quits.

## Test

```sh
./scripts/check.sh
```

That is exactly what CI runs: build, `ctest`, `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test`. Two kinds of test:

- `orbstate` — the state machine. Illegal moves are refused without changing
  state or emitting a signal, `busy` only exists while listening, barge-in
  returns to listening, error is reachable from everywhere and leaves one way.
- `qml_smoke` — loads the real QML and walks all five states with no display.
  A broken binding in a state nobody opened yet fails the build instead of
  showing a blank ring.

## CI

GitHub Actions, three platforms, on every push and pull request:

| Job | Linux | macOS | Windows |
|---|---|---|---|
| `qt` | build + ctest | build + ctest | build + ctest |
| `rust` | fmt, clippy, test | fmt, clippy, test | fmt, clippy, test |

Qt comes from `install-qt-action` pinned to 6.8.0, so CI proves the oldest Qt
this is meant to support, not the newest machine in the room.

## The ring

| State | Look |
|---|---|
| booting | thin white arc, spinning |
| standby | thick green ring, inner highlight |
| listening | yellow; thick when idle, dashed and chasing while working |
| speaking | purple, 48 bars driven by amplitude |
| error | red, pulses twice, recovers to standby |

Every number lives in `src/qml/Theme.qml`. The state names in `Theme.qml` and
`OrbState::name()` are a contract, and a test holds them together.

## Licence

MIT for this code. Qt 6 is LGPLv3, so the binary must stay dynamically linked
to Qt and ship its licence text. If you would rather the "free forever" promise
be legally binding than merely stated, GPL-3 is the switch.
