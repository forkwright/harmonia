// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Core.Notifications;

namespace Mouseion.Api.Notifications;

/// <summary>
/// API controller for managing notification configurations
/// </summary>
[ApiController]
[Route("api/v3/notifications")]
[Authorize]
public class NotificationController : ControllerBase
{
    private readonly INotificationService _notificationService;
    private readonly ILogger<NotificationController> _logger;

    public NotificationController(
        INotificationService notificationService,
        ILogger<NotificationController> logger)
    {
        _notificationService = notificationService;
        _logger = logger;
    }

    /// <summary>
    /// List all configured notifications
    /// </summary>
    [HttpGet]
    public async Task<ActionResult<List<NotificationResource>>> List(CancellationToken ct)
    {
        _logger.LogDebug("Listing all notifications");

        var definitions = await _notificationService.GetAllAsync(ct).ConfigureAwait(false);
        var resources = definitions.Select(ToResource).ToList();

        return Ok(resources);
    }

    /// <summary>
    /// Get a specific notification by ID
    /// </summary>
    [HttpGet("{id:int}")]
    public async Task<ActionResult<NotificationResource>> Get(int id, CancellationToken ct)
    {
        _logger.LogDebug("Getting notification {Id}", id);

        var definition = await _notificationService.GetAsync(id, ct).ConfigureAwait(false);

        if (definition == null)
        {
            return NotFound();
        }

        return Ok(ToResource(definition));
    }

    /// <summary>
    /// Create a new notification configuration
    /// </summary>
    [HttpPost]
    public async Task<ActionResult<NotificationResource>> Create(
        [FromBody][Required] NotificationResource resource,
        CancellationToken ct)
    {
        _logger.LogDebug("Creating notification: {Name}", resource.Name.SanitizeForLog());

        var definition = ToDefinition(resource);
        var created = await _notificationService.CreateAsync(definition, ct).ConfigureAwait(false);

        return CreatedAtAction(nameof(Get), new { id = created.Id }, ToResource(created));
    }

    /// <summary>
    /// Update an existing notification configuration
    /// </summary>
    [HttpPut("{id:int}")]
    public async Task<ActionResult<NotificationResource>> Update(
        int id,
        [FromBody][Required] NotificationResource resource,
        CancellationToken ct)
    {
        _logger.LogDebug("Updating notification {Id}", id);

        var existing = await _notificationService.GetAsync(id, ct).ConfigureAwait(false);

        if (existing == null)
        {
            return NotFound();
        }

        var definition = ToDefinition(resource);
        definition.Id = id;
        var updated = await _notificationService.UpdateAsync(definition, ct).ConfigureAwait(false);

        return Ok(ToResource(updated));
    }

    /// <summary>
    /// Delete a notification configuration
    /// </summary>
    [HttpDelete("{id:int}")]
    public async Task<ActionResult> Delete(int id, CancellationToken ct)
    {
        _logger.LogDebug("Deleting notification {Id}", id);

        var existing = await _notificationService.GetAsync(id, ct).ConfigureAwait(false);

        if (existing == null)
        {
            return NotFound();
        }

        await _notificationService.DeleteAsync(id, ct).ConfigureAwait(false);

        return NoContent();
    }

    /// <summary>
    /// Test a notification configuration
    /// </summary>
    [HttpPost("{id:int}/test")]
    public async Task<ActionResult> Test(int id, CancellationToken ct)
    {
        _logger.LogDebug("Testing notification {Id}", id);

        var existing = await _notificationService.GetAsync(id, ct).ConfigureAwait(false);

        if (existing == null)
        {
            return NotFound();
        }

        var success = await _notificationService.TestAsync(id, ct).ConfigureAwait(false);

        return Ok(new { success, message = success ? "Test notification sent successfully" : "Test notification failed" });
    }

    private static NotificationResource ToResource(NotificationDefinition definition)
    {
        return new NotificationResource
        {
            Id = definition.Id,
            Name = definition.Name,
            Type = definition.Implementation,
            Enabled = definition.Enabled,
            OnGrab = definition.OnGrab,
            OnDownload = definition.OnDownload,
            OnRename = definition.OnRename,
            OnMediaAdded = definition.OnMediaAdded,
            OnMediaDeleted = definition.OnMediaDeleted,
            OnHealthIssue = definition.OnHealthIssue,
            OnHealthRestored = definition.OnHealthRestored,
            OnApplicationUpdate = definition.OnApplicationUpdate,
            Settings = definition.Settings ?? new object()
        };
    }

    private static NotificationDefinition ToDefinition(NotificationResource resource)
    {
        return new NotificationDefinition
        {
            Id = resource.Id,
            Name = resource.Name,
            Implementation = resource.Type,
            ConfigContract = $"{resource.Type}Settings",
            Settings = resource.Settings is string s ? s : global::System.Text.Json.JsonSerializer.Serialize(resource.Settings),
            Enabled = resource.Enabled,
            OnGrab = resource.OnGrab,
            OnDownload = resource.OnDownload,
            OnRename = resource.OnRename,
            OnMediaAdded = resource.OnMediaAdded,
            OnMediaDeleted = resource.OnMediaDeleted,
            OnHealthIssue = resource.OnHealthIssue,
            OnHealthRestored = resource.OnHealthRestored,
            OnApplicationUpdate = resource.OnApplicationUpdate
        };
    }
}
