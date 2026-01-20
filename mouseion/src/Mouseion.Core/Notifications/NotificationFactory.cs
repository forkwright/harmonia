// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications;

public interface INotificationFactory
{
    INotification Create(NotificationDefinition definition);
    IEnumerable<NotificationProviderInfo> GetAvailableProviders();
}

public record NotificationProviderInfo(string Implementation, string Name, string Link);

public class NotificationFactory : INotificationFactory
{
    private readonly IServiceProvider _serviceProvider;
    private readonly ILogger<NotificationFactory> _logger;

    private static readonly Dictionary<string, Type> ProviderTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        ["Discord"] = typeof(Discord.Discord),
        ["Slack"] = typeof(Slack.Slack),
        ["Telegram"] = typeof(Telegram.Telegram),
        ["Email"] = typeof(Email.Email),
        ["Gotify"] = typeof(Gotify.Gotify),
        ["Apprise"] = typeof(Apprise.Apprise)
    };

    public NotificationFactory(IServiceProvider serviceProvider, ILogger<NotificationFactory> logger)
    {
        _serviceProvider = serviceProvider;
        _logger = logger;
    }

    public INotification Create(NotificationDefinition definition)
    {
        if (!ProviderTypes.TryGetValue(definition.Implementation, out var providerType))
        {
            throw new InvalidOperationException($"Unknown notification implementation: {definition.Implementation}");
        }

        var settingsType = GetSettingsType(providerType);
        var settings = DeserializeSettings(definition.Settings, settingsType);

        // Copy common settings from definition to settings object
        CopyCommonSettings(definition, settings);

        var notification = (INotification)Activator.CreateInstance(providerType, settings)!;
        return notification;
    }

    public IEnumerable<NotificationProviderInfo> GetAvailableProviders()
    {
        foreach (var (impl, type) in ProviderTypes)
        {
            var instance = (INotification)Activator.CreateInstance(type, CreateDefaultSettings(type))!;
            yield return new NotificationProviderInfo(impl, instance.Name, instance.Link);
        }
    }

    private static Type GetSettingsType(Type providerType)
    {
        var baseType = providerType.BaseType;
        while (baseType != null)
        {
            if (baseType.IsGenericType && baseType.GetGenericTypeDefinition() == typeof(NotificationBase<>))
            {
                return baseType.GetGenericArguments()[0];
            }
            baseType = baseType.BaseType;
        }

        throw new InvalidOperationException($"Provider {providerType.Name} does not inherit from NotificationBase<TSettings>");
    }

    private static NotificationSettings DeserializeSettings(string? json, Type settingsType)
    {
        if (string.IsNullOrEmpty(json))
        {
            return (NotificationSettings)Activator.CreateInstance(settingsType)!;
        }

        try
        {
            return (NotificationSettings)JsonSerializer.Deserialize(json, settingsType)!;
        }
        catch
        {
            return (NotificationSettings)Activator.CreateInstance(settingsType)!;
        }
    }

    private static void CopyCommonSettings(NotificationDefinition definition, NotificationSettings settings)
    {
        settings.Id = definition.Id;
        settings.Name = definition.Name;
        settings.Enabled = definition.Enabled;
        settings.OnGrab = definition.OnGrab;
        settings.OnDownload = definition.OnDownload;
        settings.OnRename = definition.OnRename;
        settings.OnMediaAdded = definition.OnMediaAdded;
        settings.OnMediaDeleted = definition.OnMediaDeleted;
        settings.OnHealthIssue = definition.OnHealthIssue;
        settings.OnHealthRestored = definition.OnHealthRestored;
        settings.OnApplicationUpdate = definition.OnApplicationUpdate;
    }

    private static NotificationSettings CreateDefaultSettings(Type providerType)
    {
        var settingsType = GetSettingsType(providerType);
        return (NotificationSettings)Activator.CreateInstance(settingsType)!;
    }
}
