// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(026)]
public class AddUserForeignKeys : FluentMigrator.Migration
{
    public override void Up()
    {
        // MediaProgress: add integer UserIdInt alongside existing string UserId
        Alter.Table("MediaProgress")
            .AddColumn("UserIdInt").AsInt32().Nullable();

        // PlaybackSessions: add integer UserIdInt
        Alter.Table("PlaybackSessions")
            .AddColumn("UserIdInt").AsInt32().Nullable();

        // Set all existing records to user 1 (admin user from migration 025)
        Execute.Sql(@"UPDATE ""MediaProgress"" SET ""UserIdInt"" = 1");
        Execute.Sql(@"UPDATE ""PlaybackSessions"" SET ""UserIdInt"" = 1");

        // Cross-device sync: PlaybackQueue table
        Create.Table("PlaybackQueues")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("DeviceName").AsString(256).NotNullable().WithDefaultValue("")
            .WithColumn("QueueData").AsString(int.MaxValue).NotNullable().WithDefaultValue("[]")
            .WithColumn("CurrentIndex").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ShuffleEnabled").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("RepeatMode").AsString(20).NotNullable().WithDefaultValue("none")
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        Create.Index("IX_PlaybackQueues_UserId")
            .OnTable("PlaybackQueues")
            .OnColumn("UserId");

        Create.Index("IX_PlaybackQueues_UserId_DeviceName")
            .OnTable("PlaybackQueues")
            .OnColumn("UserId").Ascending()
            .OnColumn("DeviceName").Ascending()
            .WithOptions().Unique();

        // Trakt import: OAuth token storage
        Create.Table("ImportListTokens")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("ImportListDefinitionId").AsInt32().NotNullable()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("AccessToken").AsString(1024).NotNullable()
            .WithColumn("RefreshToken").AsString(1024).Nullable()
            .WithColumn("TokenType").AsString(50).NotNullable().WithDefaultValue("Bearer")
            .WithColumn("ExpiresAt").AsDateTime().Nullable()
            .WithColumn("Scope").AsString(512).Nullable()
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        Create.Index("IX_ImportListTokens_ListId_UserId")
            .OnTable("ImportListTokens")
            .OnColumn("ImportListDefinitionId").Ascending()
            .OnColumn("UserId").Ascending()
            .WithOptions().Unique();

        // Import sync history for incremental imports
        Create.Table("ImportSyncHistory")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("ImportListDefinitionId").AsInt32().NotNullable()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("SyncType").AsString(50).NotNullable()
            .WithColumn("ItemsAdded").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ItemsUpdated").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("ItemsSkipped").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("Errors").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("LastSyncedAt").AsDateTime().NotNullable()
            .WithColumn("SyncedUpTo").AsDateTime().Nullable()
            .WithColumn("ErrorDetails").AsString(int.MaxValue).Nullable();

        Create.Index("IX_ImportSyncHistory_ListId_UserId")
            .OnTable("ImportSyncHistory")
            .OnColumn("ImportListDefinitionId").Ascending()
            .OnColumn("UserId").Ascending();
    }

    public override void Down()
    {
        Delete.Table("ImportSyncHistory");
        Delete.Table("ImportListTokens");
        Delete.Table("PlaybackQueues");

        Delete.Column("UserIdInt").FromTable("PlaybackSessions");
        Delete.Column("UserIdInt").FromTable("MediaProgress");
    }
}
