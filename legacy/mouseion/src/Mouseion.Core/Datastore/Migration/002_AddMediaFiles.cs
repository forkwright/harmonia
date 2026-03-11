// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(2, "Add MediaFiles")]
public class Migration_002_AddMediaFiles : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("MediaFiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Path").AsString().NotNullable()
            .WithColumn("RelativePath").AsString().Nullable()
            .WithColumn("Size").AsInt64().NotNullable()
            .WithColumn("DateAdded").AsDateTime().NotNullable()
            .WithColumn("DurationSeconds").AsInt32().Nullable()
            .WithColumn("Bitrate").AsInt32().Nullable()
            .WithColumn("SampleRate").AsInt32().Nullable()
            .WithColumn("Channels").AsInt32().Nullable()
            .WithColumn("Format").AsString().Nullable()
            .WithColumn("Quality").AsString().Nullable();

        Create.Index("IX_MediaFiles_MediaItemId")
            .OnTable("MediaFiles")
            .OnColumn("MediaItemId");

        Create.Index("IX_MediaFiles_Path")
            .OnTable("MediaFiles")
            .OnColumn("Path")
            .Unique();
    }

    public override void Down()
    {
        Delete.Table("MediaFiles");
    }
}
