// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Data;

namespace Mouseion.Core.Datastore;

public interface IDatabase
{
    IDbConnection OpenConnection();
    Version Version { get; }
    int Migration { get; }
    DatabaseType DatabaseType { get; }
    void Vacuum();
}
