// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(10, "Add Podcast tables (Shows, Episodes, PodcastFiles)")]
public class Migration_010_AddPodcastTables : FluentMigrator.Migration
{
    public override void Up()
    {
        // PodcastShows table - podcast show/series metadata
        Create.Table("PodcastShows")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("SortTitle").AsString().Nullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("ForeignPodcastId").AsString().Nullable()  // PodcastIndex GUID
            .WithColumn("ItunesId").AsString().Nullable()
            .WithColumn("Author").AsString().Nullable()
            .WithColumn("FeedUrl").AsString().NotNullable()
            .WithColumn("ImageUrl").AsString().Nullable()
            .WithColumn("Categories").AsString().Nullable()         // JSON array
            .WithColumn("Language").AsString().Nullable()
            .WithColumn("Website").AsString().Nullable()
            .WithColumn("EpisodeCount").AsInt32().Nullable()
            .WithColumn("LatestEpisodeDate").AsDateTime().Nullable()
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("MonitorNewEpisodes").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("RootFolderPath").AsString().Nullable()
            .WithColumn("QualityProfileId").AsInt32().NotNullable().WithDefaultValue(1)
            .WithColumn("Tags").AsString().Nullable()               // JSON array of tag IDs
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("LastSearchTime").AsDateTime().Nullable();

        Create.Index("IX_PodcastShows_ForeignPodcastId")
            .OnTable("PodcastShows")
            .OnColumn("ForeignPodcastId");

        Create.Index("IX_PodcastShows_FeedUrl")
            .OnTable("PodcastShows")
            .OnColumn("FeedUrl");

        Create.Index("IX_PodcastShows_Monitored")
            .OnTable("PodcastShows")
            .OnColumn("Monitored");

        // PodcastEpisodes table - individual podcast episodes
        Create.Table("PodcastEpisodes")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("PodcastShowId").AsInt32().NotNullable()
            .WithColumn("Title").AsString().NotNullable()
            .WithColumn("Description").AsString().Nullable()
            .WithColumn("EpisodeGuid").AsString().Nullable()        // RSS GUID
            .WithColumn("EpisodeNumber").AsInt32().Nullable()
            .WithColumn("SeasonNumber").AsInt32().Nullable()
            .WithColumn("PublishDate").AsDateTime().Nullable()
            .WithColumn("Duration").AsInt32().Nullable()            // Seconds
            .WithColumn("EnclosureUrl").AsString().Nullable()       // Download URL
            .WithColumn("EnclosureLength").AsInt64().Nullable()     // Bytes
            .WithColumn("EnclosureType").AsString().Nullable()      // MIME type
            .WithColumn("ImageUrl").AsString().Nullable()
            .WithColumn("Explicit").AsBoolean().NotNullable().WithDefaultValue(false)
            .WithColumn("Monitored").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime);

        Create.Index("IX_PodcastEpisodes_PodcastShowId")
            .OnTable("PodcastEpisodes")
            .OnColumn("PodcastShowId");

        Create.Index("IX_PodcastEpisodes_EpisodeGuid")
            .OnTable("PodcastEpisodes")
            .OnColumn("EpisodeGuid");

        Create.Index("IX_PodcastEpisodes_PublishDate")
            .OnTable("PodcastEpisodes")
            .OnColumn("PublishDate");

        // PodcastFiles table - physical podcast episode files
        Create.Table("PodcastFiles")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("PodcastEpisodeId").AsInt32().NotNullable()
            .WithColumn("PodcastShowId").AsInt32().Nullable()
            .WithColumn("RelativePath").AsString().NotNullable()
            .WithColumn("Path").AsString().Nullable()
            .WithColumn("Size").AsInt64().NotNullable().WithDefaultValue(0)
            .WithColumn("DateAdded").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentDateTime)
            .WithColumn("SceneName").AsString().Nullable()
            .WithColumn("ReleaseGroup").AsString().Nullable()
            .WithColumn("Quality").AsString().Nullable()            // JSON QualityModel
            .WithColumn("AudioFormat").AsString().Nullable()        // MP3, AAC, etc.
            .WithColumn("Bitrate").AsInt32().Nullable()             // kbps
            .WithColumn("SampleRate").AsInt32().Nullable()          // Hz
            .WithColumn("Channels").AsInt32().Nullable()            // 1 (mono), 2 (stereo)
            .WithColumn("Duration").AsInt32().Nullable();           // Seconds

        Create.Index("IX_PodcastFiles_PodcastEpisodeId")
            .OnTable("PodcastFiles")
            .OnColumn("PodcastEpisodeId");

        Create.Index("IX_PodcastFiles_PodcastShowId")
            .OnTable("PodcastFiles")
            .OnColumn("PodcastShowId");

        Create.Index("IX_PodcastFiles_RelativePath")
            .OnTable("PodcastFiles")
            .OnColumn("RelativePath");
    }

    public override void Down()
    {
        Delete.Table("PodcastFiles");
        Delete.Table("PodcastEpisodes");
        Delete.Table("PodcastShows");
    }
}
