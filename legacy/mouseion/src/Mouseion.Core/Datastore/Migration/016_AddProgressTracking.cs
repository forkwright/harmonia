// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(16)]
public class AddProgressTracking : FluentMigrator.Migration
{
    public override void Up()
    {
        // MediaProgress table for unified progress tracking
        Create.Table("MediaProgress")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
                .ForeignKey("FK_MediaProgress_MediaItems", "MediaItems", "Id").OnDelete(System.Data.Rule.Cascade)
            .WithColumn("UserId").AsString(255).NotNullable().WithDefaultValue("default")
            .WithColumn("PositionMs").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("TotalDurationMs").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("PercentComplete").AsDecimal(5, 2).NotNullable().WithDefaultValue(0)
            .WithColumn("LastPlayedAt").AsDateTime().NotNullable()
            .WithColumn("IsComplete").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        // Unique constraint: one progress record per user per media item
        Create.Index("IX_MediaProgress_MediaItemId_UserId")
            .OnTable("MediaProgress")
            .OnColumn("MediaItemId").Ascending()
            .OnColumn("UserId").Ascending()
            .WithOptions().Unique();

        // Index for "continue watching/reading/listening" queries
        Create.Index("IX_MediaProgress_LastPlayedAt")
            .OnTable("MediaProgress")
            .OnColumn("LastPlayedAt").Descending();

        // PlaybackSession table for session tracking
        Create.Table("PlaybackSessions")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("SessionId").AsString(36).NotNullable().Unique()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
                .ForeignKey("FK_PlaybackSessions_MediaItems", "MediaItems", "Id").OnDelete(System.Data.Rule.Cascade)
            .WithColumn("UserId").AsString(255).NotNullable().WithDefaultValue("default")
            .WithColumn("DeviceName").AsString(255).NotNullable().WithDefaultValue("")
            .WithColumn("DeviceType").AsString(100).NotNullable().WithDefaultValue("")
            .WithColumn("StartedAt").AsDateTime().NotNullable()
            .WithColumn("EndedAt").AsDateTime().Nullable()
            .WithColumn("StartPositionMs").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("EndPositionMs").AsInt64().Nullable()
            .WithColumn("DurationMs").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("IsActive").AsBoolean().NotNullable().WithDefaultValue(true);

        // Index for active sessions queries
        Create.Index("IX_PlaybackSessions_IsActive")
            .OnTable("PlaybackSessions")
            .OnColumn("IsActive").Ascending()
            .OnColumn("StartedAt").Descending();

        // Index for recent sessions by user
        Create.Index("IX_PlaybackSessions_UserId_StartedAt")
            .OnTable("PlaybackSessions")
            .OnColumn("UserId").Ascending()
            .OnColumn("StartedAt").Descending();
    }

    public override void Down()
    {
        Delete.Table("PlaybackSessions");
        Delete.Table("MediaProgress");
    }
}
