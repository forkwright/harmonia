# Integrations

Client connectivity, third-party ecosystem synchronization, and external service bridges.

## KOSync Reading Progress Sync (KOReader)

Harmonia implements the **KOSync protocol** to synchronize reading progress with KOReader, the dominant open-source e-reader for Android, Kindle (jailbroken), Kobo (custom firmware), and Linux.

### Overview

KOSync is a simple HTTP protocol for syncing ebook reading position across devices. KOReader clients send reading progress (position, device, timestamp) and fetch the latest position from any other device. The server stores one progress record per user per document (last-write-wins).

**Protocol reference:** [KOReader KOSync plugin](https://github.com/koreader/koreader/blob/master/plugins/kosync.koplugin/api.json)

### Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/kosync/users/create` | Register a new KOSync user (username + password) |
| `GET` | `/kosync/users/auth` | Authenticate (headers: `x-auth-user`, `x-auth-key`) |
| `PUT` | `/kosync/syncs/progress` | Upload reading progress |
| `GET` | `/kosync/syncs/progress/:document` | Fetch latest progress for a document |

### KOReader Configuration

1. **Install KOReader** on your device(s) (Android, Kindle, Kobo, Linux, or PocketBook).

2. **Open KOReader settings** → **Syncing** → **KOSync** → **Enable**.

3. **Set custom server URL:**
   - Server URL: `http://<harmonia-host>:<port>` (e.g., `http://harmonia.lan:7654`)
   - Username: (your username from step 4)
   - Password: (your password from step 4)

4. **Register user** (one time):
   ```bash
   curl -X POST http://harmonia.lan:7654/kosync/users/create \
     -H "Content-Type: application/json" \
     -d '{"username": "reader1", "password": "yourpassword"}'
   ```
   Or use KOReader's built-in "Register" flow in settings.

5. **Test sync:**
   - Open an ebook on Device A, read to position X.
   - KOReader will upload progress to the server.
   - Open the same ebook on Device B.
   - KOReader will download and jump to position X.

### Authentication

KOSync uses SHA1-hashed passwords in HTTP headers:

- `x-auth-user`: username
- `x-auth-key`: SHA1(password) as a 40-character hex string

Example:
```bash
PASSWORD="mypassword"
SHA1=$(echo -n "$PASSWORD" | sha1sum | cut -d' ' -f1)
curl -H "x-auth-user: reader1" \
     -H "x-auth-key: $SHA1" \
     http://harmonia.lan:7654/kosync/syncs/progress/5d41402abc4b2a76b9719d911017c592
```

### Data Model

Harmonia stores progress at the per-user-per-document level:

| Field | Format | Example |
|-------|--------|---------|
| `document` | MD5 hex hash of file content (32 chars) | `5d41402abc4b2a76b9719d911017c592` |
| `progress` | XPointer (KOReader position string) | `/body/DocFragment[20]/body/p[22]` |
| `percentage` | Float 0.0–1.0 | `0.35` |
| `device` | Device model name | `Kindle Paperwhite` |
| `device_id` | Unique device identifier | `kindle-12345abcde` |
| `timestamp` | ISO8601 server timestamp | `2026-04-22T15:30:00Z` |

### Conflict Resolution

When multiple devices write to the same document simultaneously:

- **Last-write-wins**: The most recent upload (by server timestamp) is returned on `GET`.
- On `GET /syncs/progress/:document`, the server returns a single record for that user+document pair.

### Limitations (v1)

- **Thorium Reader** is not supported - Thorium stores positions locally and does not expose a server-side sync protocol (as of 2026-04).
- **OPDS Position Sync** is not implemented (proposal remains a 2019 draft).
- **Bookmarks, highlights, annotations** are not synced (v2 feature).

Future versions may add support for these via separate endpoints.
