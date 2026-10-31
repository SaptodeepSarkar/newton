## 1. Product Overview

- **Product Name:** Newton
- **Product Type:** Assistant
- **Status:** Planning — v1 scoped 2026-10-31
- **Description:** And Audio to Audio PA and day organizer. Can edit and describe your day calendar, mails, keep news and summary ready.
- **Vision:** And Assistant that everyone like to talk and is helpful in what it does.
## 2. Problem Statement

- Every agent out there is Text based and needs MCP's, Skills and setups to become a daily driver, newton is personalized, Assistant that requires few clicks and will connect to all you daily tools to keep you updated.
- All the non-tech people have no Assistant that is Voice based, have a good UI, feels like and assistant and is free; At newton we like to bring that in.
- All existing Agents are either Text based and in that too they have to pay for Model cost eventually. And voice ones are upfront paid.
- Helps users get glance in there day to day life.

## 3. Goals & Objectives

### Primary Goals

-  Connect to tools like, Gmail, Google calendar;(google services).
- Should be local and free always.
- Data says on you device and connected google account.
- The on-boarding should be easy and no google API or console edits.
### Non-Goals

The project is not trying to access there whole life on google. The user can always select what the model and the data is used from google.

- **Not in v1:** writing to Google (creating events, sending or archiving mail). v1 is read-only.
- **Not in v1:** running on cloud models, or any Newton-operated server.
- **Not in v1:** company or customer data in the memory graph.

## 4. Core Features & Requirements
### Feature 1: Sleek UI

- **Description:** The UI is the main wall that will be the thing that users will be interacting with.
- **Purpose:** There will be no text in this assistant system, the assistant just uses its voice and UI indications to show what its is doiing right now.
### **Functional Requirements:**
- **Expected Behavior:** [[UI Flow.canvas]]
- **Edge Cases:** An Red circle in case of error.

**States.** Exactly one ring is visible at a time. No text, no chat log, no buttons, no counts.

| State | Look | Meaning |
|---|---|---|
| Booting | short white arc with a large halo on a dark puck (`#FBFAF8`) | app starting, model not loaded yet |
| StandBy | thick turquoise ring near the rim, two-tone mint → cyan (`#44F7F4`), big halo | idle, mic open, wake word armed |
| Listening | inset pale-chartreuse ring (`#F1F59C`); once working, it becomes 7–10 rotating blocks | heard the wake word / working on the request |
| Speaking | inset indigo ring (`#776EEC`); stroke width follows amplitude, up to `#9893F5` at full volume | streaming its own voice |
| Error | red circle, still undefined — no render and no canvas node exist | could not hear, could not reach Google, or no data |

Measured from the renders, with the behaviour taken from [[UI Flow.canvas]]: [[Visual Spec]]. The visualizer is the ring thickening, not bars beside it.

- The ring must change to Listening within 100 ms of the wake word firing, before any work exists. Affirmation is part of the feature, not decoration.
- No state may be ambiguous: since there is no text, every failure has to be distinguishable by color and shape alone.
- The ring is always the same size and position; only color, thickness and motion change.
- Reference renders: [[Loading.png]] [[Loading Processing Dotted Circle.png]] [[StandBy.png]] [[Listening.png]] [[Listening Dotted.png]] [[Speaking.png]] [[Speaking Visualise Thick.png]]

### Feature 2: Memory Graph

- **Description:** A detailed "Obsidian" Like graph View of the users life saved and ranked.
- **Purpose:** It is never uploaded or reveled anywhere, it is to retrieve information relevant for the user.
### **Functional Requirements:**
- **Expected Behavior:** [[Memory Graph Creation.canvas]]
- **Edge Cases:** Direct response from the agent, tell the user that it have lost access to the personal and google data, and its rules stop it from assuming things.

- Two roots: **Google data** (calendar, meet, drive, other) and **personal facts** (meetings, family, friends, favorites, location history, notes).
- Every fact node records where it came from and when. A fact without a source is not inserted.
- Retrieval is hybrid — keyword, semantic and cosine — then ranked. The top five chunks are what the model sees. Nothing else is put in context.
- Facts learned from the user's own speech are stored. Facts parsed out of mail or calendar content are not stored as memory (see [[Security Audit]] finding 4).
- The graph is the only copy of what the assistant knows. Losing it means starting over, so it must be written atomically.
- If Google access is revoked, the assistant says so out loud and does not invent an answer. No guessing is a rule, not a fallback.

