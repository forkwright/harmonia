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

    private static readonly Dictionary<string, string> FieldToColumn = new(StringComparer.OrdinalIgnoreCase)
    {
        { "Codec", "AudioFormat" },
        { "AudioFormat", "AudioFormat" },
        { "SampleRate", "SampleRate" },
        { "Bitrate", "Bitrate" },
        { "Channels", "Channels" },
        { "Quality", "Quality" },
        { "AlbumId", "AlbumId" },
        { "ArtistId", "ArtistId" },
        { "Year", "Year" },
        { "Explicit", "Explicit" },
        { "TrackNumber", "TrackNumber" },
        { "DiscNumber", "DiscNumber" },
        { "DurationSeconds", "DurationSeconds" },
        { "BitDepth", "BitDepth" },
        { "DynamicRange", "DynamicRange" },
        { "Lossless", "Lossless" },
        { "ArtistName", "ar.Name" },
        { "AlbumName", "al.Title" },
        { "Genre", "al.Genres" }
    };

    public bool IsValidField(string field)
    {
        return AllowedFields.Contains(field);
    }

    public (string Sql, DynamicParameters Parameters) BuildQuery(FilterRequest request, string baseTable)
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

        var offset = (request.Page - 1) * request.PageSize;
        var sql = $@"
            SELECT * FROM ""{baseTable}""
            WHERE {whereClause}
            ORDER BY ""Id"" DESC
            LIMIT @PageSize OFFSET @Offset";

        parameters.Add("PageSize", request.PageSize);
        parameters.Add("Offset", offset);

        return (sql, parameters);
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
            case FilterOperator.Equals:
                parameters.Add(paramName, value);
                return $"\"{columnName}\" = @{paramName}";

            case FilterOperator.NotEquals:
                parameters.Add(paramName, value);
                return $"\"{columnName}\" != @{paramName}";

            case FilterOperator.Contains:
                parameters.Add(paramName, $"%{value}%");
                return $"\"{columnName}\" LIKE @{paramName}";

            case FilterOperator.NotContains:
                parameters.Add(paramName, $"%{value}%");
                return $"\"{columnName}\" NOT LIKE @{paramName}";

            case FilterOperator.GreaterThan:
                parameters.Add(paramName, value);
                return $"\"{columnName}\" > @{paramName}";

            case FilterOperator.LessThan:
                parameters.Add(paramName, value);
                return $"\"{columnName}\" < @{paramName}";

            case FilterOperator.GreaterThanOrEqual:
                parameters.Add(paramName, value);
                return $"\"{columnName}\" >= @{paramName}";

            case FilterOperator.LessThanOrEqual:
                parameters.Add(paramName, value);
                return $"\"{columnName}\" <= @{paramName}";

            case FilterOperator.In:
                var values = value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
                var inParams = new List<string>();
                for (int i = 0; i < values.Length; i++)
                {
                    var inParamName = $"{paramName}_in{i}";
                    parameters.Add(inParamName, values[i]);
                    inParams.Add($"@{inParamName}");
                }
                return $"\"{columnName}\" IN ({string.Join(", ", inParams)})";

            case FilterOperator.NotIn:
                var notInValues = value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
                var notInParams = new List<string>();
                for (int i = 0; i < notInValues.Length; i++)
                {
                    var notInParamName = $"{paramName}_notin{i}";
                    parameters.Add(notInParamName, notInValues[i]);
                    notInParams.Add($"@{notInParamName}");
                }
                return $"\"{columnName}\" NOT IN ({string.Join(", ", notInParams)})";

            default:
                throw new ArgumentException($"Unsupported operator: {op}");
        }
    }
}
