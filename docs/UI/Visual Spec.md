# Visual Spec

Source of truth for the orb, measured from the seven renders in this folder on 2026-10-31. [[shadcn Catalog Fit]] covers what to borrow from component libraries; this note covers what the thing must look like.

## What the images are

Seven small crops of one **physical-style puck** sitting on a dark surface, with other pucks out of focus behind it. They are not flat app screenshots:

- The surface is near-black warm grey, `#161412` to `#1F1F1E` depending on the crop.
- The puck is a dark recessed disc with a faint machined rim (a groove, not a highlight).
- The lit element is a ring floating on that face, never touching the edge.
- Because each crop is zoomed differently (47 px to 185 px wide), absolute sizes are not comparable across files. Only **ratios** are.

## Measured renders

| File | State | Lit colour | Outer Ø | Stroke | Stroke / Ø | Shape |
|---|---|---|---|---|---|---|
| `Loading.png` | booting | `#FBFAF8` | 27 px | 9–10 px (arc) | — | one short white arc, heavy soft glow, puck dark |
| `StandBy.png` | standby | `#44F7F4` | 73 px | 16–18 px | ~22 % | thick closed ring, large halo, ring sits near the puck rim |
| `Listening.png` | listening, idle | `#F1F59C` | 25 px | 4–5 px | ~18 % | thin closed ring, inset, generous dark gap to the rim |
| `Listening Dotted.png` | listening, working | `#F5F594` | 26 px | 3 px | ~12 % | 7 distinct rounded blocks (measured above the glow threshold), large gaps |
| `Loading Processing Dotted Circle.png` | working, smaller variant | `#F5F598` | 25 px | 3 px | ~12 % | same block ring, dimmer, tighter crop |
| `Speaking.png` | speaking, quiet | `#776EEC` | 25 px | 3 px | ~12 % | thin closed ring, inset, indigo with a blue halo on the surface |
| `Speaking Visualise Thick.png` | speaking, loud | `#9893F5` | 26 px | 6–7 px | ~25 % | same ring at high amplitude: thicker, brighter, halo tints the surface blue |

## Behaviour, from the canvases

The behaviour lives in [[UI Flow.canvas]], not in the stills. Reading the edges:

```
Booting ──► StandBy ──► Listening ──► Speaking ──┐
   │                                                │
   └────────────────────────────────────────────────┘  (Speaking loops back to StandBy)
```

- **Booting → StandBy:** model ready. One-way.
- **StandBy → Listening:** the wake word, per [[Working Flow.canvas]].
- **Listening → Speaking:** first audio out.
- **Speaking → StandBy:** the edge drawn along the top returns it to rest. Speaking is terminal in the canvas; it does not go back to Listening.
- **Listening has two phases**, and the canvas attaches three renders to it: `Listening.png` (solid, waiting), then `Listening Dotted.png` and `Loading Processing Dotted Circle.png` (blocks, working). So the dotted look is the *second half* of one state, not a sixth state.
- **Speaking has two amplitudes**, and the canvas attaches both renders to it: `Speaking.png` (quiet) and `Speaking Visualise Thick.png` (loud). This is the "visualizer": the ring thickens and brightens with the voice. No bars, no separate widget.

## What the renders contradict, and what the canvas settles

1. **"Two weights" was wrong** — the two speaking renders are one state at two amplitudes. The PRD's "purple ring as visualizer" is correct, and the visualizer is thickness, not bars.
2. **Booting is an arc, not a ring.** A short white arc with a large bloom on a dark face. It reads as "warming up", not as a progress circle.
3. **Standby is cyan-green, not green.** `#44F7F4` is turquoise. "Two gradients of green" describes the two-tone ring; the hue is mint → cyan.
4. **The ring changes size between states.** Standby is thick and near the rim; listening and speaking are thinner and inset with a wide dark margin. It is not one shape recoloured.
5. **"Working" is discrete blocks, not a dashed stroke.** Seven-plus rounded blocks with gaps of two to three times the block length, rotating. A `dashPattern` of many small dashes would be wrong.
6. **No error render exists, and no error node exists in the canvas.** The red circle is PRD text only. Its colour, weight, motion and how it leaves are all open.
7. **No barge-in edge exists.** Speaking only goes to StandBy in the canvas. Talking over the assistant (Speaking → Listening) is an assumption in the current code, not a design decision.

## Tokens to standardise

| Token | Measured | Current `Theme.qml` | Verdict |
|---|---|---|---|
| standby | `#44F7F4` (+ mint core) | `#22E37A` / `#9DF5C0` | wrong hue: code is green, render is cyan |
| listening | `#F1F59C` | `#F2E14C` | code too saturated, render is pale chartreuse |
| speaking | `#776EEC` thin / `#9893F5` loud | `#B07CF0` | code too pink, render is indigo; thickness must follow amplitude |
| booting | `#FBFAF8` arc | `#EDEDED` full ring | code draws the wrong shape |
| error | none | `#E5484D` | invented; needs a render or a decision |
| surface | `#161412`–`#1F1F1E` | `#2B2B2D` | code too light; the render is nearly black |
| stroke / Ø | 12 %–25 % by state | 3–40 px on a 300 px orb | roughly right, unmeasured |

## Geometry rules

- Ring outer sits between roughly 0.3 and 0.55 of the puck face; standby is the largest, active states the smallest. The crops disagree, so this needs one decision, not another measurement.
- Halo is two to three stroke widths across, at 10–20 % opacity, and it tints the surface (visible as the blue cast in `Speaking Visualise Thick.png`).
- Blocks: 7–10 around the circumference, round caps, gaps 2–3× the block.
- Everything is one ring on one disc. No text, no icons, no progress numbers.

## Deltas in the code

`src/Ring.qml` and `src/Theme.qml` currently draw a 48-bar visualizer, a full-ring booting state, and the wrong three hues. Recorded here rather than changed, per instruction to leave existing code alone.

Owed, in order:

1. Speaking: replace the 48 bars with stroke width driven by amplitude, between the two measured weights (3 px and 7 px on a 26 px ring).
2. Take the measured colours in the token table above.
3. Booting: a short white arc with a large halo, not a closed ring.
4. Working: 7–10 blocks with 2–3× gaps, not a `dashPattern` of small dashes.
5. State-dependent ring size: standby near the rim, active states inset.
6. Error: needs a look before it needs code. There is no render and no canvas node.

Checked 2026-10-31. Measured with a pixel pass over the seven PNGs in this folder; ratios only, because the crops differ.
