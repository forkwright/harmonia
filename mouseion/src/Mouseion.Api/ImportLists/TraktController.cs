// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.ImportLists;
using Mouseion.Core.ImportLists.Trakt;

namespace Mouseion.Api.ImportLists;

/// <summary>
/// Trakt OAuth device code authorization and import management.
/// Device code flow: POST /authorize → user enters code at trakt.tv → POST /token (poll) → GET /status
/// </summary>
[ApiController]
[Route("api/v3/importlists/trakt")]
[Authorize(Policy = "UserOrAdmin")]
public class TraktController : ControllerBase
{
    private readonly TraktImportList _traktImportList;
    private readonly IImportListRepository _importListRepository;

    public TraktController(
        TraktImportList traktImportList,
        IImportListRepository importListRepository)
    {
        _traktImportList = traktImportList;
        _importListRepository = importListRepository;
    }

    /// <summary>
    /// Step 1: Get a device code. Show the user_code to the user and direct them to verification_url.
    /// </summary>
    [HttpPost("authorize")]
    public async Task<ActionResult<TraktDeviceCode>> RequestDeviceCode(
        CancellationToken ct = default)
    {
        var deviceCode = await _traktImportList.RequestDeviceCodeAsync(ct).ConfigureAwait(false);
        return Ok(deviceCode);
    }

    /// <summary>
    /// Step 2: Poll for authorization. Client should call this at the interval from Step 1.
    /// Returns 200 with token on success, 202 when still pending.
    /// </summary>
    [HttpPost("token")]
    public async Task<ActionResult> PollForToken(
        [FromBody][Required] DeviceCodeRequest request,
        CancellationToken ct = default)
    {
        var tokenResponse = await _traktImportList.PollForAuthorizationAsync(request.DeviceCode, ct).ConfigureAwait(false);

        if (tokenResponse == null)
        {
            // 202 Accepted = still pending, keep polling
            return Accepted(new { status = "pending", message = "Waiting for user to authorize at trakt.tv" });
        }

        return Ok(new TraktAuthResult
        {
            AccessToken = tokenResponse.AccessToken,
            RefreshToken = tokenResponse.RefreshToken,
            ExpiresIn = tokenResponse.ExpiresIn,
            Scope = tokenResponse.Scope
        });
    }

    /// <summary>
    /// Trigger a manual sync from Trakt.
    /// </summary>
    [HttpPost("sync")]
    public async Task<ActionResult<ImportListFetchResult>> SyncNow(CancellationToken ct = default)
    {
        var result = await _traktImportList.FetchAsync(ct).ConfigureAwait(false);
        return Ok(result);
    }

    /// <summary>
    /// Get current Trakt connection status.
    /// </summary>
    [HttpGet("status")]
    public ActionResult<TraktStatusResource> GetStatus()
    {
        return Ok(new TraktStatusResource
        {
            IsConnected = _traktImportList.EnableAuto,
            Username = _traktImportList.Definition?.Settings != null
                ? global::System.Text.Json.JsonSerializer.Deserialize<TraktSettings>(
                    _traktImportList.Definition.Settings)?.TraktUsername ?? ""
                : ""
        });
    }
}

public class DeviceCodeRequest
{
    public string DeviceCode { get; set; } = string.Empty;
}

public class TraktAuthResult
{
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public int ExpiresIn { get; set; }
    public string Scope { get; set; } = string.Empty;
}

public class TraktStatusResource
{
    public bool IsConnected { get; set; }
    public string Username { get; set; } = string.Empty;
}
