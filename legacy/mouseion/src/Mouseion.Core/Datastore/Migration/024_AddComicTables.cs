// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(24, "Add Comic tables")]
public class Migration024AddComicTables : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("ComicSeries")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("ComicVineId").AsInt32().Nullable()
            .WithColumn("Publisher").AsString().Nullable()
            .WithColumn("Imprint").AsString().Nullable()
            .WithColumn("StartYear").AsInt32().Nullable()
            .WithColumn("EndYear").AsInt32().Nullable()
            .WithColumn("Status").AsString().Nullable()
            .WithColumn("IssueCount").AsInt32().Nullable()
            .WithColumn("VolumeNumber").AsInt32().Nullable()
            .WithColumn("Genres").AsString().Nullable()
            .WithColumn("Characters").AsString().Nullable()
            .WithColumn("CoverUrl").AsString().Nullable()
            .WithColumn("SiteUrl").AsString().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Table("ComicIssues")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("ComicSeriesId").AsInt32().NotNullable()
            .WithColumn("Title").AsString().Nullable()
            .WithColumn("IssueNumber").AsString().Nullable()
            .WithColumn("ComicVineIssueId").AsInt32().Nullable()
            .WithColumn("StoryArc").AsString().Nullable()
            .WithColumn("Writer").AsString().Nullable()
            .WithColumn("Penciler").AsString().Nullable()
            .WithColumn("Inker").AsString().Nullable()
            .WithColumn("Colorist").AsString().Nullable()
            .WithColumn("CoverArtist").AsString().Nullable()
            .WithColumn("CoverDate").AsDateTime().Nullable()
            .WithColumn("StoreDate").AsDateTime().Nullable()
            .WithColumn("PageCount").AsInt32().Nullable()
            .WithColumn("CoverUrl").AsString().Nullable()
            .WithColumn("SiteUrl").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("IsRead").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("IsDownloaded").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("FilePath").AsString().Nullable()
            .WithColumn("FileFormat").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable();

        // ComicSeries indexes
        Create.Index("IX_ComicSeries_ComicVineId")
            .OnTable("ComicSeries")
            .OnColumn("ComicVineId");

        Create.Index("IX_ComicSeries_Publisher")
            .OnTable("ComicSeries")
            .OnColumn("Publisher");

        Create.Index("IX_ComicSeries_Monitored")
            .OnTable("ComicSeries")
            .OnColumn("Monitored");

        // ComicIssues indexes
        Create.Index("IX_ComicIssues_ComicSeriesId")
            .OnTable("ComicIssues")
            .OnColumn("ComicSeriesId");

        Create.Index("IX_ComicIssues_ComicVineIssueId")
            .OnTable("ComicIssues")
            .OnColumn("ComicVineIssueId");

        Create.Index("IX_ComicIssues_IsRead")
            .OnTable("ComicIssues")
            .OnColumn("IsRead");

        Create.Index("IX_ComicIssues_IssueNumber")
            .OnTable("ComicIssues")
            .OnColumn("IssueNumber");

        // Note: Foreign key constraints omitted for SQLite compatibility
        // Cascade delete handled at application level
    }

    public override void Down()
    {
        Delete.Table("ComicIssues");
        Delete.Table("ComicSeries");
    }
}
