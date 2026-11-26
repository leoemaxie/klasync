# Materials engine: build status (updated as work lands)

Spec: "Lecture Materials: Design Spec" (Claude doc). Branch: `claude/build-materials-engine`.

The `materials` module is **not yet wired into `lib.rs`**, so the crate still builds
exactly as before. It gets wired once the remaining files below exist and compile.

| Piece | File | State |
| --- | --- | --- |
| Migration (tables, job types, outbox, plug-in settings) | `migrations/0014_lecture_materials.sql` | written, untested against Postgres |
| Block schema, validation, inline markdown | `blocks.rs` | written, unit tests included, not yet compiled |
| Plug-in traits + registry | `registry.rs` | written, not yet compiled |
| DB access | `store.rs` | written, not yet compiled |
| Renderers: HTML, Markdown, DOCX, PDF | `render/*` | written, not yet compiled |
| Generation pipeline (outline, section, assemble) | `generate.rs` | TODO |
| Review/edit/publish service | `service.rs` | TODO |
| Course book | `book.rs` | TODO |
| Delivery channels (in-app, link, email) | `delivery.rs` | TODO |
| Events relay | `events.rs` | TODO |
| Comparison analyzer | `compare.rs` | TODO |
| HTTP API | `api/handlers/materials.rs` | TODO |
| Wiring (lib.rs, router, worker hook, post-lecture trigger) | various | TODO |

Extension points for other work: implement a trait from `registry.rs` and add it to
`Registry::builtin()`. Plug-ins only depend on `blocks.rs` and on events.
