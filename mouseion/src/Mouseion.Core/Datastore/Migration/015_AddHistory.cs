// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(15, "Add History table")]
public class Migration015AddHistory : FluentMigrator.Migration
{
    private const string TableName = "History";

    public override void Up()
    {
        Create.Table(TableName)
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("MediaItemId").AsInt32().NotNullable().Indexed()
            .WithColumn("MediaType").AsInt32().NotNullable()
            .WithColumn("SourceTitle").AsString().NotNullable()
            .WithColumn("Quality").AsString().NotNullable()
            .WithColumn("Date").AsDateTime().NotNullable()
            .WithColumn("EventType").AsInt32().NotNullable()
            .WithColumn("Data").AsString().NotNullable().WithDefaultValue("{}")
            .WithColumn("DownloadId").AsString().Nullable();

        Create.Index("IX_History_MediaItemId_Date")
            .OnTable(TableName)
            .OnColumn("MediaItemId").Ascending()
            .OnColumn("Date").Descending();

        Create.Index("IX_History_DownloadId_Date")
            .OnTable(TableName)
            .OnColumn("DownloadId").Ascending()
            .OnColumn("Date").Descending();
    }

    public override void Down()
    {
        Delete.Table(TableName);
    }
}
