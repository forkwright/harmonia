# SQLite storage architecture

> SQLite WAL infrastructure and migration strategy for Harmonia's database layer.
> The `apotheke` crate recommended here fits the workspace layout in [architecture/cargo.md](../architecture/cargo.md).
> Database path and pool sizes are Horismos-configurable; see [architecture/configuration.md](../architecture/configuration.md).

---

## Part 1: SQLite WAL architecture

### Why WAL

WAL (Write-Ahead Logging) mode enables concurrent reads during writes. SQLite enforces a single-writer constraint: exactly one connection may hold the write lock at any time, while any number of reader connections operate in parallel against a consistent snapshot. This matches the workload profile of a media server: frequent reads from multiple subsystems, periodic writes from import and quality curation pipelines. Reference: [sqlite.org/wal.html](https://sqlite.org/wal.html).

---

### Dual-pool design

The dual-pool pattern creates two separate `SqlitePool` instances mirroring SQLite's concurrency model. A single pool with N connections causes write contention: multiple tasks simultaneously race for the exclusive write lock, causing blocking and starvation. Two pools eliminate the race by design.

**Write pool:** `max_connections(1)` is the critical constraint. A single connection serializes all writes, eliminating lock contention. Additional write connections provide no concurrency benefit and add latency.

**Read pool:** `max_connections(num_cpus)` as a starting point. `min_connections(2)` keeps warm connections available for bursty read patterns. `read_only(true)` prevents accidental writes through read-pool connections.

```rust
// crates/apotheke/src/pools.rs
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous, SqlitePoolOptions};
use sqlx::SqlitePool;

pub struct DbPools {
    pub read: SqlitePool,
    pub write: SqlitePool,
}

pub async fn init_pools(db_path: &str) -> Result<DbPools, sqlx::Error> {
    let base_opts = SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let write = SqlitePoolOptions::new()
        .max_connections(1)  // CRITICAL: single writer — SQLite WAL constraint
        .connect_with(base_opts.clone().create_if_missing(true))
        .await?;

    // Apply additional WAL pragmas on write connection after pool creation
    sqlx::query("PRAGMA journal_size_limit = 67108864").execute(&write).await?;
    sqlx::query("PRAGMA temp_store = memory").execute(&write).await?;

    // Run migrations before opening read pool
    sqlx::migrate!("migrations").run(&write).await?;

    let read = SqlitePoolOptions::new()
        .max_connections(num_cpus::get() as u32)  // Start here; tune based on profiling
        .min_connections(2)
        .connect_with(base_opts.read_only(true))
        .await?;

    Ok(DbPools { read, write })
}
```

---

### Pragma specification

These pragmas are applied on the write connection after pool creation. The `journal_mode` and `synchronous` values are also set via `SqliteConnectOptions`; the `PRAGMA` calls here set the values that `SqliteConnectOptions` does not expose.

| Pragma | Value | Rationale |
|--------|-------|-----------|
| `journal_mode` | `WAL` | Concurrent reads during writes; set via `SqliteConnectOptions::journal_mode` |
| `synchronous` | `NORMAL` | Safe with WAL; `FULL` adds fsync latency with negligible durability gain for WAL-mode databases |
| `foreign_keys` | `ON` | Enforce referential integrity at the SQLite layer; set via `SqliteConnectOptions::foreign_keys` |
| `journal_size_limit` | `67108864` (64 MB) | Prevents unbounded WAL file growth during sustained write bursts; starting point; tune upward for high-write workloads |
| `wal_autocheckpoint` | `1000` | SQLite default; checkpoint WAL to main DB file after 1000 pages (~4 MB); reduce if WAL growth is a concern |
| `temp_store` | `memory` | Temporary indices and sort buffers in RAM instead of temp files; improves sort-heavy query performance |

**Tuning guidance:**
- `journal_size_limit`: Increase for workloads with large batch imports. Decrease for memory-constrained devices. Monitor WAL file size under load.
- `wal_autocheckpoint`: Lower values reduce WAL file size but increase checkpoint frequency, adding periodic latency spikes. Default (1000) is appropriate for media server workloads.
- `synchronous = NORMAL`: Safe for WAL mode; committed transactions survive power loss. The WAL file ensures durability between checkpoints. `FULL` is only meaningful in journal (non-WAL) mode.

---

### Write transaction rule

> **HARD RULE: Never hold a write transaction across an `await` point.**

Tokio's cooperative scheduler yields at every `.await` point. When a write transaction is open, SQLite's exclusive write lock is held for the duration of the transaction, not just during individual statements. If a task yields while holding a write transaction, the scheduler can run another task that also needs to write. That second task blocks on the pool's single connection, waiting for the first task to resume and commit. Under sufficient concurrency, this causes latency degradation and eventual deadlock-like starvation.

**Correct pattern: all writes in one atomic block, committed before any external await:**

```rust
async fn record_import(pool: &SqlitePool, item: &ImportedItem) -> Result<(), DbError> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "INSERT INTO haves (id, want_id, media_type, quality_score, file_path, imported_at)
         VALUES (?, ?, ?, ?, ?, ?)",
        item.id, item.want_id, item.media_type, item.quality_score,
        item.file_path, item.imported_at
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE wants SET status = 'fulfilled', fulfilled_at = ? WHERE id = ?",
        item.imported_at, item.want_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;  // Commit before any further async work

    // Notify external systems AFTER commit — not inside the transaction
    notify_plex(&item.file_path).await?;

    Ok(())
}
```

**Incorrect pattern: write transaction held across an external await:**

```rust
async fn record_import_wrong(pool: &SqlitePool, item: &ImportedItem) -> Result<(), DbError> {
    let mut tx = pool.begin().await?;

    sqlx::query!("INSERT INTO haves ...").execute(&mut *tx).await?;

    // WRONG: external HTTP call while transaction is open
    // SQLite write lock held during this entire call
    notify_plex(&item.file_path).await?;  // <-- other tasks cannot write during this

    tx.commit().await?;
    Ok(())
}
```

Reference: RESEARCH.md Pitfall 1 (write transaction held across await).

---

### Checkpoint starvation prevention

Long-running read transactions prevent WAL checkpoints. SQLite's checkpoint operation transfers committed changes from the WAL file back to the main database file. A checkpoint cannot proceed past a WAL frame that is still being read by an open read transaction. If a read transaction remains open for minutes (e.g., a library scan streaming thousands of rows), WAL frames accumulate without being checkpointed and the WAL file grows without bound.

**Mitigations:**
- **Prefer paginated queries over full-table reads.** `LIMIT` / `OFFSET` or cursor-based pagination keeps individual read transactions short.
- **`journal_size_limit`** is a safety net; it limits WAL file size and causes SQLite to checkpoint more aggressively when the limit is approached.
- **Avoid holding `SqlitePool` connections across unrelated work.** Acquire a connection, complete the query, release it.

Reference: RESEARCH.md Pitfall 2 (checkpoint starvation).

---

### Harmonia-db crate

A new `apotheke` crate in the workspace is the recommended location for `DbPools`, the migration runner, and typed query functions. Multiple subsystems need database access (Episkope reads wants, Kritike reads quality profiles, Kathodos writes haves); if each subsystem set up its own pool independently, the single-writer constraint would be violated or require global coordination.

`apotheke` is a leaf dependency alongside `horismos`. It exports `DbPools` and all query functions. Subsystems receive `Arc<DbPools>` via constructor injection.

**Updated workspace crate inventory (addition to [cargo.md](../architecture/cargo.md)):**

```
crates/
├── themelion/    # Leaf — shared newtypes, events
├── horismos/           # Leaf — configuration
├── apotheke/        # Leaf — database pools, migrations, typed queries
├── exousia/            # depends on themelion, horismos
├── ...                 # all other subsystems can depend on apotheke
```

**Updated dependency entry in `Cargo.toml`:**

```toml
# harmonia/Cargo.toml — workspace.dependencies addition
apotheke = { path = "crates/apotheke" }

# apotheke/Cargo.toml
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "macros", "migrate"] }
uuid = { version = "1", features = ["v7"] }
num_cpus = "1"
```

Subsystems that need database access declare `apotheke.workspace = true` in their `[dependencies]` section.

---

### Configuration integration

Database path and pool sizes are Horismos-configurable via a `[database]` section in `harmonia.toml`. The `apotheke` crate receives `Arc<DatabaseConfig>` from archon at startup.

```toml
# harmonia.toml — safe committed defaults
[database]
path = "data/harmonia.db"
read_pool_size = 0          # 0 = auto (num_cpus); set explicitly to override
```

```rust
// crates/horismos/src/config.rs — addition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,          // default: "data/harmonia.db"
    pub read_pool_size: u32,    // default: 0 (= num_cpus at runtime)
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/harmonia.db"),
            read_pool_size: 0,
        }
    }
}
```

Override via environment: `HARMONIA__DATABASE__PATH=/mnt/data/harmonia.db` or `HARMONIA__DATABASE__READ_POOL_SIZE=8`.

---

## Part 2: migration strategy

### Sqlx embedded migrations

`sqlx::migrate!` embeds SQL migration files at compile time. On startup, `MIGRATOR.run(&write_pool)` applies all pending migrations in version order. The `_sqlx_migrations` table (created automatically by sqlx on first run) tracks which versions have been applied, their checksums, and timestamps.

```rust
// crates/apotheke/src/migrations.rs
use sqlx::migrate::Migrator;
use sqlx::SqlitePool;

static MIGRATOR: Migrator = sqlx::migrate!("migrations");

pub async fn run_migrations(write_pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(write_pool).await
}
```

Migrations run on the write pool only. The read pool is opened after migrations complete (as shown in `init_pools` above), ensuring the read pool never sees a pre-migration schema.

---

### Migration file naming

Sequential integers, not timestamps. Pattern: `NNNN_description.sql`. Timestamps add noise, introduce time-zone ambiguity, and create lexicographic ordering that breaks if two developers add migrations on the same day. Sequential integers are readable, unambiguous, and predictable.

```
crates/apotheke/migrations/
├── 0001_create_media_registry.sql
├── 0002_create_quality_profiles.sql
├── 0003_create_wants.sql
├── 0004_create_music_schema.sql
├── 0005_create_audiobook_schema.sql
├── 0006_create_book_schema.sql
├── 0007_create_comic_schema.sql
├── 0008_create_podcast_schema.sql
├── 0009_create_movie_schema.sql
├── 0010_create_tv_schema.sql
└── 0011_create_indexes.sql
```

These correspond to the Phase 4 schemas. New migrations receive the next sequential number; never insert between existing numbers, never reuse a number.

---

### Build.rs requirement

The `apotheke` crate **must** include a `build.rs` with the following content:

```rust
// crates/apotheke/build.rs
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
```

Without this, Cargo's change detection does not watch `.sql` files. Adding or modifying a migration file completes `cargo build` without recompiling the crate that embeds it; the binary ships with stale embedded migrations. This is a silent failure: the binary builds successfully but runs old schema definitions.

Reference: RESEARCH.md Pitfall 5 (missing build.rs for migration recompilation).

---

### Offline mode for CI

`sqlx::query!` and `sqlx::query_as!` macros verify SQL against a live database at compile time. In CI where no database exists, this verification fails unless offline mode is configured.

**Setup:**

1. Run `cargo sqlx prepare` locally after every schema or query change. This generates `.sqlx/` (a directory of query metadata files) in the crate root.
2. Commit the `.sqlx/` directory to the repository.
3. Set `SQLX_OFFLINE=true` in CI environment. With this flag, sqlx uses the committed metadata instead of a live connection.

```yaml
# .github/workflows/ci.yml — relevant section
env:
  SQLX_OFFLINE: "true"
```

```bash
# Developer workflow: after changing any migration or query
cd crates/apotheke
cargo sqlx prepare
git add .sqlx/
git commit -m "chore(apotheke): update sqlx offline query cache"
```

**Warning sign:** CI passes locally but fails in GitHub Actions with "cannot connect to database"; `.sqlx/` was not committed or `SQLX_OFFLINE` is not set.

Reference: RESEARCH.md Pitfall 6 (sqlx offline mode not prepared).

---

### Version tracking

The `_sqlx_migrations` table is created and managed entirely by sqlx. Harmonia does not create, read from, or modify this table directly.

| Column | Description |
|--------|-------------|
| `version` | The integer prefix of the migration file (e.g., `1` for `0001_create_media_registry.sql`) |
| `description` | The text portion after the number (e.g., `create_media_registry`) |
| `installed_on` | UTC timestamp of when the migration was applied |
| `success` | `1` if the migration completed, `0` if it failed midway |
| `checksum` | SHA-256 of the migration file content; sqlx verifies this on each run to detect tampering |

sqlx refuses to run if an applied migration's checksum no longer matches, which means **never modify a migration file after it has been applied to any environment**. Add a new migration instead.

---

### Zero-downtime migration approach

For future schema changes:

**Additive changes (no downtime):** Adding columns with defaults, adding new tables, adding indexes. These are backward-compatible; old code reading from the table ignores new columns.

**Destructive changes (two-phase approach):**

Phase 1 (release N): Add the new column/table. Begin writing to both old and new locations. Old reads still work.

Phase 2 (release N+1): Migrate existing data to the new location. Remove the old column/table.

Never perform data migration and column removal in the same migration; if anything fails during migration, rollback leaves the schema in an intermediate state that is incompatible with both old and new application code.

**Policy:** Migrations are append-only. Every migration in `crates/apotheke/migrations/` must be safe to apply against a running (paused, not actively writing) Harmonia instance. Multi-step destructive changes are split across releases with explicit migration files for each step.

---

### Anti-patterns

**No external migration tool at runtime.** `sqlx-cli` is a dev tool only. The binary runs migrations itself via `sqlx::migrate!`. Adding a runtime dependency on `sqlx-cli` introduces a deployment requirement and a version-mismatch failure mode.

**No timestamp-named migrations.** `20241101_init.sql`: ambiguous ordering when two developers add migrations on the same day; longer names with no benefit over sequential integers.

**No manual `_sqlx_migrations` manipulation.** This table is owned by sqlx. Direct inserts or deletes corrupt the migration state and will cause `MigrateError::VersionMissing` or checksum failures.

**No `BEGIN IMMEDIATE` for read transactions.** `BEGIN IMMEDIATE` acquires the write lock before any statements; this is unnecessary for reads and defeats WAL's concurrency advantage.

**No single pool for reads and writes.** Even a pool with `max_connections(1)` used for both reads and writes serializes all database access, not just writes. Use the dual-pool design.