### Feature 3: Voice Loop

- **Description:** Always-on wake word, then one audio-to-audio model turn.
- **Purpose:** Make the interaction hands-free and instant, with no pipeline to assemble.

**Functional Requirements:**
- Wake word is detected locally, always, from a bounded in-memory buffer. Nothing is written to disk and nothing is sent anywhere.
- After the wake word, audio goes to **LFM3-audio** through llama.cpp. The model handles speech-in, reasoning and speech-out in one pass; there is no separate STT, LLM or TTS step.
- Voice output starts streaming before the sentence is finished.
- Barge-in: speaking over the assistant stops it and returns to Listening.
- Tool calls the model emits are constrained output (a fixed schema), never free text parsed by hand.

### Feature 4: Google Connection (read-only)

- **Description:** One sign-in that gives Newton read access to the user's Gmail and Calendar.
- **Purpose:** The whole value is the day: what is on the calendar, what arrived in mail, what needs attention.

**Functional Requirements:**
- One "Sign in with Google" action. No API console, no client ID pasting, no project setup by the user.
- Scopes requested are read-only, and requested incrementally: calendar first, mail only when the user asks for a mail feature.
- The user can revoke access; Newton then reports lost access and keeps working without Google.
- The user chooses which Google sources Newton may read. Unselected sources are never fetched.
- Tokens live in the OS keychain, never in the `.conf` file.
- Loopback redirect with PKCE, bound to 127.0.0.1 on a random port.

### Feature 5: Harness & Context Management

- **Description:** The part that keeps a small model's context small and a low-RAM machine responsive.
- **Purpose:** Local models drown in context and in system stats. The harness decides what the model needs and fetches only that.

**Functional Requirements:**
- System facts (RAM, VRAM, CPU, time, disk, open apps, open sites, notifications) are fetched on demand through a mother helper that drives child helpers. No polling.
- Helpers are offloaded when idle and reloaded on request.
- Context is assembled per turn from the top-ranked memory chunks plus the current tool result. It is not appended to forever.
- Every turn has a context ceiling; when it is hit, the harness summarizes or drops, never truncates mid-fact.
- Preset tool calls only. The model may not construct new tools, run shell commands, or read arbitrary files.

## 5. User Experience (UX/UI)
[[UI Flow.canvas]]
[[Loading.png]] [[Loading Processing Dotted Circle.png]]
[[StandBy.png]] [[Speaking.png]]
A black circle just like Samsung watch or Pixel watch ring UI and indication. The Assistant is made to respond in seconds just like humans. Asking a question will immediately give affirmation that it is doing the task.

**The first five minutes.** Install → ring boots white → green → "Say Newton to begin" is spoken, not shown → Google sign-in in the OS browser → back to green. Target: first useful answer inside five minutes, with no config file opened and no console visited.

**Component language:** the ring is hand-built in QML. The only outside reference is the shadcn catalog, for arc geometry, state proportions and motion tokens — see [[shadcn Catalog Fit]]. No web layer, no React.
- **Behaviour:** five states, and only four of them transition in the canvas: Booting → StandBy → Listening → Speaking → back to StandBy. Listening has two phases (waiting, then working), Speaking has two amplitudes (quiet, loud). Measured colours and the full behaviour model: [[Visual Spec]].

## 6. Technical Requirements

- **Frontend:** Qml, C++, The UI will be written in qml and will not use quickshell it will use standalone QT 6 so that the project can also work on windows.
- **Backend:** Rust, .conf, C++
- **Database:** Graph like
- **Deployment:** Standalone compiled binary for all platforms. (Installer)
- **Platform Support:** Windows, Linux and MacOS

Refinements from the 2026-10-31 scoping pass:

