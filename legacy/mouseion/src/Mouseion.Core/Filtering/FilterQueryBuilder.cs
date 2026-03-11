// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;

namespace Mouseion.Core.Filtering;

public interface IFilterQueryBuilder
{
    (string Sql, DynamicParameters Parameters) BuildQuery(FilterRequest request, string baseTable);
    (string Sql, DynamicParameters Parameters) BuildCountQuery(FilterRequest request, string baseTable);
    bool IsValidField(string field);
}

public class FilterQueryBuilder : IFilterQueryBuilder
{
    private static readonly HashSet<string> AllowedFields = new(StringComparer.OrdinalIgnoreCase)
    {
        "Codec",
        "AudioFormat",
        "SampleRate",
        "Bitrate",
        "Channels",
        "Quality",
        "AlbumId",
        "ArtistId",
        "Year",
        "Explicit",
        "TrackNumber",
        "DiscNumber",
        "DurationSeconds",
        "BitDepth",
        "DynamicRange",
        "Lossless",
        "ArtistName",
        "AlbumName",
        "Genre"
    };

    // Column names are table-qualified for use in JOIN queries.
    // mf = MusicFiles, m = MediaItems, ar = Artists, al = Albums
    private static readonly Dictionary<string, string> FieldToColumn = new(StringComparer.OrdinalIgnoreCase)
    {
        // MusicFiles fields
        { "Codec", "mf.\"AudioFormat\"" },
        { "AudioFormat", "mf.\"AudioFormat\"" },
        { "SampleRate", "mf.\"SampleRate\"" },
        { "Bitrate", "mf.\"Bitrate\"" },
        { "Channels", "mf.\"Channels\"" },
        { "Quality", "mf.\"Quality\"" },
        { "BitDepth", "mf.\"BitDepth\"" },
        { "DynamicRange", "mf.\"DynamicRange\"" },
        { "Lossless", "mf.\"Lossless\"" },
        // MediaItems fields
        { "AlbumId", "m.\"AlbumId\"" },
        { "ArtistId", "m.\"ArtistId\"" },
        { "Explicit", "m.\"Explicit\"" },
        { "TrackNumber", "m.\"TrackNumber\"" },
        { "DiscNumber", "m.\"DiscNumber\"" },
        { "DurationSeconds", "m.\"DurationSeconds\"" },
        // Joined table fields
        { "Year", "al.\"Year\"" },
        { "ArtistName", "ar.\"Name\"" },
        { "AlbumName", "al.\"Title\"" },
        { "Genre", "al.\"Genres\"" }
    };

    public bool IsValidField(string field)
    {
        return AllowedFields.Contains(field);
    }

    public (string Sql, DynamicParameters Parameters) BuildQuery(FilterRequest request, string baseTable)
    {
        var (whereClause, parameters) = BuildWhereFragment(request);

        var offset = (request.Page - 1) * request.PageSize;
        // The base query is a template — callers like TrackRepository.FilterAsync
        // replace the SELECT/FROM with a JOIN query. Column references in whereClause
        // are already table-qualified (e.g., mf."AudioFormat", m."AlbumId").
        var sql = $@"
            SELECT * FROM ""{baseTable}""
            WHERE {whereClause}
            ORDER BY m.""Id"" DESC
            LIMIT @PageSize OFFSET @Offset";

        parameters.Add("PageSize", request.PageSize);
        parameters.Add("Offset", offset);

        return (sql, parameters);
    }

    public (string Sql, DynamicParameters Parameters) BuildCountQuery(FilterRequest request, string baseTable)
    {
        var (whereClause, parameters) = BuildWhereFragment(request);

        var sql = $@"
            SELECT COUNT(*) FROM ""{baseTable}""
            WHERE {whereClause}";

        return (sql, parameters);
    }

    private (string WhereClause, DynamicParameters Parameters) BuildWhereFragment(FilterRequest request)
    {
        if (request.Conditions.Count == 0)
        {
            throw new ArgumentException("At least one filter condition is required");
        }

        foreach (var condition in request.Conditions)
        {
            if (!IsValidField(condition.Field))
            {
                throw new ArgumentException($"Field '{condition.Field}' is not allowed for filtering");
            }
        }

        var parameters = new DynamicParameters();
        var whereClauses = new List<string>();

        for (int i = 0; i < request.Conditions.Count; i++)
        {
            var condition = request.Conditions[i];
            var paramName = $"p{i}";
            var columnName = FieldToColumn[condition.Field];
            var clause = BuildWhereClause(columnName, condition.Operator, paramName, condition.Value, parameters);
            whereClauses.Add(clause);
        }

        var logicOperator = request.Logic == FilterLogic.And ? " AND " : " OR ";
        var whereClause = string.Join(logicOperator, whereClauses);

        return (whereClause, parameters);
    }

    private static string BuildWhereClause(
        string columnName,
        FilterOperator op,
        string paramName,
        string value,
        DynamicParameters parameters)
    {
        switch (op)
        {
            // columnName is already table-qualified and quoted (e.g., mf."AudioFormat")
            case FilterOperator.Equals:
                parameters.Add(paramName, value);
                return $"{columnName} = @{paramName}";

            case FilterOperator.NotEquals:
                parameters.Add(paramName, value);
                return $"{columnName} != @{paramName}";

            case FilterOperator.Contains:
                parameters.Add(paramName, $"%{value}%");
                return $"{columnName} LIKE @{paramName}";

            case FilterOperator.NotContains:
                parameters.Add(paramName, $"%{value}%");
                return $"{columnName} NOT LIKE @{paramName}";

            case FilterOperator.GreaterThan:
                parameters.Add(paramName, value);
                return $"{columnName} > @{paramName}";

            case FilterOperator.LessThan:
                parameters.Add(paramName, value);
                return $"{columnName} < @{paramName}";

            case FilterOperator.GreaterThanOrEqual:
                parameters.Add(paramName, value);
                return $"{columnName} >= @{paramName}";

            case FilterOperator.LessThanOrEqual:
                parameters.Add(paramName, value);
                return $"{columnName} <= @{paramName}";

            case FilterOperator.In:
                var values = value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
                var inParams = new List<string>();
                for (int i = 0; i < values.Length; i++)
                {
                    var inParamName = $"{paramName}_in{i}";
                    parameters.Add(inParamName, values[i]);
                    inParams.Add($"@{inParamName}");
                }
                return $"{columnName} IN ({string.Join(", ", inParams)})";

            case FilterOperator.NotIn:
                var notInValues = value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
                var notInParams = new List<string>();
                for (int i = 0; i < notInValues.Length; i++)
                {
                    var notInParamName = $"{paramName}_notin{i}";
                    parameters.Add(notInParamName, notInValues[i]);
                    notInParams.Add($"@{notInParamName}");
                }
                return $"{columnName} NOT IN ({string.Join(", ", notInParams)})";

            default:
                throw new ArgumentException($"Unsupported operator: {op}");
        }
    }
}
