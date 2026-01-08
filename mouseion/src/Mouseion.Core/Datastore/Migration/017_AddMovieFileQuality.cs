// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(17)]
public class AddMovieFileQuality : FluentMigrator.Migration
{
    public override void Up()
    {
        // Add Quality column to MovieFiles table
        // QualityModel is serialized as JSON (IEmbeddedDocument)
        Alter.Table("MovieFiles")
            .AddColumn("Quality").AsString().Nullable();
    }

    public override void Down()
    {
        Delete.Column("Quality").FromTable("MovieFiles");
    }
}
