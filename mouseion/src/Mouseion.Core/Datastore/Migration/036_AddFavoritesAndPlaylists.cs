// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(036)]
public class AddFavoritesAndPlaylists : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("Favorites")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentUTCDateTime);

        Create.Index("IX_Favorites_UserId_MediaItemId")
            .OnTable("Favorites")
            .OnColumn("UserId").Ascending()
            .OnColumn("MediaItemId").Ascending()
            .WithOptions().Unique();

        Create.Table("Playlists")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("UserId").AsInt32().NotNullable()
            .WithColumn("Name").AsString(500).NotNullable()
            .WithColumn("Description").AsString(2000).Nullable()
            .WithColumn("Created").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentUTCDateTime)
            .WithColumn("Modified").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentUTCDateTime);

        Create.Table("PlaylistTracks")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("PlaylistId").AsInt32().NotNullable()
            .WithColumn("MediaItemId").AsInt32().NotNullable()
            .WithColumn("Position").AsInt32().NotNullable()
            .WithColumn("Added").AsDateTime().NotNullable().WithDefault(SystemMethods.CurrentUTCDateTime);

        Create.Index("IX_PlaylistTracks_PlaylistId")
            .OnTable("PlaylistTracks")
            .OnColumn("PlaylistId").Ascending();
    }

    public override void Down()
    {
        Delete.Table("PlaylistTracks");
        Delete.Table("Playlists");
        Delete.Table("Favorites");
    }
}
