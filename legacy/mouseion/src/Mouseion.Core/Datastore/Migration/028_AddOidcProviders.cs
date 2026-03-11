// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentMigrator;

namespace Mouseion.Core.Datastore.Migration;

[Migration(028, "Add OIDC provider configuration and state tables")]
public class Migration028AddOidcProviders : FluentMigrator.Migration
{
    public override void Up()
    {
        Create.Table("OidcProviders")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("Name").AsString(200).NotNullable()
            .WithColumn("Slug").AsString(100).NotNullable()
            .WithColumn("IssuerUrl").AsString(1024).NotNullable()
            .WithColumn("ClientId").AsString(512).NotNullable()
            .WithColumn("ClientSecret").AsString(1024).NotNullable().WithDefaultValue("")
            .WithColumn("Scopes").AsString(512).NotNullable().WithDefaultValue("openid profile email")
            .WithColumn("ClaimRoleMappings").AsString(int.MaxValue).NotNullable().WithDefaultValue("{}")
            .WithColumn("RoleClaimType").AsString(200).NotNullable().WithDefaultValue("role")
            .WithColumn("DefaultRole").AsInt32().NotNullable().WithDefaultValue(1) // UserRole.User
            .WithColumn("AutoProvisionUsers").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("Enabled").AsBoolean().NotNullable().WithDefaultValue(true)
            .WithColumn("CreatedAt").AsDateTime().NotNullable()
            .WithColumn("UpdatedAt").AsDateTime().NotNullable();

        Create.Index("IX_OidcProviders_Slug")
            .OnTable("OidcProviders")
            .OnColumn("Slug")
            .Unique();

        Create.Index("IX_OidcProviders_Enabled")
            .OnTable("OidcProviders")
            .OnColumn("Enabled");

        // OIDC state for PKCE flow validation (short-lived)
        Create.Table("OidcAuthStates")
            .WithColumn("Id").AsInt32().PrimaryKey().Identity()
            .WithColumn("State").AsString(512).NotNullable()
            .WithColumn("CodeVerifier").AsString(512).NotNullable()
            .WithColumn("ProviderSlug").AsString(100).NotNullable()
            .WithColumn("ReturnUrl").AsString(2048).Nullable()
            .WithColumn("ExpiresAt").AsDateTime().NotNullable()
            .WithColumn("CreatedAt").AsDateTime().NotNullable();

        Create.Index("IX_OidcAuthStates_State")
            .OnTable("OidcAuthStates")
            .OnColumn("State")
            .Unique();

        Create.Index("IX_OidcAuthStates_ExpiresAt")
            .OnTable("OidcAuthStates")
            .OnColumn("ExpiresAt");
    }

    public override void Down()
    {
        Delete.Table("OidcAuthStates");
        Delete.Table("OidcProviders");
    }
}
