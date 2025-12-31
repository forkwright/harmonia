// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using Mouseion.Common.Http;
using Newtonsoft.Json;

namespace Mouseion.Common.Serializer
{
    public class HttpUriConverter : JsonConverter
    {
        public override void WriteJson(JsonWriter writer, object value, JsonSerializer serializer)
        {
            if (value == null)
            {
                writer.WriteNull();
            }
            else if (value is HttpUri)
            {
                writer.WriteValue((value as HttpUri).FullUri);
            }
        }

        public override object ReadJson(JsonReader reader, Type objectType, object existingValue, JsonSerializer serializer)
        {
            return new HttpUri(reader.ReadAsString());
        }

        public override bool CanConvert(Type objectType)
        {
            return objectType == typeof(HttpUri);
        }
    }
}
