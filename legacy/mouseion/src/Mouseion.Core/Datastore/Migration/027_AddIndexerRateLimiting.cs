// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(027)]
public class AddIndexerRateLimiting : FluentMigrator.Migration
{
    public override void Up()
    {
        // Per-indexer rate limit configuration
        Create.Table("IndexerRateLimits")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("IndexerName").AsString(256).NotNullable().Unique()
            .WithColumn("MaxRequestsPerHour").AsInt32().NotNullable().WithDefaultValue(100)
            .WithColumn("BackoffUntil").AsDateTime().Nullable()
            .WithColumn("BackoffMultiplier").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("LastErrorCode").AsInt32().Nullable()
            .WithColumn("LastErrorMessage").AsString(1024).Nullable()
            .WithColumn("LastErrorAt").AsDateTime().Nullable()
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        // Sliding window request tracking
        Create.Table("IndexerRequestLog")
            .WithColumn("Id").AsInt64().PrimaryKey().Identity()
            .WithColumn("IndexerName").AsString(256).NotNullable()
            .WithColumn("RequestedAt").AsDateTime().NotNullable()
            .WithColumn("ResponseCode").AsInt32().Nullable()
            .WithColumn("ResponseTimeMs").AsInt32().Nullable()
            .WithColumn("ResultCount").AsInt32().Nullable()
            .WithColumn("SearchQuery").AsString(512).Nullable();

        Create.Index("IX_IndexerRequestLog_Name_Time")
            .OnTable("IndexerRequestLog")
            .OnColumn("IndexerName").Ascending()
            .OnColumn("RequestedAt").Descending();

        // Search history for dedup (Spec 08 Phase 3 prep)
        Create.Table("SearchHistory")
            .WithColumn("Id").AsInt64().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().Nullable()
            .WithColumn("IndexerName").AsString(256).NotNullable()
            .WithColumn("SearchQuery").AsString(512).NotNullable()
            .WithColumn("ResultCount").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("BestMatchTitle").AsString(512).Nullable()
            .WithColumn("BestMatchScore").AsDouble().Nullable()
            .WithColumn("GrabbedReleaseId").AsString(256).Nullable()
            .WithColumn("SearchedAt").AsDateTime().NotNullable();

        Create.Index("IX_SearchHistory_MediaItem")
            .OnTable("SearchHistory")
            .OnColumn("MediaItemId").Ascending()
            .OnColumn("SearchedAt").Descending();

        Create.Index("IX_SearchHistory_Indexer")
            .OnTable("SearchHistory")
            .OnColumn("IndexerName").Ascending()
            .OnColumn("SearchedAt").Descending();
    }

    public override void Down()
    {
        Delete.Table("SearchHistory");
        Delete.Table("IndexerRequestLog");
        Delete.Table("IndexerRateLimits");
    }
}
