// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(18)]
public class AddMediaFileHash : FluentMigrator.Migration
{
    public override void Up()
    {
        Alter.Table("MediaFiles")
            .AddColumn("FileHash").AsString(64).Nullable();
    }

    public override void Down()
    {
        Delete.Column("FileHash").FromTable("MediaFiles");
    }
}
