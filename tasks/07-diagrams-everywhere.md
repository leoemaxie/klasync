# 07 - Diagrams everywhere, and check a freshly generated map

**Why.** Concept-map figures draw in HTML, PDF, EPUB and DOCX. Markdown and
LaTeX still show the text outline. Also, the figure was verified with a
hand-built diagram because the AI quota ran out; it has not been seen with a
freshly generated concept map.

**Build.**
- LaTeX: emit a TikZ picture (or `\includegraphics` of a generated PNG/PDF).
- Markdown: embed the SVG as a data URI image, or keep the outline and add an
  image link, whichever readers render.
- Generate a real concept map for a lecture and check the layout with many
  nodes (24 cap), long labels and many edges; improve the layout if it is
  unreadable (the circle layout does not scale well).

**Done when.** All six formats show the figure (or a deliberate fallback), and a
real generated map is reviewed in each. Update the README format table.

**Start at.** `backend/src/materials/render/svg.rs`, `latex.rs`, `markdown.rs`.
