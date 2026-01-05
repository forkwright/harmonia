// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Linq;
using System.Net.Sockets;
using System.Threading.Tasks;
using MailKit.Net.Smtp;
using MailKit.Security;
using Microsoft.Extensions.Logging;
using MimeKit;

namespace Mouseion.Core.Notifications.Email
{
    /// <summary>
    /// Email notification implementation (SMTP)
    /// </summary>
    public class Email : NotificationBase<EmailSettings>
    {
        private readonly ILogger<Email> _logger;

        public Email(EmailSettings settings, ILogger<Email> logger)
            : base(settings)
        {
            _logger = logger;
        }

        public override string Name => "Email";
        public override string Link => "https://en.wikipedia.org/wiki/Email";

        public override async Task<bool> TestAsync()
        {
            try
            {
                await SendEmailAsync(
                    "Mouseion Test Notification",
                    $"This is a test email from Mouseion sent at {DateTime.Now:yyyy-MM-dd HH:mm:ss}"
                );
                return true;
            }
            catch (SocketException ex)
            {
                _logger.LogError(ex, "Network error sending email test notification");
                return false;
            }
            catch (AuthenticationException ex)
            {
                _logger.LogError(ex, "SMTP authentication failed during email test");
                return false;
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Invalid email configuration");
                return false;
            }
            catch (TaskCanceledException ex)
            {
                _logger.LogWarning(ex, "Request timed out or was cancelled sending email test notification");
                return false;
            }
        }

        public override Task OnGrabAsync(GrabMessage message)
        {
            var subject = $"Mouseion - Media Grabbed: {message.Title}";
            var body = $"Media Grabbed\n\n" +
                       $"Title: {message.Title}\n" +
                       $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}\n" +
                       $"Release Group: {message.ReleaseGroup ?? "Unknown"}\n" +
                       $"Indexer: {message.Indexer ?? "Unknown"}\n" +
                       $"Download Client: {message.DownloadClient ?? "Unknown"}";
            return SendEmailAsync(subject, body);
        }

        public override Task OnDownloadAsync(DownloadMessage message)
        {
            var status = message.IsUpgrade ? "upgraded" : "downloaded";
            var subject = $"Mouseion - Media {status}: {message.Title}";
            var body = $"Media {status}\n\n" +
                       $"Title: {message.Title}\n" +
                       $"Type: {message.MediaType}\n" +
                       $"Quality: {message.Quality}\n" +
                       $"Size: {FormatBytes(message.SizeBytes)}\n" +
                       $"File: {message.FilePath}";
            return SendEmailAsync(subject, body);
        }

        public override Task OnMediaAddedAsync(MediaAddedMessage message)
        {
            var subject = $"Mouseion - Media Added: {message.Title}";
            var body = $"Media Added to Library\n\n" +
                       $"Title: {message.Title}\n" +
                       $"Type: {message.MediaType}\n" +
                       $"Year: {message.Year}";
            if (!string.IsNullOrWhiteSpace(message.Overview))
            {
                body += $"\n\nOverview:\n{message.Overview}";
            }
            return SendEmailAsync(subject, body);
        }

        public override Task OnHealthIssueAsync(HealthIssueMessage message)
        {
            var subject = $"Mouseion - Health Check {message.Type}: {message.Source}";
            var body = $"Health Check {message.Type}\n\n" +
                       $"Source: {message.Source}\n" +
                       $"Message: {message.Message}";
            if (!string.IsNullOrWhiteSpace(message.WikiUrl))
            {
                body += $"\n\nMore Info: {message.WikiUrl}";
            }
            return SendEmailAsync(subject, body);
        }

        public override Task OnHealthRestoredAsync(HealthRestoredMessage message)
        {
            var subject = $"Mouseion - Health Restored: {message.Source}";
            var body = $"Health Restored\n\n" +
                       $"Source: {message.Source}\n" +
                       $"Previous Issue: {message.PreviousMessage}";
            return SendEmailAsync(subject, body);
        }

        public override Task OnApplicationUpdateAsync(ApplicationUpdateMessage message)
        {
            var subject = "Mouseion - Application Updated";
            var body = $"Application Updated\n\n" +
                       $"Previous Version: {message.PreviousVersion}\n" +
                       $"New Version: {message.NewVersion}";
            if (!string.IsNullOrWhiteSpace(message.ReleaseNotes))
            {
                body += $"\n\nRelease Notes:\n{message.ReleaseNotes}";
            }
            return SendEmailAsync(subject, body);
        }

        private async Task SendEmailAsync(string subject, string body)
        {
            var mimeMessage = new MimeMessage();

            // From
            mimeMessage.From.Add(string.IsNullOrWhiteSpace(Settings.FromName)
                ? new MailboxAddress(Settings.FromAddress, Settings.FromAddress)
                : new MailboxAddress(Settings.FromName, Settings.FromAddress));

            // To
            foreach (var address in Settings.ToAddresses.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
            {
                mimeMessage.To.Add(MailboxAddress.Parse(address));
            }

            // CC
            if (!string.IsNullOrWhiteSpace(Settings.CcAddresses))
            {
                foreach (var address in Settings.CcAddresses.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
                {
                    mimeMessage.Cc.Add(MailboxAddress.Parse(address));
                }
            }

            // BCC
            if (!string.IsNullOrWhiteSpace(Settings.BccAddresses))
            {
                foreach (var address in Settings.BccAddresses.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
                {
                    mimeMessage.Bcc.Add(MailboxAddress.Parse(address));
                }
            }

            mimeMessage.Subject = subject;
            mimeMessage.Body = new TextPart("plain") { Text = body };

            using var client = new SmtpClient();
            await client.ConnectAsync(Settings.Server, Settings.Port, Settings.UseSsl);

            if (!string.IsNullOrWhiteSpace(Settings.Username) && !string.IsNullOrWhiteSpace(Settings.Password))
            {
                await client.AuthenticateAsync(Settings.Username, Settings.Password);
            }

            await client.SendAsync(mimeMessage);
            await client.DisconnectAsync(true);

            _logger.LogDebug("Email notification sent successfully");
        }
    }
}
