// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using Newtonsoft.Json;

namespace Mouseion.Common.Serializer
{
    public class IntConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType)
        {
            return objectType == typeof(int) || objectType == typeof(int?);
        }

        public override object? ReadJson(JsonReader reader, Type objectType, object? existingValue, JsonSerializer serializer)
        {
            if (reader.TokenType == JsonToken.Null)
            {
                return objectType == typeof(int?) ? null : 0;
            }

            if (reader.TokenType == JsonToken.Integer)
            {
                return Convert.ToInt32(reader.Value);
            }

            if (reader.TokenType == JsonToken.String)
            {
                var stringValue = reader.Value?.ToString();
                if (string.IsNullOrWhiteSpace(stringValue))
                {
                    return objectType == typeof(int?) ? null : 0;
                }

                if (int.TryParse(stringValue, out var intValue))
                {
                    return intValue;
                }
            }

            throw new JsonSerializationException($"Cannot convert {reader.Value} to Int32");
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            if (value == null)
            {
                writer.WriteNull();
            }
            else
            {
                writer.WriteValue((int)value);
            }
        }
    }
}
