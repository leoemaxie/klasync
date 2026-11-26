# 01 - Native LMS connectors (Canvas, Moodle)

**Why.** The `webhook` delivery channel posts a signed JSON message, but a
lecturer still needs a connector on the other side to turn it into a Canvas
page or Moodle resource. Most universities will not run that glue themselves.

**Build.**
- A `lms` plug-in kind (or delivery channels `canvas`, `moodle`) that calls the
  LMS API directly: create/update a page or file in the course that matches this
  offering.
- Per-course config: base URL, course id in the LMS, credential. Store tokens
  encrypted (see how other secrets are handled; do not store them in plain
  `material_plugin_settings.config`).
- Idempotent: re-publishing updates the same LMS object instead of creating a
  duplicate (remember the LMS object id per material).
- Settings UI next to the webhook fields in `MaterialSettingsPanel.svelte`.

**Done when.**
- Publishing a lecture creates/updates a Canvas page and a Moodle resource in a
  test instance, and re-publishing updates it.
- A bad token or missing course shows a clear error on the delivery row.
- Unit tests with a mocked LMS HTTP server; no secret appears in API responses.

**Start at.** `backend/src/materials/lms.rs` (webhook channel, SSRF checks to
reuse), `backend/src/materials/registry.rs` (`DeliveryChannel`).
