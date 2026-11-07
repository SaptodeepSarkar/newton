# Features.
## Context management
Working with ollama and local models is a tough task when dealing with low VRAM and system RAM, Newton harness must all times manage context check whats important for the Model to know to give the answer and provide it with that information while keeping the context usage low.
### How it does it
Newton will build a graph based "Obsidian" like memory system about the user and will use ranking system to rank documents chunks of information based on Keyword search and Semantic search.

### Managing system information
System information like RAM usage, VRAM usage, CPU usage, System Time, Storage left on drives, apps open, website open, Notifications, metrics, etc. Are live data that keeps changing and needs to be updated constantly.

Newton will use a main helper as a command processes and then use child helper to provide data on demand. Like if the model whats to know the notification the Harness dispatches a request to the mother helper and then the mother helper user its Child notification helper to fetch notification data.

This on demand fetching makes the ram usage low and stops from polling.

## Execution
Newton can execute preset tool calls that can do basic task on the PC.
# Wake word.
The turn does not start on any sound. Before a turn is opened, the utterance is compared with the templates recorded from the owner's voice: log energies at 20 mel-spaced bands, compared by dynamic time warping (`src/audio/WakeModel.cpp`). Nothing is transcribed to decide this, and nothing leaves the machine.

- Enrol: `newton --enroll-wake 3`, say the word three times, and the threshold is set from how closely the three takes agree (`~/.local/share/newton/Newton/wake.model`).
- Not enrolled: any speech opens a turn, so the product is testable straight out of the box.
- Enrolled: a low score returns the orb to standby without speaking and without counting a turn.

Checked 2026-11-07.
