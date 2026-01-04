// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Movies.Calendar;

namespace Mouseion.Api.Movies;

[ApiController]
[Route("api/v3/calendar")]
[Authorize]
public class CalendarController : ControllerBase
{
    private readonly IMovieCalendarService _calendarService;

    public CalendarController(IMovieCalendarService calendarService)
    {
        _calendarService = calendarService;
    }

    /// <summary>
    /// Get calendar entries for movies within a date range
    /// </summary>
    /// <param name="start">Start date (default: today)</param>
    /// <param name="end">End date (default: today + 2 days)</param>
    /// <param name="includeUnmonitored">Include unmonitored movies (default: false)</param>
    /// <param name="ct">Cancellation token</param>
    /// <returns>List of calendar entries</returns>
    [HttpGet]
    public async Task<ActionResult<List<CalendarEntryResource>>> GetCalendar(
        [FromQuery] DateTime? start = null,
        [FromQuery] DateTime? end = null,
        [FromQuery] bool includeUnmonitored = false,
        CancellationToken ct = default)
    {
        var startDate = start ?? DateTime.Today;
        var endDate = end ?? DateTime.Today.AddDays(2);

        var entries = await _calendarService.GetCalendarEntriesAsync(startDate, endDate, includeUnmonitored, ct).ConfigureAwait(false);

        return Ok(entries.Select(ToResource).ToList());
    }

    private static CalendarEntryResource ToResource(MovieCalendarEntry entry)
    {
        return new CalendarEntryResource
        {
            MovieId = entry.MovieId,
            Title = entry.Title,
            Year = entry.Year,
            ReleaseDate = entry.ReleaseDate,
            ReleaseType = entry.ReleaseType,
            Monitored = entry.Monitored,
            HasFile = entry.HasFile
        };
    }
}

public class CalendarEntryResource
{
    public int MovieId { get; set; }
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? ReleaseType { get; set; }
    public bool Monitored { get; set; }
    public bool HasFile { get; set; }
}
