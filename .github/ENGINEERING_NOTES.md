# Engineering Notes

Known issues that are deliberately **not** fixed in the current work, recorded
here so they are not lost. Nothing in this file changes runtime behaviour.

---

## Config writes are not atomic

**Status:** open — intentionally out of scope for the notification-sources work.

`AppConfig::save` writes the config with a plain `fs::write`, and
`AppConfig::load` swallows parse errors and falls back to a fresh default
config.

A crash, power loss, or full disk during a write can therefore leave a
truncated `config.json`. On the next launch the parse fails, `load` returns
`Self::default()`, and every user setting — including the notification source
registry, colours, and the allowlist — is silently replaced by defaults. There
is no error shown and no backup left behind.

This is pre-existing and predates the V2 notification-sources work, which
inherits it unchanged. It was found during the V2 design audit and recorded
rather than fixed, because bundling a persistence change with a feature PR
would make both harder to review.

**Future fix:** write to a temporary file in the same directory, flush and
close it as appropriate, then atomically rename it over `config.json`. The
rename is the atomic step, so a partial write can never replace a good config.
Worth doing as its own PR, together with surfacing a parse failure to the
user rather than silently resetting.

Relevant code: `src/config.rs` — `AppConfig::save` and `AppConfig::load`.