- **UI layer makes no network calls and never sees a token.** It renders state and owns audio devices. That is all.
- **Rust owns the turn loop, tool dispatch, the Google connector and the memory store.** C++ owns the orb and audio capture/playback.
- **Model:** LFM3-audio, GGUF, bundled with the app, loaded through llama.cpp on the GPU and kept resident. System RAM stays low because the weights live in VRAM.
- **Config (`.conf`)** holds preferences only — sources selected, wake word, device ids. Never secrets.
- **Memory store:** embedded graph database plus an embedding index, both local files. Encrypted at rest, key in the OS keychain.
- **No Quickshell, no Electron, no browser runtime.** One Qt6 binary plus helpers.

## 7. System Architecture

**Components**

1. **Orb UI** — QML/C++/Qt6. State machine renderer, audio device I/O, 60 fps ring animation.
2. **Wake detector** — tiny always-on model over a bounded PCM ring buffer. Runs independently of the main model because the main model cannot stay loaded on a low-RAM machine.
3. **Inference runtime** — llama.cpp + LFM3-audio. Streaming audio in, streaming audio out. One load per session, resident in VRAM.
4. **Harness** — Rust. Turn state machine, context budget, tool-call validation, helper dispatch.
5. **Mother helper + child helpers** — separate processes for system facts, offloaded when idle.
6. **Google connector** — Rust. OAuth loopback flow, incremental read-only scopes, token refresh, keychain storage.
7. **Memory store** — graph + embeddings + hybrid ranker, encrypted file.

**Communication**

- UI ↔ harness: in-process FFI for state and audio frames.
- Harness ↔ helpers: local IPC only (unix socket on Linux/macOS, named pipe on Windows), authenticated by peer identity, strict schema.
- Harness ↔ Google: HTTPS to Google hosts only. No Newton endpoint exists.
- Harness ↔ memory: in-process, plus one writer lock so two app instances cannot corrupt the graph.

**Data flow**

```
wake word → yellow ring → LFM3-audio (audio in)
                              ↓ tool call (constrained)
                    harness → memory rank / Google read / helper
                              ↓ top-5 chunks + result injected
                        LFM3-audio (audio out) → purple ring
```

**External dependencies**

- Google Identity (OAuth) and Google Calendar/Gmail APIs. The only network dependency.
- llama.cpp (pinned), Qt6 (pinned), one embedded graph database, one embedding model.

**Module boundaries**

- The UI cannot read memory, hold tokens, or open a socket.
- The Google connector cannot write memory and cannot talk to the model.
- The memory store cannot write to Google.
- The model's only output channel is a validated tool call or speech.

## 8. Performance Requirements

Targets are for the reference machine: any GPU with 8 GB VRAM, 8 GB system RAM, SSD. "First audio" means from end of the user's speech to the first audible word.

- **Startup:** install → green ring ≤ 3 s. The model loads on the first turn, not at boot, so standby is cheap.
- **Memory:** idle resident set ≤ 250 MB including the wake detector; ≤ 700 MB system RAM during a turn, with weights in VRAM. No growth over a long session.
- **CPU:** wake detector ≤ 2% of one core at idle. Nothing else runs while green.
- **GPU:** full power during a turn is acceptable and expected; ≤ 5% at idle.
- **Latency:** first audio ≤ 1.5 s (target), p95 ≤ 3 s. Ring transitions ≤ 100 ms.
- **Google reads:** calendar ≤ 1 s, seven days of mail summarized ≤ 3 s, both cached per turn so a follow-up question costs nothing.
- **Memory:** query ≤ 100 ms at 10k facts, ≤ 250 ms at 100k.
- **Network:** zero requests outside the Google allowlist. No polling, no heartbeats, no update pings without a user action.
- **Scalability:** one user, one machine, one instance. No server-side concerns by design.

## 9. Security & Privacy

Full design review and findings: [[Security Audit]]. Requirements that must hold in code:

