#!/usr/bin/env bash
# Mouseion config backup — snapshot /config volume to timestamped archive
#
# Usage:
#   ./deploy/backup.sh                    # default: ~/docker_configs/mouseion/config
#   ./deploy/backup.sh /path/to/config    # custom config path
#   BACKUP_DIR=/mnt/nas/backups ./deploy/backup.sh  # custom backup destination
#
# Cron example (daily at 3am):
#   0 3 * * * /path/to/mouseion/deploy/backup.sh >> /var/log/mouseion-backup.log 2>&1
#
# Retention: keeps last 14 backups, deletes older ones.

set -euo pipefail

CONFIG_DIR="${1:-${MOUSEION_CONFIG:-$HOME/docker_configs/mouseion/config}}"
BACKUP_DIR="${BACKUP_DIR:-$HOME/backups/mouseion}"
RETENTION_DAYS=14
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/mouseion-${TIMESTAMP}.tar.gz"

# Validate source
if [ ! -d "$CONFIG_DIR" ]; then
    echo "ERROR: Config directory not found: $CONFIG_DIR"
    exit 1
fi

if [ ! -f "$CONFIG_DIR/mouseion.db" ]; then
    echo "ERROR: mouseion.db not found in $CONFIG_DIR — is this the right path?"
    exit 1
fi

# Create backup directory
mkdir -p "$BACKUP_DIR"

# SQLite online backup — safe even while Mouseion is running
# Uses .backup command which handles WAL correctly
BACKUP_DB="${CONFIG_DIR}/mouseion-backup.db"
BACKUP_LOG_DB="${CONFIG_DIR}/logs-backup.db"

echo "[$(date)] Starting backup of $CONFIG_DIR"

# Create consistent SQLite snapshots
if command -v sqlite3 &>/dev/null; then
    echo "[$(date)] Creating SQLite online backup..."
    sqlite3 "$CONFIG_DIR/mouseion.db" ".backup '$BACKUP_DB'"
    sqlite3 "$CONFIG_DIR/logs.db" ".backup '$BACKUP_LOG_DB'" 2>/dev/null || true

    # Archive the backup copies + secrets
    tar -czf "$BACKUP_FILE" \
        -C "$CONFIG_DIR" \
        mouseion-backup.db \
        $([ -f "$BACKUP_LOG_DB" ] && echo "logs-backup.db") \
        $([ -f "$CONFIG_DIR/.jwt-secret" ] && echo ".jwt-secret") \
        $([ -f "$CONFIG_DIR/.webhook-secret" ] && echo ".webhook-secret") \
        $([ -f "$CONFIG_DIR/appsettings.json" ] && echo "appsettings.json") \
        2>/dev/null

    # Clean up temp backup files
    rm -f "$BACKUP_DB" "$BACKUP_LOG_DB"
else
    echo "[$(date)] WARN: sqlite3 not found — using file copy (may be inconsistent if Mouseion is running)"
    tar -czf "$BACKUP_FILE" \
        -C "$CONFIG_DIR" \
        mouseion.db \
        $([ -f "$CONFIG_DIR/logs.db" ] && echo "logs.db") \
        $([ -f "$CONFIG_DIR/.jwt-secret" ] && echo ".jwt-secret") \
        $([ -f "$CONFIG_DIR/.webhook-secret" ] && echo ".webhook-secret") \
        $([ -f "$CONFIG_DIR/appsettings.json" ] && echo "appsettings.json") \
        2>/dev/null
fi

BACKUP_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo "[$(date)] Backup complete: $BACKUP_FILE ($BACKUP_SIZE)"

# Prune old backups
DELETED=0
find "$BACKUP_DIR" -name "mouseion-*.tar.gz" -mtime +$RETENTION_DAYS -print -delete | while read -r f; do
    DELETED=$((DELETED + 1))
done
echo "[$(date)] Pruned backups older than ${RETENTION_DAYS} days"

# Summary
TOTAL=$(find "$BACKUP_DIR" -name "mouseion-*.tar.gz" | wc -l)
TOTAL_SIZE=$(du -sh "$BACKUP_DIR" | cut -f1)
echo "[$(date)] Retention: $TOTAL backups, $TOTAL_SIZE total"
