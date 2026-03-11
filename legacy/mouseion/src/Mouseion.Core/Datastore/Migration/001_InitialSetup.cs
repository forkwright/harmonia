// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(1, "Initial Setup")]
public class Migration_001_InitialSetup : FluentMigrator.Migration
{
    public override void Up()
    {
        // FluentMigrator creates its own VersionInfo table automatically
        // This migration exists as a placeholder for the first migration number
    }

    public override void Down()
    {
        // Nothing to roll back
    }
}
