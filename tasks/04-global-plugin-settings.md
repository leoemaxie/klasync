# 04 - Institution-wide plug-in settings

**Why.** `material_plugin_settings` supports `scope = 'global'` and the registry
honours it, but only course overrides have an API and UI. Admins have to edit
rows by hand to turn a renderer or channel off for everyone.

**Build.**
- An admin role/check (find how admins are represented; if there is none, decide
  and document how an admin is identified).
- `GET/PUT/DELETE /admin/materials/plugins/...` mirroring the course routes,
  with the same protection for `in_app` and `html`.
- A small admin screen; the course Settings tab should show "set by institution"
  and, optionally, lock a plug-in so courses cannot override it.

**Done when.**
- A global "off" disables the plug-in in every course that has no override.
- A locked global setting cannot be overridden from a course.
- Tests cover precedence (course > global > default) and the lock.

**Start at.** `backend/src/materials/registry.rs` (`pick_setting`),
`list_plugins`/`put_plugin` in `backend/src/api/handlers/materials.rs`.
