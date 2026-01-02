// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Datastore;

public class DatabaseConnectionInfo
{
    public DatabaseType DatabaseType { get; set; }
    public string ConnectionString { get; set; } = string.Empty;
}

public enum DatabaseType
{
    SQLite,
    PostgreSQL
}
