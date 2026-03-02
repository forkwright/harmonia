# IRC Announce Integration — Zetesis Private Tracker Real-Time Feeds

> Zetesis monitors IRC announce channels to receive real-time release notifications from private trackers — an alternative acquisition path that bypasses search latency.
> Cross-references: [architecture/subsystems.md](../architecture/subsystems.md) (Zetesis ownership), [download/indexer-protocol.md](indexer-protocol.md) (IndexerClient, releases table), [download/orchestration.md](orchestration.md) (Syntaxis queue, QueueItem).

---

## Subsystem Ownership

IRC announce lives in **Zetesis** — it is a form of search result ingestion, not a separate subsystem.

**Rationale:** Zetesis already owns indexer credentials, protocol negotiation, and the `releases` insert path. IRC announces are another source of `AnnounceRelease` records that feed the same downstream pipeline. Creating a new subsystem for one feature type would violate clean domain boundaries.

**Lifecycle:** IRC connections are long-running Tokio tasks spawned by Zetesis at startup, one per unique IRC server hostname. Announce matches are fed to Syntaxis via direct call — the same path used by Episkope after a regular search result.

---

## `IrcAnnounceDefinition` Type

Per-tracker announce configuration stored in Horismos config (TOML). Not in Cardigann YAML — IRC announce patterns are Harmonia-native configuration.

```rust
pub struct IrcAnnounceDefinition {
    pub tracker_id: String,         // matches indexers.name — links to the indexer registry
    pub network: String,            // IRC server hostname, e.g. "irc.p2p-network.net"
    pub port: u16,                  // typically 6697 for TLS
    pub tls: bool,                  // default: true
    pub channel: String,            // announce channel, e.g. "#announces"
    pub announce_bot: String,       // bot nickname that posts announces, e.g. "AnnounceBot"
    pub pattern: String,            // regex pattern with named capture groups (compiled at startup)
    pub invite_cmd: Option<String>, // e.g. "/msg InviteBot !invite {nick} {key}" (some trackers)
    pub auth: IrcAuth,
}

pub enum IrcAuth {
    Sasl {
        username: String,
        password: String,
    },
    NickServ {
        password: String,
    },
    None,
}
```

**`pattern` requirements:** Named capture groups at minimum: `title`, `category`, `size`, `url`. Additional groups are permitted and stored in `AnnounceRelease.extra_fields`. The regex is compiled once at startup and shared across all messages on that channel.

**`tracker_id`** links to `indexers.name` — announce matches create `releases` rows with the matching `indexer_id`. If no indexer with that name exists, the announce is logged at warn level and discarded.

---

## Announce Parsing Flow

```
IRC client receives PRIVMSG on announce channel
    |
Filter: message sender nick != announce_bot? → discard
    |
Apply pattern regex to message text
    |
Regex no match? → log at trace level, discard (expected for non-matching messages)
    |
Extract named capture groups → AnnounceRelease
    |
Check AnnounceRelease against active wants:
    - Category match against want's media_type
    - Title match (fuzzy, same logic as Episkope)
    |
No want match? → discard
    |
Want match found:
    - Zetesis inserts releases row
    - Zetesis creates QueueItem with priority = 3 (wanted-missing)
    - Zetesis calls Syntaxis.enqueue(queue_item) directly
```

Announce parsing failure (regex no match) is not logged as error — the vast majority of announces on any channel will not match active wants.

---

## `AnnounceRelease` Type

```rust
pub struct AnnounceRelease {
    pub tracker_id: String,
    pub title: String,
    pub category: Option<String>,
    pub size_bytes: Option<u64>,
    pub download_url: String,
    pub announced_at: DateTime<Utc>,
    pub extra_fields: HashMap<String, String>,  // additional named capture groups
}
```

**From announce to `releases` row:** Zetesis maps `AnnounceRelease` fields to the `releases` table schema (same as from `SearchResult`). `found_at` = `announced_at`. `indexer_id` = looked up from `tracker_id` via `indexers.name`.

---

## Connection Management

### Group by Network

Multiple trackers on the same IRC server share **one connection**. One Tokio task per unique IRC server hostname — not per tracker, not per channel.

```
Startup:
    Group IrcAnnounceDefinitions by network hostname
    For each unique network: spawn IrcNetworkTask
        Subscribes to all channels on that network
        Routes messages by (channel, sender_nick) → tracker definition
```

**Message routing:** When a PRIVMSG arrives, the task looks up which `IrcAnnounceDefinition` matches `(channel, announce_bot)` and applies that definition's regex pattern.

### Global Nick

A single configurable IRC nickname is used across all networks:

```toml
[zetesis]
irc_nick = "harmonia-bot"
```

Nick conflicts (nick already in use on connect): append a numeric suffix (`harmonia-bot1`, `harmonia-bot2`). The resolved nick is logged at info level.

### `irc` Crate 1.0 Usage

```rust
use irc::client::prelude::*;

let config = Config {
    nickname: Some(global_nick.clone()),
    server: Some(network.server.clone()),
    port: Some(network.port),
    use_tls: Some(network.tls),
    channels: channels_for_this_network,  // all channels configured for this server
    ..Default::default()
};

let mut client = Client::from_config(config).await
    .context(IrcConnectSnafu { network: network.server.clone() })?;

client.identify()
    .context(IrcIdentifySnafu)?;

let mut stream = client.stream()
    .context(IrcStreamSnafu)?;

while let Some(message) = stream.next().await.transpose()
    .context(IrcStreamSnafu)?
{
    match message.command {
        Command::PRIVMSG(ref target, ref text) => {
            handle_privmsg(target, text, &message, &definitions);
        }
        _ => {}
    }
}
```

