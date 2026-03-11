// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Caching.Memory;
using Mouseion.Core.Datastore;
using Serilog;

namespace Mouseion.Api.Library;

[ApiController]
[Route("api/v3/library")]
[Authorize]
public class FacetsController : ControllerBase
{
    private const string CacheKey = "library_facets";
    private const int CacheDurationMinutes = 60;

    private readonly IDatabase _database;
    private readonly IMemoryCache _cache;
    private readonly ILogger _logger;

    public FacetsController(IDatabase database, IMemoryCache cache)
    {
        _database = database;
        _cache = cache;
        _logger = Log.ForContext<FacetsController>();
    }

    [HttpGet("facets")]
    public ActionResult<LibraryFacetsResource> GetFacets()
    {
        try
        {
            if (_cache.TryGetValue(CacheKey, out LibraryFacetsResource? cached))
            {
                _logger.Debug("Cache hit for library facets");
                return Ok(cached);
            }

            var facets = BuildFacets();
            _cache.Set(CacheKey, facets, TimeSpan.FromMinutes(CacheDurationMinutes));

            return Ok(facets);
        }
        catch (InvalidOperationException ex)
        {
            _logger.Error(ex, "Database error fetching library facets");
            return StatusCode(500, new { error = "Internal server error" });
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Error(ex, "Data error fetching library facets");
            return StatusCode(500, new { error = "Internal server error" });
        }
    }

    private LibraryFacetsResource BuildFacets()
    {
        using var conn = _database.OpenConnection();

        var formats = GetUniquFormats(conn);
        var sampleRates = GetUniqueSampleRates(conn);
        var bitDepths = GetUniqueBitDepths(conn);
        var genres = GetUniqueGenres(conn);
        var dynamicRangeRange = GetDynamicRangeRange(conn);
        var yearRange = GetYearRange(conn);

        return new LibraryFacetsResource
        {
            Formats = formats,
            SampleRates = sampleRates,
            BitDepths = bitDepths,
            Genres = genres,
            DynamicRangeRange = dynamicRangeRange,
            YearRange = yearRange
        };
    }

    private List<string> GetUniquFormats(global::System.Data.IDbConnection conn)
    {
        const string sql = @"
            SELECT DISTINCT COALESCE(""AudioFormat"", 'Unknown') as Format
            FROM ""MusicFiles""
            WHERE ""AudioFormat"" IS NOT NULL
            ORDER BY ""AudioFormat""";

        try
        {
            return conn.Query<string>(sql).Where(f => !string.IsNullOrWhiteSpace(f)).Distinct().ToList();
        }
        catch (InvalidOperationException ex)
        {
            _logger.Warning(ex, "Database error querying unique formats");
            return new List<string>();
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Warning(ex, "Data error querying unique formats");
            return new List<string>();
        }
    }

    private List<int> GetUniqueSampleRates(global::System.Data.IDbConnection conn)
    {
        const string sql = @"
            SELECT DISTINCT ""SampleRate""
            FROM ""MusicFiles""
            WHERE ""SampleRate"" IS NOT NULL AND ""SampleRate"" > 0
            ORDER BY ""SampleRate""";

        try
        {
            return conn.Query<int>(sql).Where(sr => sr > 0).Distinct().ToList();
        }
        catch (InvalidOperationException ex)
        {
            _logger.Warning(ex, "Database error querying unique sample rates");
            return new List<int>();
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Warning(ex, "Data error querying unique sample rates");
            return new List<int>();
        }
    }

