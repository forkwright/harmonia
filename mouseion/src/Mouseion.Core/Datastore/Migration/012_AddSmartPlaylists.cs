// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(12)]
public class Migration012AddSmartPlaylists : FluentMigrator.Migration
{
    private const string TableSmartPlaylists = "SmartPlaylists";
    private const string TableSmartPlaylistTracks = "SmartPlaylistTracks";
    private const string ColumnSmartPlaylistId = "SmartPlaylistId";

    public override void Up()
    {
        Create.Table(TableSmartPlaylists)
            .WithColumn("Id").AsString().PrimaryKey()
            .WithColumn("Name").AsString().NotNullable()
            .WithColumn("FilterRequestJson").AsString().NotNullable()
            .WithColumn("TrackCount").AsInt32().NotNullable().WithDefaultValue(0)
            .WithColumn("LastRefreshed").AsString().NotNullable()
            .WithColumn("CreatedAt").AsString().NotNullable()
            .WithColumn("UpdatedAt").AsString().NotNullable();

        Create.Table(TableSmartPlaylistTracks)
            .WithColumn(ColumnSmartPlaylistId).AsString().NotNullable()
            .WithColumn("TrackId").AsString().NotNullable()
            .WithColumn("Position").AsInt32().NotNullable();

        Create.PrimaryKey("PK_SmartPlaylistTracks")
            .OnTable(TableSmartPlaylistTracks)
            .Columns(ColumnSmartPlaylistId, "TrackId");

        Create.Index("idx_smart_playlist_tracks_playlist")
            .OnTable(TableSmartPlaylistTracks)
            .OnColumn(ColumnSmartPlaylistId);

        Create.Index("idx_smart_playlist_tracks_position")
            .OnTable(TableSmartPlaylistTracks)
            .OnColumn(ColumnSmartPlaylistId)
            .Ascending()
            .OnColumn("Position")
            .Ascending();
    }

    public override void Down()
    {
        Delete.Table(TableSmartPlaylistTracks);
        Delete.Table(TableSmartPlaylists);
    }
}
