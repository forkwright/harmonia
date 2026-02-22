// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(032)]
public class AddImportHistory : FluentMigrator.Migration
{
    public override void Up()
    {
        // Import sessions — one per sync/wizard execution
        Create.Table("ImportSessions")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("ImportListId").AsInt32().NotNullable()
            .WithColumn("ImportListName").AsString(200).NotNullable()
            .WithColumn("ListType").AsInt32().NotNullable()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Status").AsInt32().NotNullable().WithDefaultValue(0) // Pending=0, Running=1, Completed=2, Failed=3, DryRun=4
            .WithColumn("IsDryRun").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("ItemsFetched").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ItemsAdded").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ItemsUpdated").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ItemsSkipped").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ItemsFailed").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ErrorMessage").AsString(int.MaxValue).Nullable()
            .WithColumn("StartedAt").AsDateTime().NotNullable()
            .WithColumn("CompletedAt").AsDateTime().Nullable();

        Create.Index("IX_ImportSessions_ImportListId")
            .OnTable("ImportSessions")
            .OnColumn("ImportListId").Ascending();

        Create.Index("IX_ImportSessions_StartedAt")
            .OnTable("ImportSessions")
            .OnColumn("StartedAt").Descending();

        // Import session items — individual item outcomes within a session
        Create.Table("ImportSessionItems")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("SessionId").AsInt32().NotNullable()
            .WithColumn("Title").AsString(500).NotNullable()
            .WithColumn("Year").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("Action").AsInt32().NotNullable() // Added=0, Updated=1, Skipped=2, Conflict=3, Failed=4, Excluded=5
            .WithColumn("Reason").AsString(500).Nullable() // Why this action was taken
            .WithColumn("ExternalIds").AsString(int.MaxValue).Nullable() // JSON: {tmdbId, imdbId, isbn, ...}
            .WithColumn("MediaItemId").AsInt32().Nullable() // If linked to existing item
            .WithColumn("DiffJson").AsString(int.MaxValue).Nullable() // For conflicts: {field: {old, new}}
            .WithColumn("UserRating").AsInt32().Nullable()
            .WithColumn("ProcessedAt").AsDateTime().NotNullable();

        Create.Index("IX_ImportSessionItems_SessionId")
            .OnTable("ImportSessionItems")
            .OnColumn("SessionId").Ascending();

        Create.Index("IX_ImportSessionItems_Action")
            .OnTable("ImportSessionItems")
            .OnColumn("Action").Ascending();

        Create.ForeignKey("FK_ImportSessionItems_SessionId")
            .FromTable("ImportSessionItems").ForeignColumn("SessionId")
            .ToTable("ImportSessions").PrimaryColumn("Id");
    }

    public override void Down()
    {
        Delete.ForeignKey("FK_ImportSessionItems_SessionId").OnTable("ImportSessionItems");
        Delete.Table("ImportSessionItems");
        Delete.Table("ImportSessions");
    }
}
