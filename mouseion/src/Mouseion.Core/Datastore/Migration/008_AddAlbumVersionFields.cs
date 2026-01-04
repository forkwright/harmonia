// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(8, "Add album version fields for release-group grouping")]
public class Migration_008_AddAlbumVersionFields : FluentMigrator.Migration
{
    public override void Up()
    {
        Alter.Table("Albums")
            .AddColumn("ReleaseGroupMbid").AsString().Nullable()
            .AddColumn("ReleaseStatus").AsString().Nullable()
            .AddColumn("ReleaseCountry").AsString().Nullable()
            .AddColumn("RecordLabel").AsString().Nullable();

        Create.Index("IX_Albums_ReleaseGroupMbid")
            .OnTable("Albums")
            .OnColumn("ReleaseGroupMbid");
    }

    public override void Down()
    {
        Delete.Index("IX_Albums_ReleaseGroupMbid").OnTable("Albums");
        Delete.Column("ReleaseGroupMbid").FromTable("Albums");
        Delete.Column("ReleaseStatus").FromTable("Albums");
        Delete.Column("ReleaseCountry").FromTable("Albums");
        Delete.Column("RecordLabel").FromTable("Albums");
    }
}
