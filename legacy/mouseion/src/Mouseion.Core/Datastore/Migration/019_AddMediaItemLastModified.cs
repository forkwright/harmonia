// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(19)]
public class AddMediaItemLastModified : FluentMigrator.Migration
{
    public override void Up()
    {
        Alter.Table("MediaItems")
            .AddColumn("LastModified").AsDateTime().Nullable();

        // Set initial LastModified to Added date for existing records
        Execute.Sql("UPDATE \"MediaItems\" SET \"LastModified\" = \"Added\"");
    }

    public override void Down()
    {
        Delete.Column("LastModified").FromTable("MediaItems");
    }
}
