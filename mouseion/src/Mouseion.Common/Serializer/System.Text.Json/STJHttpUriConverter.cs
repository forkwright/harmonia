// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using Mouseion.Common.Http;

namespace Mouseion.Common.Serializer
{
    public class STJHttpUriConverter : JsonConverter<HttpUri>
    {
        public override HttpUri Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            return new HttpUri(reader.GetString());
        }

        public override void Write(Utf8JsonWriter writer, HttpUri value, JsonSerializerOptions options)
        {
            if (value == null)
            {
                writer.WriteNullValue();
            }
            else
            {
                writer.WriteStringValue(value.FullUri);
            }
        }
    }
}
