// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(035)]
public class AddOidcProviderIdToUsers : FluentMigrator.Migration
{
    public override void Up()
    {
        Alter.Table("Users")
            .AddColumn("OidcProviderId").AsInt32().Nullable()
            .AddColumn("OidcSubject").AsString(500).Nullable();
    }

    public override void Down()
    {
        Delete.Column("OidcSubject").FromTable("Users");
        Delete.Column("OidcProviderId").FromTable("Users");
    }
}
