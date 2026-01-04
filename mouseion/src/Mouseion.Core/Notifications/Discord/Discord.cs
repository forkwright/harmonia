// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications.Discord
{
    /// <summary>
    /// Discord webhook notification implementation
    /// </summary>
    public class Discord : NotificationBase<DiscordSettings>
    {
        private readonly ILogger<Discord> _logger;
        private readonly HttpClient _httpClient;

        public Discord(DiscordSettings settings, ILogger<Discord> logger, IHttpClientFactory httpClientFactory)
            : base(settings)
        {
            _logger = logger;
            _httpClient = httpClientFactory.CreateClient();
        }

        public override string Name => "Discord";
        public override string Link => "https://support.discord.com/hc/en-us/articles/228383668-Intro-to-Webhooks";

        public override async Task<bool> TestAsync()
        {
            try
            {
                var payload = new DiscordPayload
                {
                    Username = Settings.Username,
                    AvatarUrl = Settings.AvatarUrl,
                    Content = $"Test notification from Mouseion at {DateTime.Now:yyyy-MM-dd HH:mm:ss}"
                };

                await SendPayloadAsync(payload);
                return true;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Discord test notification failed");
                return false;
            }
        }

        public override async Task OnGrabAsync(GrabMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = message.Title,
                Description = "Media Grabbed",
                Color = DiscordColors.Standard,
                Fields = new List<DiscordField>
                {
                    new() { Name = "Type", Value = message.MediaType, Inline = true },
                    new() { Name = "Quality", Value = message.Quality, Inline = true },
                    new() { Name = "Size", Value = FormatBytes(message.SizeBytes), Inline = true },
                    new() { Name = "Release Group", Value = message.ReleaseGroup ?? "Unknown", Inline = true },
                    new() { Name = "Indexer", Value = message.Indexer ?? "Unknown", Inline = true },
                    new() { Name = "Download Client", Value = message.DownloadClient ?? "Unknown", Inline = true }
                },
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            await SendEmbedAsync(embed);
        }

        public override async Task OnDownloadAsync(DownloadMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = message.Title,
                Description = message.IsUpgrade ? "Media Upgraded" : "Media Downloaded",
                Color = message.IsUpgrade ? DiscordColors.Upgrade : DiscordColors.Success,
                Fields = new List<DiscordField>
                {
                    new() { Name = "Type", Value = message.MediaType, Inline = true },
                    new() { Name = "Quality", Value = message.Quality, Inline = true },
                    new() { Name = "Size", Value = FormatBytes(message.SizeBytes), Inline = true },
                    new() { Name = "File", Value = $"```{message.FilePath}```", Inline = false }
                },
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            await SendEmbedAsync(embed);
        }

        public override async Task OnRenameAsync(RenameMessage message)
        {
            var embed = new DiscordEmbed
            {
                Title = message.Title,
                Description = "Media Renamed",
                Color = DiscordColors.Standard,
                Fields = new List<DiscordField>
                {
                    new() { Name = "Old Path", Value = $"```{message.OldPath}```", Inline = false },
                    new() { Name = "New Path", Value = $"```{message.NewPath}```", Inline = false }
                },
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            await SendEmbedAsync(embed);
        }

        public override async Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = message.Title,
                Description = "Media Added to Library",
                Color = DiscordColors.Success,
                Fields = new List<DiscordField>
                {
                    new() { Name = "Type", Value = message.MediaType, Inline = true },
                    new() { Name = "Year", Value = message.Year.ToString(), Inline = true }
                },
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            if (!string.IsNullOrWhiteSpace(message.Overview))
            {
                var overview = message.Overview.Length <= 300
                    ? message.Overview
                    : message.Overview.Substring(0, 297) + "...";
                embed.Fields.Add(new DiscordField { Name = "Overview", Value = overview, Inline = false });
            }

            await SendEmbedAsync(embed);
        }

        public override async Task OnMediaDeletedAsync(MediaDeletedMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = message.Title,
                Description = "Media Deleted from Library",
                Color = DiscordColors.Danger,
                Fields = new List<DiscordField>
                {
                    new() { Name = "Type", Value = message.MediaType, Inline = true },
                    new() { Name = "Files Deleted", Value = message.DeletedFiles ? "Yes" : "No", Inline = true },
                    new() { Name = "Reason", Value = message.Reason ?? "Unknown", Inline = true }
                },
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            await SendEmbedAsync(embed);
        }

        public override async Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = $"Health Check: {message.Source}",
                Description = message.Message,
                Color = message.Type == "Error" ? DiscordColors.Danger : DiscordColors.Warning,
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            if (!string.IsNullOrWhiteSpace(message.WikiUrl))
            {
                embed.Fields = new List<DiscordField>
                {
                    new() { Name = "More Info", Value = message.WikiUrl, Inline = false }
                };
            }

            await SendEmbedAsync(embed);
        }

        public override async Task OnHealthRestoredAsync(HealthRestoredMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = $"Health Restored: {message.Source}",
                Description = $"The following issue is now resolved: {message.PreviousMessage}",
                Color = DiscordColors.Success,
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            await SendEmbedAsync(embed);
        }

        public override async Task OnApplicationUpdateAsync(ApplicationUpdateMessage message)
        {
            var embed = new DiscordEmbed
            {
                Author = new DiscordAuthor
                {
                    Name = Settings.Author ?? "Mouseion",
                    IconUrl = "https://raw.githubusercontent.com/forkwright/mouseion/develop/Logo/256.png"
                },
                Title = "Application Updated",
                Description = $"Mouseion updated from {message.PreviousVersion} to {message.NewVersion}",
                Color = DiscordColors.Standard,
                Timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            };

            if (!string.IsNullOrWhiteSpace(message.ReleaseNotes))
            {
                embed.Fields = new List<DiscordField>
                {
                    new() { Name = "Release Notes", Value = message.ReleaseNotes, Inline = false }
                };
            }

            await SendEmbedAsync(embed);
        }

        private Task SendEmbedAsync(DiscordEmbed embed)
        {
            var payload = new DiscordPayload
            {
                Username = Settings.Username,
                AvatarUrl = Settings.AvatarUrl,
                Embeds = new List<DiscordEmbed> { embed }
            };

            return SendPayloadAsync(payload);
        }

        private async Task SendPayloadAsync(DiscordPayload payload)
        {
            var json = JsonSerializer.Serialize(payload, new JsonSerializerOptions
            {
                DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull
            });
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await _httpClient.PostAsync(Settings.WebhookUrl, content);
            response.EnsureSuccessStatusCode();

            _logger.LogDebug("Discord notification sent successfully");
        }
    }
}
