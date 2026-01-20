// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(23, "Add Manga and Webcomic tables")]
public class Migration023AddMangaWebcomicTables : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("MangaSeries")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("MangaDexId").AsString().Nullable()
            .WithColumn("AniListId").AsInt32().Nullable()
            .WithColumn("MyAnimeListId").AsInt32().Nullable()
            .WithColumn("Author").AsString().Nullable()
            .WithColumn("Artist").AsString().Nullable()
            .WithColumn("Status").AsString().Nullable()
            .WithColumn("Year").AsInt32().Nullable()
            .WithColumn("OriginalLanguage").AsString().Nullable()
            .WithColumn("ContentRating").AsString().Nullable()
            .WithColumn("Genres").AsString().Nullable()
            .WithColumn("Tags").AsString().Nullable()
            .WithColumn("CoverUrl").AsString().Nullable()
            .WithColumn("LastChapterNumber").AsDecimal().Nullable()
            .WithColumn("LastVolumeNumber").AsInt32().Nullable()
            .WithColumn("ChapterCount").AsInt32().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Table("MangaChapters")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MangaSeriesId").AsInt32().NotNullable()
            .WithColumn("Title").AsString().Nullable()
            .WithColumn("ChapterNumber").AsDecimal().Nullable()
            .WithColumn("VolumeNumber").AsInt32().Nullable()
            .WithColumn("MangaDexChapterId").AsString().Nullable()
            .WithColumn("ScanlationGroup").AsString().Nullable()
            .WithColumn("TranslatedLanguage").AsString().Nullable()
            .WithColumn("PageCount").AsInt32().Nullable()
            .WithColumn("ExternalUrl").AsString().Nullable()
            .WithColumn("PublishDate").AsDateTime().Nullable()
            .WithColumn("IsRead").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("IsDownloaded").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("FilePath").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Table("WebcomicSeries")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("WebtoonId").AsString().Nullable()
            .WithColumn("TapasId").AsString().Nullable()
            .WithColumn("Author").AsString().Nullable()
            .WithColumn("Artist").AsString().Nullable()
            .WithColumn("Status").AsString().Nullable()
            .WithColumn("Platform").AsString().Nullable()
            .WithColumn("UpdateSchedule").AsString().Nullable()
            .WithColumn("Genres").AsString().Nullable()
            .WithColumn("Tags").AsString().Nullable()
            .WithColumn("CoverUrl").AsString().Nullable()
            .WithColumn("ThumbnailUrl").AsString().Nullable()
            .WithColumn("SiteUrl").AsString().Nullable()
            .WithColumn("LastEpisodeNumber").AsInt32().Nullable()
            .WithColumn("EpisodeCount").AsInt32().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Table("WebcomicEpisodes")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("WebcomicSeriesId").AsInt32().NotNullable()
            .WithColumn("Title").AsString().Nullable()
            .WithColumn("EpisodeNumber").AsInt32().Nullable()
            .WithColumn("SeasonNumber").AsInt32().Nullable()
            .WithColumn("ExternalId").AsString().Nullable()
            .WithColumn("ExternalUrl").AsString().Nullable()
            .WithColumn("ThumbnailUrl").AsString().Nullable()
            .WithColumn("PublishDate").AsDateTime().Nullable()
            .WithColumn("IsRead").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("IsDownloaded").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("FilePath").AsString().Nullable()
            .WithColumn("Added").AsDateTime().NotNullable();

        // MangaSeries indexes
        Create.Index("IX_MangaSeries_MangaDexId")
            .OnTable("MangaSeries")
            .OnColumn("MangaDexId");

        Create.Index("IX_MangaSeries_AniListId")
            .OnTable("MangaSeries")
            .OnColumn("AniListId");

        Create.Index("IX_MangaSeries_Monitored")
            .OnTable("MangaSeries")
            .OnColumn("Monitored");

        // MangaChapters indexes
        Create.Index("IX_MangaChapters_MangaSeriesId")
            .OnTable("MangaChapters")
            .OnColumn("MangaSeriesId");

        Create.Index("IX_MangaChapters_MangaDexChapterId")
            .OnTable("MangaChapters")
            .OnColumn("MangaDexChapterId");

        Create.Index("IX_MangaChapters_IsRead")
            .OnTable("MangaChapters")
            .OnColumn("IsRead");

        Create.Index("IX_MangaChapters_ChapterNumber")
            .OnTable("MangaChapters")
            .OnColumn("ChapterNumber");

        // WebcomicSeries indexes
        Create.Index("IX_WebcomicSeries_WebtoonId")
            .OnTable("WebcomicSeries")
            .OnColumn("WebtoonId");

        Create.Index("IX_WebcomicSeries_TapasId")
            .OnTable("WebcomicSeries")
            .OnColumn("TapasId");

        Create.Index("IX_WebcomicSeries_Monitored")
            .OnTable("WebcomicSeries")
            .OnColumn("Monitored");

        // WebcomicEpisodes indexes
        Create.Index("IX_WebcomicEpisodes_WebcomicSeriesId")
            .OnTable("WebcomicEpisodes")
            .OnColumn("WebcomicSeriesId");

        Create.Index("IX_WebcomicEpisodes_IsRead")
            .OnTable("WebcomicEpisodes")
            .OnColumn("IsRead");

        Create.Index("IX_WebcomicEpisodes_EpisodeNumber")
            .OnTable("WebcomicEpisodes")
            .OnColumn("EpisodeNumber");

        // Note: Foreign key constraints omitted for SQLite compatibility
        // Cascade delete handled at application level
    }

    public override void Down()
    {
        Delete.Table("WebcomicEpisodes");
        Delete.Table("WebcomicSeries");
        Delete.Table("MangaChapters");
        Delete.Table("MangaSeries");
    }
}
