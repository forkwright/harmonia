// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(13, "Add Tags table")]
public class Migration013AddTags : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("Tags")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Label").AsString().NotNullable().Unique();

        Create.Index("IX_Tags_Label")
            .OnTable("Tags")
            .OnColumn("Label")
            .Unique();
    }

    public override void Down()
    {
        Delete.Table("Tags");
    }
}
