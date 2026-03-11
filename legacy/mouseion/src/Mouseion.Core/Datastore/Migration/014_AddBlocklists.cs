// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(14, "Add Blocklists table")]
public class Migration014AddBlocklists : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("Blocklists")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable().Indexed()
            .WithColumn("SourceTitle").AsString().NotNullable()
            .WithColumn("Quality").AsString().NotNullable()
            .WithColumn("Date").AsDateTime().NotNullable()
            .WithColumn("PublishedDate").AsDateTime().Nullable()
            .WithColumn("Size").AsInt64().Nullable()
            .WithColumn("Protocol").AsInt32().NotNullable()
            .WithColumn("Indexer").AsString().Nullable()
            .WithColumn("Message").AsString().Nullable()
            .WithColumn("TorrentInfoHash").AsString().Nullable().Indexed();
    }

    public override void Down()
    {
        Delete.Table("Blocklists");
    }
}
