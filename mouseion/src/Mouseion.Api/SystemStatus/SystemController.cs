// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.SystemInfo;

namespace Mouseion.Api.SystemStatus;

[ApiController]
[Route("api/v3/system")]
[Authorize]
public class SystemController : ControllerBase
{
    private readonly ISystemService _systemService;

    public SystemController(ISystemService systemService)
    {
        _systemService = systemService;
    }

    [HttpGet("status")]
    public ActionResult<Core.SystemInfo.SystemInfo> GetStatus()
    {
        var systemInfo = _systemService.GetSystemInfo();
        return Ok(systemInfo);
    }
}
