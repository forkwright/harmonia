// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Data;
using System.Text.Json;
using Dapper;

namespace Mouseion.Core.Datastore;

public class JsonListStringHandler : SqlMapper.TypeHandler<List<string>>
{
    public override void SetValue(IDbDataParameter parameter, List<string>? value)
    {
        parameter.Value = value != null ? JsonSerializer.Serialize(value) : "[]";
    }

    public override List<string> Parse(object value)
    {
        if (value is string json && !string.IsNullOrEmpty(json))
        {
            try { return JsonSerializer.Deserialize<List<string>>(json) ?? new List<string>(); }
            catch { return new List<string>(); }
        }
        return new List<string>();
    }
}

public class JsonHashSetIntHandler : SqlMapper.TypeHandler<HashSet<int>>
{
    public override void SetValue(IDbDataParameter parameter, HashSet<int>? value)
    {
        parameter.Value = value != null ? JsonSerializer.Serialize(value) : "[]";
    }

    public override HashSet<int> Parse(object value)
    {
        if (value is string json && !string.IsNullOrEmpty(json))
        {
            try { return JsonSerializer.Deserialize<HashSet<int>>(json) ?? new HashSet<int>(); }
            catch { return new HashSet<int>(); }
        }
        return new HashSet<int>();
    }
}

public static class DapperTypeHandlers
{
    public static void Register()
    {
        SqlMapper.AddTypeHandler(new JsonListStringHandler());
        SqlMapper.AddTypeHandler(new JsonHashSetIntHandler());
    }
}
