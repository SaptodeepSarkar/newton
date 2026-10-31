# shadcn Catalog Fit

Checked the `@shadcn` registry (61 UI items) against the Newton orb: the green standby ring, the yellow listening ring, the dotted/crossed processing ring, the purple speaking visualizer, the red error circle.

Read with [[PRD]] §4–5 and the renders in this folder. Conclusion up front: **the catalog has no orb, and it should not.** What is worth taking is a handful of proportions, states and motion tokens, hand-built in QML. Everything else in shadcn is React + Tailwind + Radix and does not survive the trip to Qt.

## What the catalog actually contains

- 61 `registry:ui` items. Closest to a ring: `spinner`, `progress`, `skeleton`. There is no audio visualizer, no glow, no pulse, no orb. Searches for `audio wave visualizer` and `glow pulse orb` return nothing.
- `spinner` is a stroked arc (a lucide loader icon) rotated by CSS `animate-spin`. That is the same geometry as the Newton booting ring.
- `progress` is a track plus a translated indicator bar. That is the same geometry as a determinate ring, unrolled.
- Everything else (`button`, `dialog`, `switch`, `badge`, `sonner`, `tooltip`, `empty`) is a text UI. Newton's ring has no text, so most of the catalog is for the two screens where text is unavoidable: first-run setup and settings.

## The four things worth taking

| shadcn item | Why it fits Newton | What to take | QML target |
|---|---|---|---|
| `spinner` | An arc with a round cap, spinning — the booting ring and the "working" ring | Arc geometry, round cap, constant angular speed, no easing | `PathAngleArc` + `RotationAnimation` |
| `progress` | The cut/crossed processing ring is a determinate arc, not a spinner | Track-vs-indicator contrast: faint full circle behind, bright arc in front | two `PathAngleArc` in one `Shape` |
| `sonner` (toast) | The only error language available in a text-free UI | The rule that an error is short, one at a time, and auto-dismisses — translated to a red ring that pulses twice and returns to green | red state + `SequentialAnimation` |
| `switch` / `badge` | Per-source Google consent (calendar, mail) and "2 sources connected" | Toggle semantics and the connected/disabled look | `QtQuick.Controls` `Switch`, token colours |

Honourable mentions, all marginal: `skeleton` (shimmer = the loading pulse), `avatar` (a future contact view), `empty` (the no-data state), `tooltip` (the only hover affordance a ring can have).

## Why nothing else ports

- shadcn ships JSX, Tailwind class strings, Radix primitives and `class-variance-authority`. None of that has a Qt equivalent. The *code* converts to nothing; only the *design* converts.
- Reusing it would mean embedding a web view in a Qt app. That adds a browser runtime to a product whose whole promise is 250 MB idle RAM and full local control. Vetoed — see [[Security Audit]] finding 8, the privacy test would fail on the first webview telemetry call.

## The conversion map (CSS → Qt Quick)

| What the browser does | Qt 6 equivalent | Note |
|---|---|---|
| `border-radius: 50%` ring | `Rectangle { radius: width/2; border.width; border.color; color: "transparent" }` | the plain standby ring; same trick as `spinner`'s border |
| `border-top-color: transparent` (spinner gap) | `Shape` + `PathAngleArc { sweepAngle: 270 }` | the gap is a property, not a border hack |
| `stroke-dasharray` (dotted ring) | `ShapePath { strokeStyle: DashLine; dashPattern: [1, 3]; capStyle: RoundCap }` | the dotted processing ring |
| two-stop gradient ring | `ConicalGradient` as the `Shape` fill, or `LinearGradient` inside a masked `Rectangle` | the "two greens" of standby |
| `box-shadow` / glow | `MultiEffect { blurEnabled; shadowEnabled; shadowBlur; shadowColor }` (Qt 6.5+) | `QtGraphicalEffects` is Qt5-only; most tutorials will send you to a dead module |
| `transition: 200ms cubic-bezier(.4,0,.2,1)` | `Behavior on … { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }` | the shadcn feel, kept |
| `animate-spin` | `RotationAnimation { loops: Animation.Infinite }` | linear, never eased — a spinner that eases looks broken |
| audio visualizer bars | `Repeater` of segments driven by one amplitude property, or one `Canvas` | 24–48 segments; no particle system needed |

## Proportions that differ on purpose

shadcn's spinner is thin: roughly a sixteenth of its diameter, because it sits next to 14px text. The Newton renders are heavy: roughly a fifth, because the ring is the whole interface and has to read from across a desk. Take the geometry and the motion; do not take the stroke ratio. Eyeballed from the renders in this folder, not measured.

## What the orb must add that no catalog has

- **Five states with one shape** — colour, thickness, dash and rotation are the only variables.
- **Amplitude-driven ring** — the purple visualizer is audio level, not a timer. It needs one `level` property from the audio layer, 24–48 segments around the circumference.
- **Barge-in feedback** — the ring must visibly flinch when the user speaks over it.
- **A mute affordance** — a text-free UI has no room for a mic icon, so the ring itself has to carry "mic off" (a slit, not just a colour change).

## Where shadcn is genuinely useful

Two screens will have text no matter how pure the ring is: **first-run** (wake word, Google sign-in, source selection) and **settings** (which sources may be read, forget-a-fact, mute). Use the shadcn *look* there — `dialog`, `switch`, `badge`, `sonner` for errors, `empty` for no data — so those two screens feel like the same product. They are ordinary Qt widgets in a sheet, styled to the same tokens. No web layer.

## Decision

Build the orb in QML by hand. Take from the catalog: the spinner arc, the progress track/indicator contrast, sonner's one-error-at-a-time rule, and switch semantics. Do not add shadcn, React or a web view to a Qt product.

Owed: a `Theme.qml` token singleton (colours per state, stroke widths, two motion durations) before the first ring is drawn, so the five states stay one shape.

Checked 2026-10-31. Catalog: `@shadcn`, 61 UI items, queried via the shadcn MCP.
