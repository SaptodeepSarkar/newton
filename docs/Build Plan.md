# Build Plan

32 phases, eight waves of four. Each phase owns its tests; each wave ends with a review gate that runs the whole suite before the next wave starts. A phase is done when its own test passes, `./scripts/check.sh` is green, and the reviewer has read the diff.

Companion docs: [[PRD]] (what), [[Security Audit]] (constraints), [[Visual Spec]] (look), [[Newton Harness]] (runtime).

## Wave 1 — Foundation

| # | Phase | Test that proves it |
|---|---|---|
| 1 | Audio capture: device list, 16 kHz mono frames, bounded in-memory ring buffer, nothing written to disk | unit: frame size, ring overwrite, buffer never exceeds cap, no file opened |
| 2 | VAD + wake-word harness: energy VAD and a wake state machine, model-agnostic | unit: silence, speech, false-accept cooldown, one wake per utterance |
| 3 | Model manifest + downloader: versioned manifest, SHA-256 per file, resume, offline mode | unit: bad checksum rejected, truncated file retried, offline refuses cleanly |
| 4 | Packaging: CPack for Linux (TGZ/deb), macOS (bundle), Windows (ZIP), CI artifacts | CI: three artifacts produced on three runners |

## Wave 2 — Model runtime

| # | Phase | Test |
|---|---|---|
| 5 | llama.cpp + LFM3-audio load from a local GGUF, pinned version | load test on a fixture model; refuses a corrupt file. **Model fetched and checksummed 2026-11-07** (`tools/models/manifest.json`); the llama.cpp link is the open half. |
| 6 | Streaming audio in / audio out; first-audio latency measured | timing test asserts p50 budget on the reference machine |
| 7 | Constrained decoding wired to the tool-call crate | grammar rejects invented tool; enforcement test |
| 8 | Context budget manager: per-turn ceiling, drop vs summarise | property test on 1k synthetic facts |

## Wave 3 — Memory

| # | Phase | Test |
|---|---|---|
| 9 | Graph store: nodes, edges, atomic write, single writer lock | crash-mid-write test leaves the old file intact |
| 10 | Embedding index, local model only | query returns neighbours in order |
| 11 | Hybrid ranking: keyword + semantic + cosine, top five | 20 known facts, three phrasings each, 90 % hit rate |
| 12 | Provenance, delete, export | a forgotten fact never returns; export round-trips |

## Wave 4 — Google

| # | Phase | Test |
|---|---|---|
| 13 | OAuth loopback + PKCE + keychain, no token on disk | state mismatch rejected; token absent from config |
| 14 | Calendar read | fixture JSON renders a day; empty day is not an error |
| 15 | Gmail read, seven-day window, quota backoff | 429 handled once, not in a loop |
| 16 | Revocation and degraded mode | revoked token speaks, never guesses |

## Wave 5 — Harness

| # | Phase | Test |
|---|---|---|
| 17 | Turn loop: wake → listen → tool → speak, drives the orb | state machine integration test |
| 18 | Mother/child helpers with authenticated IPC | impostor helper refused |
| 19 | Preset tool execution + allowlist | unknown tool cannot run |
| 20 | Error taxonomy → red ring, one reason per failure | every failure path reaches Error and recovers |

## Wave 6 — Experience

| # | Phase | Test |
|---|---|---|
| 21 | Theme aligned to [[Visual Spec]] tokens | token snapshot test |
| 22 | Booting arc, working blocks, amplitude-driven speaking ring | screenshot diff per state |
| 23 | Wake-word UX: mute, visible mic state, no hidden listening | mute stops capture, indicator follows |
| 24 | First-run + settings screens (text allowed here only) | headless QML walk of both screens |

## Wave 7 — Ship

| # | Phase | Test |
|---|---|---|
| 25 | Installer downloads models from a release, not from git | fresh install on a clean VM gets a working orb |
| 26 | Signing and notarisation hooks (keys supplied by the owner, never in CI logs) | unsigned build refuses to publish |
| 27 | Manual update channel, no phone-home | traffic test shows no update ping |
| 28 | Crash handling: local report only, no upload, no audio in it | crash dump contains no PCM |

## Wave 8 — Hardening

| # | Phase | Test |
|---|---|---|
| 29 | Privacy gate: socket-level allowlist in CI | any non-Google host fails the build |
| 30 | RAM and latency soak: 8 hours idle, 500 turns | no growth, p95 within budget |
| 31 | Wake-word tuning on real ambient audio | ≤ 1 false accept per 8 hours. **Enrolled per voice 2026-11-07** (templates + DTW, `src/audio/WakeModel.cpp`); the 8-hour count has not run. |
| 32 | 1.0 checklist: re-run [[Security Audit]], licence texts, release notes | all findings closed or accepted in writing |

## Gates

- **Per phase:** own test green, `./scripts/check.sh` green, diff reviewed.
- **Per wave:** full suite green on three platforms, and the wave's doc updated in the vault and in `docs/`.
- **No wave starts while the previous one is red.** A red wave is the only thing that stops the line.

## Blocked, and by what

| Phase | Blocked by | Why it cannot be faked |
|---|---|---|
| ~~5, 25~~ | ~~Model licence~~ | **Closed 2026-11-07.** The LFM Open License v1.0 (Section 4) allows redistribution and distribution of derivatives; Section 5 allows commercial use below $10M annual revenue. Shipping a model build means shipping `tools/models/LFM-Open-License-1.0.txt` beside it. |
| 6, 30 | Hardware | The latency and RAM targets are for a GPU machine. Measuring them on a CPU box produces a number that is not the product. |
| 13–15 | Google verification | Gmail read is a Restricted scope; without verification the app is capped at 100 users with seven-day tokens. |
| 25, 26 | Signing keys and a Mac | Windows and macOS installers that users will trust need certificates and notarisation. Neither can be generated here. |
| Models in git | GitHub limits | Weights are gigabytes; git is not a CDN. Releases (and LFS above 100 MB per file) is the only workable host. |

Checked 2026-11-07.