    private List<int> GetUniqueBitDepths(global::System.Data.IDbConnection conn)
    {
        const string sql = @"
            SELECT DISTINCT ""BitDepth""
            FROM ""MusicFiles""
            WHERE ""BitDepth"" IS NOT NULL AND ""BitDepth"" > 0
            ORDER BY ""BitDepth""";

        try
        {
            var bitDepths = conn.Query<int>(sql).ToList();
            return bitDepths.Count > 0 ? bitDepths : new List<int> { 16, 24, 32 };
        }
        catch (InvalidOperationException ex)
        {
            _logger.Warning(ex, "Database error querying unique bit depths, returning defaults");
            return new List<int> { 16, 24, 32 };
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Warning(ex, "Data error querying unique bit depths, returning defaults");
            return new List<int> { 16, 24, 32 };
        }
    }

    private List<string> GetUniqueGenres(global::System.Data.IDbConnection conn)
    {
        const string sql = @"
            SELECT ""Genres""
            FROM ""Albums""
            WHERE ""Genres"" IS NOT NULL AND ""Genres"" != ''";

        try
        {
            var genreJsonArrays = conn.Query<string>(sql).ToList();
            var genres = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

            foreach (var jsonArray in genreJsonArrays)
            {
                if (!string.IsNullOrWhiteSpace(jsonArray))
                {
                    ParseGenresFromJson(jsonArray, genres);
                }
            }

            return genres.OrderBy(g => g).ToList();
        }
        catch (InvalidOperationException ex)
        {
            _logger.Warning(ex, "Database error querying unique genres");
            return new List<string>();
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Warning(ex, "Data error querying unique genres");
            return new List<string>();
        }
    }

    private void ParseGenresFromJson(string jsonArray, HashSet<string> genres)
    {
        try
        {
            using var doc = global::System.Text.Json.JsonDocument.Parse(jsonArray);
            var root = doc.RootElement;

            if (root.ValueKind != global::System.Text.Json.JsonValueKind.Array)
                return;

            foreach (var element in root.EnumerateArray())
            {
                if (element.ValueKind != global::System.Text.Json.JsonValueKind.String)
                    continue;

                var genre = element.GetString();
                if (!string.IsNullOrWhiteSpace(genre))
                {
                    genres.Add(genre);
                }
            }
        }
        catch (global::System.Text.Json.JsonException)
        {
            _logger.Warning("Invalid JSON in genres field");
        }
    }

    private RangeResource GetDynamicRangeRange(global::System.Data.IDbConnection conn)
    {
        const string sql = @"
            SELECT
                MIN(COALESCE(""DynamicRange"", 0)) as MinValue,
                MAX(COALESCE(""DynamicRange"", 0)) as MaxValue
            FROM ""MusicFiles""
            WHERE ""DynamicRange"" IS NOT NULL";

        try
        {
            var result = conn.QuerySingleOrDefault<(int Min, int Max)>(sql);
            return result != default ? new RangeResource { Min = result.Min, Max = result.Max } : new RangeResource { Min = 0, Max = 20 };
        }
        catch (InvalidOperationException ex)
        {
            _logger.Warning(ex, "Database error querying dynamic range, returning defaults");
            return new RangeResource { Min = 0, Max = 20 };
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Warning(ex, "Data error querying dynamic range, returning defaults");
            return new RangeResource { Min = 0, Max = 20 };
        }
    }

    private RangeResource GetYearRange(global::System.Data.IDbConnection conn)
    {
        const string sql = @"
            SELECT
                MIN(COALESCE(CAST(strftime('%Y', ""ReleaseDate"") AS INTEGER), 1900)) as MinValue,
                MAX(COALESCE(CAST(strftime('%Y', ""ReleaseDate"") AS INTEGER), 2100)) as MaxValue
            FROM ""Albums""
            WHERE ""ReleaseDate"" IS NOT NULL";

        try
        {
            var result = conn.QuerySingleOrDefault<(int Min, int Max)>(sql);
            return new RangeResource
            {
                Min = result.Min > 0 ? result.Min : 1900,
                Max = result.Max > 0 ? result.Max : DateTime.Now.Year
            };
        }
        catch (InvalidOperationException ex)
        {
            _logger.Warning(ex, "Database error querying year range");
            return new RangeResource { Min = 1900, Max = DateTime.Now.Year };
        }
        catch (global::System.Data.DataException ex)
        {
            _logger.Warning(ex, "Data error querying year range");
            return new RangeResource { Min = 1900, Max = DateTime.Now.Year };
        }
    }

    /// <summary>
    /// Invalidate the facets cache - call this after library scans
    /// </summary>
    public static void InvalidateCache(IMemoryCache cache)
    {
        cache.Remove(CacheKey);
    }
}

public class LibraryFacetsResource
{
    public List<string> Formats { get; set; } = new();
    public List<int> SampleRates { get; set; } = new();
    public List<int> BitDepths { get; set; } = new();
    public List<string> Genres { get; set; } = new();
    public RangeResource DynamicRangeRange { get; set; } = new();
    public RangeResource YearRange { get; set; } = new();
}

public class RangeResource
{
    public int Min { get; set; }
    public int Max { get; set; }
}