---

## Reconnection Strategy

On connection drop (stream error or `Err(Closed)` from next()):

| Attempt | Backoff |
|---------|---------|
| 1 | 5 seconds |
| 2 | 15 seconds |
| 3 | 45 seconds |
| 4 | 2 minutes |
| 5 | 5 minutes |
| 6+ | `irc_reconnect_max_seconds` (default: 600 seconds) |

After reaching the max backoff interval, retries continue indefinitely at that interval. The task never exits — IRC announce is a long-running monitor, not a one-shot request.

**On successful reconnect:**
1. Re-identify (SASL or NickServ per auth configuration)
2. Re-send invite commands for channels that require them
3. Re-join all configured channels
4. Log reconnect at info level with elapsed downtime

---

## Authentication Flows

### SASL PLAIN (Preferred)

The `irc` crate supports SASL natively via `Config`:

```rust
let config = Config {
    nickname: Some(nick.clone()),
    server: Some(network.server.clone()),
    use_tls: Some(true),
    // SASL auth fields:
    nick_password: None,   // SASL replaces NickServ
    // irc crate handles SASL handshake automatically before JOIN
    ..Default::default()
};
```

Authentication happens during the connection handshake before channel joins. SASL failure → `IrcIdentify` error → reconnection backoff.

### NickServ (Legacy)

```rust
// After CONNECTED and before channel JOINs:
client.send_privmsg("NickServ", format!("IDENTIFY {}", password))?;

// Wait for NickServ acknowledgement (RPL_NOTICE from NickServ)
// Then proceed with channel joins and invite commands
```

NickServ auth is fire-and-forget in the message stream — Zetesis sends the IDENTIFY and then waits a configurable `nickserv_wait_seconds` (default: 3) before joining channels.

### Invite Commands

Some trackers require sending a command to a bot before the announce channel will accept the join:

```rust
if let Some(ref invite_cmd) = definition.invite_cmd {
    // Resolve template: {nick} → actual_nick, {key} → tracker key from config
    let resolved = resolve_invite_cmd(invite_cmd, &actual_nick, tracker_key);
    client.send_raw(resolved)?;
    // Wait for channel invite (INVITE message) then auto-accept
}
```

The `invite_cmd` field is a template string. The `irc` crate handles auto-accepting INVITE messages — Zetesis joins the invited channel automatically.

---

## Error Handling

`ZetesisError` variants for IRC operations (extending the enum from `indexer-protocol.md`):

```rust
#[derive(Debug, Snafu)]
pub enum ZetesisError {
    // ... existing variants from indexer-protocol.md ...

    #[snafu(display("failed to connect to IRC network {network}"))]
    IrcConnect {
        network: String,
        source: irc::error::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("IRC authentication failed on {network}"))]
    IrcIdentify {
        network: String,
        source: irc::error::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("IRC message stream error on {network}"))]
    IrcStream {
        network: String,
        source: irc::error::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("failed to join IRC channel {channel} on {network}"))]
    IrcChannelJoin {
        channel: String,
        network: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
```

**Announce parse failures** (regex no match) are NOT errors — log at `trace` level and discard. The overwhelming majority of announces on any channel will not match.

**Error recovery:** All IRC errors trigger reconnection backoff rather than permanent failure. The network task restarts itself. Only unrecoverable OS-level errors (OOM, etc.) would propagate to the Tokio runtime.

---

## Horismos Configuration — `[zetesis]` IRC Additions

```toml
[zetesis]
# Global IRC nickname used across all configured networks
irc_nick = "harmonia-bot"

# Maximum reconnect backoff in seconds (actual max interval, not total time)
irc_reconnect_max_seconds = 600

# IRC announce definitions — one entry per tracker channel
[[zetesis.irc_announces]]
tracker_id = "MyTracker"          # matches indexers.name
network = "irc.mytracker.net"
port = 6697
tls = true
channel = "#announce"
announce_bot = "Synd1c4t3"
pattern = 'New Torrent: \[(?P<category>[^\]]+)\] (?P<title>.+?) \| (?P<size>[\d.]+ [KMGT]B) \| (?P<url>https?://\S+)'
# invite_cmd = "/msg InviteBot !invite {nick} {key}"  # optional

[zetesis.irc_announces.auth]
type = "sasl"
username = "my-username"
password = "my-sasl-password"

[[zetesis.irc_announces]]
tracker_id = "OtherTracker"
network = "irc.othernet.org"
port = 6667
tls = false
channel = "#releases"
announce_bot = "AnnounceBot"
pattern = '(?P<title>.+?) - (?P<category>\w+) - (?P<size>[\d.]+[MG]B) - (?P<url>https?://\S+)'

[zetesis.irc_announces.auth]
type = "nickserv"
password = "my-nickserv-password"
```

`ZetesisConfig` additions in `crates/horismos/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IrcAnnounceConfig {
    pub tracker_id: String,
    pub network: String,
    pub port: u16,
    pub tls: bool,
    pub channel: String,
    pub announce_bot: String,
    pub pattern: String,
    pub invite_cmd: Option<String>,
    pub auth: IrcAuthConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IrcAuthConfig {
    Sasl { username: String, password: String },
    NickServ { password: String },
    None,
}

// Added to ZetesisConfig:
pub irc_nick: Option<String>,                       // required if irc_announces non-empty
pub irc_reconnect_max_seconds: u64,                 // default: 600
pub irc_announces: Vec<IrcAnnounceConfig>,          // default: empty
```
