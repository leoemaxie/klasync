# 02 - Webhook hardening: retries, test button, DNS pinning

**Why.** The webhook is one attempt with a 15 s timeout, and the SSRF check
resolves the host separately from the request, so a DNS-rebinding host could
pass the check and then resolve to an internal address.

**Build.**
- Resolve once, then connect to that address (pin it with
  `reqwest::ClientBuilder::resolve`) so the checked IP is the used IP.
- Retry with backoff on network errors and 5xx (for example 3 tries over a few
  minutes), recording each attempt on the delivery row.
- A "Send test" button in the webhook settings that posts a sample payload and
  shows the receiver's status code.
- Document the payload and signature check with a small verification snippet.

**Done when.**
- A test with a host that resolves to a public IP at check time and a private IP
  at connect time is refused.
- A flaky receiver (500 then 200) ends as `sent`; a dead one ends `failed` with
  the last error.
- The test button works and never sends real student data.

**Start at.** `backend/src/materials/lms.rs`, `backend/src/materials/delivery.rs`
(`process`, delivery status), `MaterialSettingsPanel.svelte`.
