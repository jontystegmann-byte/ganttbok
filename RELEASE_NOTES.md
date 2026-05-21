Blik Plan v1.4.0 — Chaser feature

NEW
- **Contacts page**. Click the people icon in the sidebar footer to add and manage contacts (name, Telegram chat_id, handle, notes). Each contact gets a per-row "Send test ping" button to verify the setup.
- **Telegram-bot chasers**. Assign a contact to any task. Three one-click chaser templates: *Manual update*, *Deadline approaching*, *Behind schedule*. Plus a free-text *Custom message* option. Editable templates with `{task}`, `{days}`, `{contact_name}`, `{job_name}` placeholders.
- **Auto-nudges** fire on app launch and on window focus (debounced to once every 5 min). For each assigned task whose end date is within the threshold (default 3 days), the bot sends a "deadline approaching" message. For each overdue task, it sends "behind schedule". A 24-hour throttle prevents double-sends.
- **Settings → Chaser** section: bot token, test chat_id + Test button, threshold slider (1–14 days), auto-nudge toggle, 3 editable message templates.

SETUP
- In Telegram, search for `@BotFather` → `/newbot` → follow prompts → copy the token.
- Paste the token into Settings → Chaser → Bot token.
- For each contact: have them search for `@userinfobot` in Telegram and read off the numeric ID. Add a contact and paste it as the Telegram chat_id.
- Each contact must also send `/start` to your bot once so the bot has permission to message them.
- Assign contacts to tasks via the new "Assigned to" picker in the task details panel.

UNDER THE HOOD
- DB schema bumped to v6: new `contact` table; `task` gains `contact_id` (nullable FK with `ON DELETE SET NULL`) and `last_chaser_sent_at` for 24h throttling.
- Bot token + threshold + template strings stored in app meta. Token is plain-text local-only (your DB lives in `~/Library/Application Support/Gantt Bok/`).
- New `reqwest` dep (rustls-tls + blocking + json) for the Telegram API call.
