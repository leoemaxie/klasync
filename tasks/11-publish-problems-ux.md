# 11 - Publish-blocking problems: jump to the block

**Why.** Publishing is blocked when the draft has problems, but the message is
just "Fix the highlighted problems". With a long draft, finding them is slow.

**Build.**
- A problems list (count, click to scroll to and focus the block) in the editor
  bar, using `validateDoc` (already mirrors the server rules).
- Show the server's validation error list verbatim if it disagrees with the
  client check, with the same jump links where it names a block.

**Done when.** Clicking a problem scrolls to the offending block; a unit test
covers mapping server messages to block ids.

**Start at.** `MaterialEditor.svelte`, `src/lib/materials/doc.ts`.
