// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(4, "Add hierarchical parent tables")]
public class Migration_004_AddHierarchicalParentTables : FluentMigrator.Migration
{
    public override void Up()
    {
        // Authors table - parent for Books/Audiobooks
        Create.Table("Authors")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("SortName").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("ForeignAuthorId").AsString().Nullable()  // OpenLibrary, Goodreads, etc.
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("Tags").AsString().Nullable();  // JSON array of tag IDs

        Create.Index("IX_Authors_ForeignAuthorId")
            .OnTable("Authors")
            .OnColumn("ForeignAuthorId");

        Create.Index("IX_Authors_Monitored")
            .OnTable("Authors")
            .OnColumn("Monitored");

        // BookSeries table - parent for Books in a series
        Create.Table("BookSeries")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("ForeignSeriesId").AsString().Nullable()
            .WithColumn("AuthorId").AsInt32().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false);

        Create.Index("IX_BookSeries_AuthorId")
            .OnTable("BookSeries")
            .OnColumn("AuthorId");

        // Artists table - parent for Music (Albums/Tracks)
        Create.Table("Artists")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("SortName").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("ForeignArtistId").AsString().Nullable()  // MusicBrainz ID
            .WithColumn("DiscogsId").AsString().Nullable()
            .WithColumn("ArtistType").AsString().Nullable()       // Person, Group, Orchestra, etc.
            .WithColumn("Status").AsString().Nullable()           // Active, Ended, etc.
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("Tags").AsString().Nullable();  // JSON array of tag IDs

        Create.Index("IX_Artists_ForeignArtistId")
            .OnTable("Artists")
            .OnColumn("ForeignArtistId");

        Create.Index("IX_Artists_Monitored")
            .OnTable("Artists")
            .OnColumn("Monitored");

        // TVShows table - parent for TV Episodes
        Create.Table("TVShows")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("TvdbId").AsInt32().Nullable()
            .WithColumn("TmdbId").AsInt32().Nullable()
            .WithColumn("ImdbId").AsString().Nullable()
            .WithColumn("AniDbId").AsInt32().Nullable()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("CleanTitle").AsString().Nullable()
            .WithColumn("Overview").AsString().Nullable()
            .WithColumn("Network").AsString().Nullable()
            .WithColumn("Status").AsInt32().NotNullable()          // Continuing, Ended, etc.
            .WithColumn("Runtime").AsInt32().Nullable()
            .WithColumn("AirTime").AsString().Nullable()
            .WithColumn("Certification").AsString().Nullable()
            .WithColumn("FirstAired").AsDateTime().Nullable()
            .WithColumn("Year").AsInt32().NotNullable()
            .WithColumn("Genres").AsString().Nullable()            // JSON array
            .WithColumn("OriginalLanguage").AsString().Nullable()
            .WithColumn("IsAnime").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("SeriesType").AsInt32().NotNullable().WithDefaultValue(0)  // Standard, Anime, Daily
            .WithColumn("UseSceneNumbering").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable()
            .WithColumn("SeasonFolder").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("MonitorNewItems").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Tags").AsString().Nullable()              // JSON array of tag IDs
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("LastSearchTime").AsDateTime().Nullable();

        Create.Index("IX_TVShows_TvdbId")
            .OnTable("TVShows")
            .OnColumn("TvdbId");

        Create.Index("IX_TVShows_Monitored")
            .OnTable("TVShows")
            .OnColumn("Monitored");

        Create.Index("IX_TVShows_Path")
            .OnTable("TVShows")
            .OnColumn("Path");
    }

    public override void Down()
    {
        Delete.Table("TVShows");
        Delete.Table("Artists");
        Delete.Table("BookSeries");
        Delete.Table("Authors");
    }
}
