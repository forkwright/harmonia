// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(7, "Add RootFolders table for library path management")]
public class Migration_007_AddRootFolders : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("RootFolders")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Path").AsString().NotNullable().Unique()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Accessible").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("FreeSpace").AsInt64().Nullable()
            .WithColumn("TotalSpace").AsInt64().Nullable();

        Create.Index("IX_RootFolders_MediaType").OnTable("RootFolders").OnColumn("MediaType");
    }

    public override void Down()
    {
        Delete.Index("IX_RootFolders_MediaType").OnTable("RootFolders");
        Delete.Table("RootFolders");
    }
}
