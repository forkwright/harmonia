// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Datastore.Migration.Framework;

public class MigrationContext
{
    public MigrationType MigrationType { get; }

    public MigrationContext(MigrationType migrationType)
    {
        MigrationType = migrationType;
    }
}
