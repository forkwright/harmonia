// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(29, "Add search history and deduplication tables")]
public class Migration029AddSearchDeduplication : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("SearchHistory")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("MediaType").AsString(50).NotNullable()
            .WithColumn("IndexerName").AsString(200).NotNullable()
            .WithColumn("SearchQuery").AsString(1000).NotNullable()
            .WithColumn("ResultCount").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("BestMatchTitle").AsString(500).Nullable()
            .WithColumn("BestMatchGuid").AsString(500).Nullable()
            .WithColumn("SearchedAt").AsDateTime().NotNullable();

        Create.Index("IX_SearchHistory_MediaItem_Indexer")
            .OnTable("SearchHistory")
            .OnColumn("MediaItemId").Ascending()
            .OnColumn("IndexerName").Ascending();

        Create.Index("IX_SearchHistory_SearchedAt")
            .OnTable("SearchHistory")
            .OnColumn("SearchedAt");

        Create.Table("GrabbedReleases")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("IndexerName").AsString(200).NotNullable()
            .WithColumn("ReleaseGuid").AsString(500).NotNullable()
            .WithColumn("ReleaseTitle").AsString(500).NotNullable()
            .WithColumn("Quality").AsString(100).NotNullable()
            .WithColumn("SizeBytes").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("DownloadClientId").AsString(200).Nullable()
            .WithColumn("GrabbedAt").AsDateTime().NotNullable();

        Create.Index("IX_GrabbedReleases_ReleaseGuid")
            .OnTable("GrabbedReleases")
            .OnColumn("ReleaseGuid");

        Create.Index("IX_GrabbedReleases_MediaItemId")
            .OnTable("GrabbedReleases")
            .OnColumn("MediaItemId");

        Create.Table("SkippedReleases")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("ReleaseGuid").AsString(500).NotNullable()
            .WithColumn("ReleaseTitle").AsString(500).NotNullable()
            .WithColumn("Reason").AsString(500).NotNullable()
            .WithColumn("SkippedAt").AsDateTime().NotNullable();

        Create.Index("IX_SkippedReleases_ReleaseGuid")
            .OnTable("SkippedReleases")
            .OnColumn("ReleaseGuid");

        Create.Index("IX_SkippedReleases_MediaItemId")
            .OnTable("SkippedReleases")
            .OnColumn("MediaItemId");
    }

    public override void Down()
    {
        Delete.Table("SkippedReleases");
        Delete.Table("GrabbedReleases");
        Delete.Table("SearchHistory");
    }
}
