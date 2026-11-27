# 05 - Verify email attachments with a real provider

**Why.** Lecture emails now carry the handout as PDF/DOCX, but it was only
unit-checked: the dev stack has no Resend key.

**Build.**
- Send a real message with an attachment to a test inbox and confirm the file
  opens. Check the 8 MB cap against Resend's limits.
- Mention the attachment in the email text/HTML ("A copy is attached") only when
  one is attached (`build_email` in `backend/src/materials/delivery.rs`).
- Decide what happens when rendering the PDF is slow: the delivery loop renders
  once before sending, so check it does not delay the first email noticeably.

**Done when.** A real test email arrives with a working PDF, and the text matches
whether or not a file is attached. Add a note to the materials README.
