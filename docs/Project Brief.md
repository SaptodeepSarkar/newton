# Project Brief

A one-page understanding of Newton. Source of truth is [[PRD]], with the runtime details in [[Newton Harness]] and the risk list in [[Security Audit]].

## What it is

A local, voice-only personal assistant. It shows no text: a single ring on the screen tells you what it is doing, and it talks back. It reads your Gmail and Google Calendar, tells you about your day, and remembers what it learns about you in a local graph.

Three claims define the product:

1. **Nothing leaves the device.** No Newton server, no cloud model, no telemetry. The only outbound traffic is the user's own Google account.
2. **Free, forever.** No subscription, no per-token cost, because the model runs on the user's own GPU.
3. **No setup.** No API console, no MCP config, no prompt engineering. One installer, one Google sign-in, then talk.

## The four visual states

The whole interface is a ring. No text, no chat history, no buttons.

- **Booting:** a short white arc with a big halo on a dark puck. Not a full ring.
- **StandBy:** thick turquoise ring near the puck rim, mint → cyan (`#44F7F4`). Idle, wake word armed.
- **Listening:** inset pale-chartreuse ring (`#F1F59C`). Once it starts working, the ring breaks into 7–10 rotating blocks.
- **Speaking:** inset indigo ring (`#776EEC`). Stroke width follows the voice, up to `#9893F5` at full volume — the visualizer *is* the ring.
- **Error:** red circle. Only prose so far: no render, no canvas node.

The loop is Booting → StandBy → Listening → Speaking → back to StandBy. See [[Visual Spec]] for the measurements and [[UI Flow.canvas]] for the wiring the design actually draws.

## How one turn works

1. Wake word fires from an always-on mic buffer.
2. The ring goes yellow, so the user gets instant feedback before any result exists.
3. **LFM3-audio** takes the raw audio in and produces speech out — speech-to-text, reasoning and text-to-speech are one model, one pass. There is no STT → LLM → TTS pipeline to assemble.
4. If the answer needs facts, the harness dispatches a tool call (constrained output, not free text) to the Google connector or the memory store.
5. Results are ranked and the best few chunks are injected back into the model's context.
6. Audio streams out while the ring turns purple. Text appears nowhere.

## The three subsystems

- **Orb (Qt6 / QML / C++).** Rendering and audio devices only. Knows no tokens, holds no memory, makes no network calls. Qt6 standalone, not Quickshell, so Windows works.
- **Harness (Rust).** Owns the turn loop, the state machine, the context budget and tool dispatch. Its job is to keep a small model's context clean on a low-RAM machine, so it fetches system data on demand through a mother helper that drives child helpers, instead of polling. Idle helpers are offloaded. See [[Newton Harness]].
- **Connectors and memory (Rust).** Google OAuth (loopback redirect, read-only scopes) and the memory graph. Both are local; neither is reachable from the UI.

## The memory graph

An Obsidian-like graph of the user's life, stored locally, never uploaded. Two roots:

- **Google data:** calendar, meet, drive, other.
- **Public / personal facts:** meetings, family, friends, favorites, location history, notes. The canvas also lists company and customer data — see the audit, that is not v1 material.

Retrieval is hybrid: keyword search, semantic search, cosine similarity, then a ranking pass. The top five chunks go into the model's context. That small window is the point: it is what makes a small local model answer like it knows you.

See [[Memory Graph Creation.canvas]].

## Stack and delivery

| Part | Choice |
|---|---|
| UI | Qt6 + QML + C++, standalone (cross-platform) |
| Core | Rust |
| Config | `.conf` |
| Model | LFM3-audio via llama.cpp, bundled with the app |
| Storage | Graph database, local only |
| Ship | One signed installer for Windows, Linux, macOS |

## Decisions locked 2026-10-31

- v1 is talk + read-only Gmail and Calendar + memory. Writing to Google is v2.
- Model is bundled and runs in-process through llama.cpp. No Ollama, no user API keys.
- Google auth ships one OAuth client and uses a loopback redirect.
- GPU is allowed to be fully used; system RAM must stay low (model lives in VRAM).
- Activation is an always-on wake word, not a hotkey.
- Local-only is a hard line, not a default.
- Latency is defined by the audio-to-audio model: one load, one pass, first words out.

## Where it stands

Planning only. `/home/saptodeep/Projects/Newton` is empty — no code, no repo, no dependencies. There is a PRD with goals and features, a harness note with two ideas, three design canvases, seven renders, and now this brief plus the audit.

The open blockers are not technical style: Google's app-verification rules for Gmail, the licence for redistributing the model, and a defined microphone-buffer policy. All three are cheaper to settle now than after the architecture exists.

Checked 2026-10-31.
