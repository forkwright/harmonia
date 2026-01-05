// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Notifications.Slack
{
    /// <summary>
    /// Slack webhook notification implementation
    /// </summary>
    public class Slack : NotificationBase<SlackSettings>
    {
        private readonly ILogger<Slack> _logger;
        private readonly HttpClient _httpClient;

        public Slack(SlackSettings settings, ILogger<Slack> logger, IHttpClientFactory httpClientFactory)
            : base(settings)
        {
            _logger = logger;
            _httpClient = httpClientFactory.CreateClient();
        }

        public override string Name => "Slack";
        public override string Link => "https://api.slack.com/messaging/webhooks";

        public override async Task<bool> TestAsync()
        {
            try
            {
                var payload = new SlackPayload
                {
                    Username = Settings.Username ?? "Mouseion",
                    IconEmoji = Settings.IconEmoji,
                    IconUrl = Settings.IconUrl,
                    Channel = Settings.Channel,
                    Text = $"Test notification from Mouseion at {DateTime.Now:yyyy-MM-dd HH:mm:ss}"
                };

                await SendPayloadAsync(payload);
                return true;
            }
            catch (HttpRequestException ex)
            {
                _logger.LogError(ex, "Network error sending Slack test notification");
                return false;
            }
            catch (TaskCanceledException ex)
            {
                _logger.LogWarning(ex, "Request timed out or was cancelled sending Slack test notification");
                return false;
            }
        }

        public override async Task OnGrabAsync(GrabMessage message)
        {
            var attachment = new SlackAttachment
            {
                Color = SlackColors.Standard,
                Title = message.Title,
                Text = "Media Grabbed",
                Fields = new List<SlackField>
                {
                    new() { Title = "Type", Value = message.MediaType, Short = true },
                    new() { Title = "Quality", Value = message.Quality, Short = true },
                    new() { Title = "Size", Value = FormatBytes(message.SizeBytes), Short = true },
                    new() { Title = "Release Group", Value = message.ReleaseGroup ?? "Unknown", Short = true },
                    new() { Title = "Indexer", Value = message.Indexer ?? "Unknown", Short = true },
                    new() { Title = "Download Client", Value = message.DownloadClient ?? "Unknown", Short = true }
                },
                Footer = "Mouseion",
                Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            };

            await SendAttachmentAsync(attachment);
        }

        public override async Task OnDownloadAsync(DownloadMessage message)
        {
            var attachment = new SlackAttachment
            {
                Color = message.IsUpgrade ? SlackColors.Upgrade : SlackColors.Good,
                Title = message.Title,
                Text = message.IsUpgrade ? "Media Upgraded" : "Media Downloaded",
                Fields = new List<SlackField>
                {
                    new() { Title = "Type", Value = message.MediaType, Short = true },
                    new() { Title = "Quality", Value = message.Quality, Short = true },
                    new() { Title = "Size", Value = FormatBytes(message.SizeBytes), Short = true },
                    new() { Title = "File", Value = message.FilePath, Short = false }
                },
                Footer = "Mouseion",
                Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            };

            await SendAttachmentAsync(attachment);
        }

        public override async Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            var attachment = new SlackAttachment
            {
                Color = SlackColors.Good,
                Title = message.Title,
                Text = "Media Added to Library",
                Fields = new List<SlackField>
                {
                    new() { Title = "Type", Value = message.MediaType, Short = true },
                    new() { Title = "Year", Value = message.Year.ToString(), Short = true }
                },
                Footer = "Mouseion",
                Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            };

            if (!string.IsNullOrWhiteSpace(message.Overview))
            {
                var overview = message.Overview.Length <= 300
                    ? message.Overview
                    : message.Overview.Substring(0, 297) + "...";
                attachment.Fields.Add(new SlackField { Title = "Overview", Value = overview, Short = false });
            }

            await SendAttachmentAsync(attachment);
        }

        public override async Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            var attachment = new SlackAttachment
            {
                Color = message.Type == "Error" ? SlackColors.Danger : SlackColors.Warning,
                Title = $"Health Check: {message.Source}",
                Text = message.Message,
                Footer = "Mouseion",
                Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            };

            if (!string.IsNullOrWhiteSpace(message.WikiUrl))
            {
                attachment.Fields = new List<SlackField>
                {
                    new() { Title = "More Info", Value = message.WikiUrl, Short = false }
                };
            }

            await SendAttachmentAsync(attachment);
        }

        private Task SendAttachmentAsync(SlackAttachment attachment)
        {
            var payload = new SlackPayload
            {
                Username = Settings.Username ?? "Mouseion",
                IconEmoji = Settings.IconEmoji,
                IconUrl = Settings.IconUrl,
                Channel = Settings.Channel,
                Attachments = new List<SlackAttachment> { attachment }
            };

            return SendPayloadAsync(payload);
        }

        private async Task SendPayloadAsync(SlackPayload payload)
        {
            var json = JsonSerializer.Serialize(payload, new JsonSerializerOptions
            {
                DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull
            });
            var content = new StringContent(json, Encoding.UTF8, "application/json");

            var response = await _httpClient.PostAsync(Settings.WebhookUrl, content);
            response.EnsureSuccessStatusCode();

            _logger.LogDebug("Slack notification sent successfully");
        }
    }
}
