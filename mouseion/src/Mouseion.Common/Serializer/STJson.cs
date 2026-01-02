// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

namespace Mouseion.Common.Serializer
{
    public static class STJson
    {
        private static readonly JsonSerializerOptions SerializerSettings;

        static STJson()
        {
            SerializerSettings = GetSerializerSettings();
        }

        public static JsonSerializerOptions GetSerializerSettings()
        {
            var serializerSettings = new JsonSerializerOptions();
            ApplySerializerSettings(serializerSettings);
            return serializerSettings;
        }

        public static void ApplySerializerSettings(JsonSerializerOptions serializerSettings)
        {
            serializerSettings.AllowTrailingCommas = true;
            serializerSettings.DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull;
            serializerSettings.PropertyNameCaseInsensitive = true;
            serializerSettings.DictionaryKeyPolicy = JsonNamingPolicy.CamelCase;
            serializerSettings.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
            serializerSettings.WriteIndented = true;

            serializerSettings.Converters.Add(new JsonStringEnumConverter(JsonNamingPolicy.CamelCase, true));
            serializerSettings.Converters.Add(new STJVersionConverter());
            serializerSettings.Converters.Add(new STJTimeSpanConverter());
            serializerSettings.Converters.Add(new STJUtcConverter());
        }

        public static T? Deserialize<T>(string json)
            where T : class
        {
            return JsonSerializer.Deserialize<T>(json, SerializerSettings);
        }

        public static T? Deserialize<T>(ReadOnlySpan<byte> json)
            where T : class
        {
            return JsonSerializer.Deserialize<T>(json, SerializerSettings);
        }

        public static object? Deserialize(string json, Type type)
        {
            return JsonSerializer.Deserialize(json, type, SerializerSettings);
        }

        public static ValueTask<T?> DeserializeAsync<T>(Stream json)
        {
            return JsonSerializer.DeserializeAsync<T>(json, SerializerSettings);
        }

        public static bool TryDeserialize<T>(string json, out T? result)
            where T : class
        {
            try
            {
                result = Deserialize<T>(json);
                return true;
            }
            catch (JsonException)
            {
                result = null;
                return false;
            }
        }

        public static string ToJson(object obj)
        {
            return JsonSerializer.Serialize(obj, SerializerSettings);
        }

        public static Task SerializeAsync<TModel>(TModel model, Stream outputStream)
        {
            return JsonSerializer.SerializeAsync(outputStream, model, SerializerSettings);
        }
    }
}
