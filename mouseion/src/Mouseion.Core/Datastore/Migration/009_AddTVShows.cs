// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(9, "Add TV Show tables (Series, Seasons, Episodes, EpisodeFiles, SceneMappings)")]
public class Migration_009_AddTVShows : FluentMigrator.Migration
{
    public override void Up()
    {
        // Series table (TV shows)
        Create.Table("Series")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("TvdbId").AsInt32().Nullable()
            .WithColumn("TmdbId").AsInt32().Nullable()
            .WithColumn("ImdbId").AsString().Nullable()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("CleanTitle").AsString().Nullable()
            .WithColumn("Overview").AsString().Nullable()
            .WithColumn("Status").AsString().NotNullable()
            .WithColumn("AirTime").AsString().Nullable()
            .WithColumn("Network").AsString().Nullable()
            .WithColumn("Runtime").AsInt32().Nullable()
            .WithColumn("Genres").AsString().Nullable()  // JSON array
            .WithColumn("Year").AsInt32().NotNullable()
            .WithColumn("FirstAired").AsDateTime().Nullable()
            .WithColumn("Images").AsString().Nullable()  // JSON array
            .WithColumn("Path").AsString().NotNullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable()
            .WithColumn("SeasonFolder").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("UseSceneNumbering").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Added").AsDateTime().NotNullable()
            .WithColumn("Tags").AsString().Nullable();  // JSON array

        Create.Index("IX_Series_TvdbId").OnTable("Series").OnColumn("TvdbId");
        Create.Index("IX_Series_CleanTitle").OnTable("Series").OnColumn("CleanTitle");

        // Seasons table
        Create.Table("Seasons")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("SeriesId").AsInt32().NotNullable()
            .WithColumn("SeasonNumber").AsInt32().NotNullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true);

        Create.Index("IX_Seasons_SeriesId").OnTable("Seasons").OnColumn("SeriesId");

        // Episodes table
        Create.Table("Episodes")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("SeriesId").AsInt32().NotNullable()
            .WithColumn("SeasonNumber").AsInt32().NotNullable()
            .WithColumn("EpisodeNumber").AsInt32().NotNullable()
            .WithColumn("Title").AsString().Nullable()
            .WithColumn("Overview").AsString().Nullable()
            .WithColumn("AirDate").AsDateTime().Nullable()
            .WithColumn("AirDateUtc").AsDateTime().Nullable()
            .WithColumn("EpisodeFileId").AsInt32().Nullable()
            .WithColumn("AbsoluteEpisodeNumber").AsInt32().Nullable()
            .WithColumn("SceneSeasonNumber").AsInt32().Nullable()
            .WithColumn("SceneEpisodeNumber").AsInt32().Nullable()
            .WithColumn("SceneAbsoluteEpisodeNumber").AsInt32().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Added").AsDateTime().NotNullable();

        Create.Index("IX_Episodes_SeriesId").OnTable("Episodes").OnColumn("SeriesId");
        Create.Index("IX_Episodes_EpisodeFileId").OnTable("Episodes").OnColumn("EpisodeFileId");

        // EpisodeFiles table
        Create.Table("EpisodeFiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("SeriesId").AsInt32().NotNullable()
            .WithColumn("SeasonNumber").AsInt32().NotNullable()
            .WithColumn("RelativePath").AsString().NotNullable()
            .WithColumn("Size").AsInt64().NotNullable()
            .WithColumn("DateAdded").AsDateTime().NotNullable()
            .WithColumn("SceneName").AsString().Nullable()
            .WithColumn("ReleaseGroup").AsString().Nullable()
            .WithColumn("Quality").AsString().Nullable()  // JSON
            .WithColumn("MediaInfo").AsString().Nullable();  // JSON

        Create.Index("IX_EpisodeFiles_SeriesId").OnTable("EpisodeFiles").OnColumn("SeriesId");

        // SceneMappings table for scene numbering
        Create.Table("SceneMappings")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("TvdbId").AsInt32().NotNullable()
            .WithColumn("SeasonNumber").AsInt32().Nullable()
            .WithColumn("SceneSeasonNumber").AsInt32().Nullable()
            .WithColumn("EpisodeNumber").AsInt32().Nullable()
            .WithColumn("SceneEpisodeNumber").AsInt32().Nullable()
            .WithColumn("Title").AsString().Nullable();

        Create.Index("IX_SceneMappings_TvdbId").OnTable("SceneMappings").OnColumn("TvdbId");
    }

    public override void Down()
    {
        Delete.Table("SceneMappings");
        Delete.Table("EpisodeFiles");
        Delete.Table("Episodes");
        Delete.Table("Seasons");
        Delete.Table("Series");
    }
}