- **Authentication:** Google OAuth 2.0 with PKCE; loopback redirect on 127.0.0.1; state parameter verified; single-use.
- **Data storage:** tokens in the OS keychain (Keychain / DPAPI / libsecret). Memory graph encrypted at rest. Config holds no secrets.
- **Retention:** audio is discarded after the turn. Nothing is written to disk by default. Memory facts keep a source and a date, and the user can delete them.
- **Encryption:** memory graph at rest; TLS for Google. No custom crypto.
- **Permission management:** Google sources are opt-in individually; microphone permission requested once with an honest explanation; revoking Google degrades to no-Google mode instead of breaking.
- **Privacy:** hard line — no cloud model, no telemetry, no crash reporter that uploads, no Newton-operated server. The only outbound host set is Google's.
- **Threat model:** covered in [[Security Audit]] — always-on mic buffers, prompt injection from mail content, local process hijacking of the OAuth redirect and helper IPC, and third-party data in the graph.

## 10. Constraints & Limitations

- **Google app verification is the schedule risk.** Gmail read scopes are Restricted. Public use needs app verification plus an annual third-party security assessment; unverified apps are capped at 100 test users and their refresh tokens expire after seven days. Verify current policy before committing. Calendar read is Sensitive, which is lighter but still needs verification.
- **The OAuth client secret ships inside the binary and is extractable.** That is normal for installed apps and accepted. Do not "fix" it later with obfuscation; document it.
- **Model redistribution.** Bundling LFM3-audio requires a licence that allows shipping the weights in a free app. If it does not, the installer cannot ship them and first-run download re-introduces a network dependency and a hosting cost.
- **Hardware:** the latency target assumes a GPU with ~8 GB VRAM. CPU-only machines are slower than the product promise; support for them is an open question.
- **Always-on microphone** needs OS permission on macOS (TCC) and Windows, and on Linux it depends on the desktop. It also needs a written buffer policy — see the audit.
- **Budget:** zero. No server, no relay, no hosted OAuth, no paid model, no telemetry service. Every design choice must cost nothing to operate.
- **Windows and macOS** need code signing and notarisation, which cost money and are a prerequisite for users to trust a binary that reads their mail.
- **No text UI** means ambiguity cannot be explained in words. Error states must be unmistakable by shape and colour alone.
- **Linux secret storage** is not guaranteed; libsecret may be absent and a fallback policy is required.
- **Known limitation:** one Google account per install in v1.

## 11. Development Roadmap

### Phase 1: Foundation

- Repo, Qt6 app shell, the five ring states with real animation.
- llama.cpp + LFM3-audio load, audio in, audio out, one turn end to end.
- Decide the three blockers first: Google verification path, model licence, microphone buffer policy.

### Phase 2: Core Functionality

- Wake detector and the green → yellow → purple loop with barge-in.
- Harness turn loop, context ceiling, constrained tool calls.
- Google connector: loopback OAuth, keychain storage, calendar read, seven-day mail read.
- Memory store: graph, embeddings, hybrid ranking, top-5 injection.
- Mother/child helpers for on-demand system facts.

### Phase 3: Advanced Features

- Encrypted memory at rest, fact deletion and export.
- Write access to Google (create event, draft mail) behind explicit confirmation.
- Preset PC actions from the harness.
- Packaging: signed and notarised installers for all three platforms.

### Phase 4: Testing & Optimization

- Golden set of spoken requests on the reference machine; measure intent accuracy and first-audio latency.
- Traffic capture test proving the host allowlist.
- Idle RAM/CPU soak over 8 hours.
- Wake-word false-accept tuning on real ambient audio.

## 12. Testing & Acceptance Criteria

A feature is complete only when its functional requirements are satisfied, expected behaviour matches this spec, edge cases are handled, no critical regressions are introduced, and the relevant tests pass.

Specific gates:

- **Orb:** all five states reachable and visually distinct; transition to Listening ≤ 100 ms; 60 fps on the reference machine; error state reachable by pulling the network and by muting the mic.
- **Voice loop:** ≥ 90% of 50 spoken requests produce a correct spoken answer on the reference machine; first audio p50 ≤ 1.5 s, p95 ≤ 3 s; barge-in stops audio within 200 ms.
- **Wake word:** ≤ 1 false accept per 8 hours of ambient audio; ≥ 95% recall at 1 metre; no audio written to disk (verified by watching the data dir during a session).
- **Memory:** 20 known facts asked three different ways each — ≥ 90% appear in the top five injected chunks; deleting a fact removes it from answers; a fact never appears without a source.
- **Google:** revoked token produces a spoken "I lost access", never a guess; unselected sources are never requested; no token appears in any log or in the `.conf`.
- **Privacy:** a full day of use captured at the socket level shows no host outside the allowlist. This test is the product promise; it must be automated.
- **Resilience:** no network, no mic, no GPU, and Google down each produce a red ring and recovery, never a crash or a hang.

