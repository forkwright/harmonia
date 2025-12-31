// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Mouseion.Common.Serializer
{
    public class STJUtcConverter : JsonConverter<DateTime>
    {
        public override DateTime Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            if (reader.TokenType == JsonTokenType.String)
            {
                var stringValue = reader.GetString();
                if (string.IsNullOrWhiteSpace(stringValue))
                {
                    return DateTime.MinValue;
                }

                var dateTime = DateTime.Parse(stringValue);
                return DateTime.SpecifyKind(dateTime, DateTimeKind.Utc);
            }

            throw new JsonException($"Expected string token for DateTime, got {reader.TokenType}");
        }

        public override void Write(Utf8JsonWriter writer, DateTime value, JsonSerializerOptions options)
        {
            var utcValue = value.Kind == DateTimeKind.Utc
                ? value
                : DateTime.SpecifyKind(value, DateTimeKind.Utc);

            writer.WriteStringValue(utcValue.ToString("O"));
        }
    }
}
