# 10 - Editor accessibility and mobile layout

**Why.** The editor was checked visually on desktop only.

**Build.**
- Keyboard: reorder blocks and sections without a mouse, a visible focus order,
  and a shortcut to jump between blocks.
- Screen readers: announce save state, validation problems and moves
  (live regions), label every control; test with a real screen reader.
- Mobile: the three-column layout collapses, but check toolbars, the add-block
  row and the table editor at 360-430 px; avoid horizontal scroll.
- Contrast and focus rings on the warning/error colours.

**Done when.** An axe run shows no serious issues on the editor, and the main
edit-publish flow works with keyboard only and at phone width.

**Start at.** `src/lib/components/materials/MaterialEditor.svelte`,
`BlockEditor.svelte`.