## 13. Success Metrics

- ≥ 40% of installs still active on day 7.
- Median ≥ 3 turns per active day.
- "What?" / repeat rate under 10% of turns — a proxy for understanding.
- Crash-free sessions ≥ 99%.
- First audio p50 ≤ 1.5 s, p95 ≤ 3 s on the reference machine.
- Idle system RAM ≤ 250 MB; no growth over 8 hours.
- Zero outbound hosts outside the Google allowlist, verified per release.
- Setup: install to first useful answer ≤ 5 minutes, ≤ 3 clicks after sign-in.

## 14. Open Questions & Future Considerations

- **Read-only vs the description.** Section 1 says Newton "can edit and describe your day calendar, mails". v1 is read-only. Confirm that editing is v2 and update section 1 so the doc does not promise what it will not do.
- **One model or two?** [[Working Flow.canvas]] routes speech through "a bigger model to see if the user wants any information", then injects the result back into LFM3. The 2026-10-31 decision says one audio-to-audio model does the whole turn. These cannot both be true. Either the intent router is a second bundled model (and the RAM budget changes), or the canvas is describing a design that was dropped.
- **Text output.** The same canvas says LFM3 "outputs text and speech at the same time", while Feature 1 says the product has no text anywhere. Text may be a log or a debug channel, but the PRD should say so.
- **Barge-in.** The canvas only draws Speaking → StandBy. Talking over the assistant is in the code, not in the design. Keep it or drop it.
- **The error state.** Specified as "a red circle" in prose only: no render, no canvas node, no exit edge. It needs a look before it needs code.
- **Ring size per state.** Standby sits near the puck rim, active states are inset. Is that one ring that resizes, or three rings?
- **The graph view.** Feature 2 calls for a visible "Obsidian-like graph view", but Feature 1 says the UI has no text and only a ring. Is the graph a separate screen, a debug tool, or internal only? Today it is internal.
- **Wake word choice.** Which phrase, and what does a false accept cost? Every false accept reads Google data and speaks.
- **CPU-only support.** Yes or no? If yes, the latency target needs a second, slower number.
- **Model licence.** Can the weights be redistributed in a free installer? This decides Phase 1.
- **Who owns the OAuth project.** A Google Cloud project with a privacy policy URL, verified domain and annual assessment is required for public Gmail access. Who pays and who maintains it?
- **Mail window.** Whole inbox or seven days? Scope, quota, context size and privacy all pull in different directions.
- **Memory encryption key on Linux.** Where does it live when libsecret is missing?
- **Multi-account.** One Google account per install in v1; add later?
- **Accents.** The wake word must work for Indian English, not just US English.
- **Update channel.** A local-only, no-server app still needs a way to ship fixes. Manual download is the honest answer; anything else needs a host.

## 15. References

- Design: [[UI Flow.canvas]] · [[Memory Graph Creation.canvas]] · [[Working Flow.canvas]] · [[shadcn Catalog Fit]]
- Renders: [[Loading.png]] · [[StandBy.png]] · [[Listening.png]] · [[Listening Dotted.png]] · [[Loading Processing Dotted Circle.png]] · [[Speaking.png]] · [[Speaking Visualise Thick.png]]
- Runtime notes: [[Newton Harness]]
- This project: [[Project Brief]] · [[Security Audit]]
- Related research in this vault: [[Vaani local STT-LLM fit]] · [[FINAL-kyuma-decision-model]] (constrained decoding and small-model tool calling)
- Repositories: none yet. `/home/saptodeep/Projects/Newton` is empty.

Checked 2026-10-31.
