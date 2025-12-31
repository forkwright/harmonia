// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.IO;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Serialization;

namespace Mouseion.Common.Serializer
{
    public static class Json
    {
        private static readonly JsonSerializer Serializer;
        private static readonly JsonSerializerSettings SerializerSettings;

        static Json()
        {
            SerializerSettings = GetSerializerSettings();
            Serializer = JsonSerializer.Create(SerializerSettings);
        }

        public static JsonSerializerSettings GetSerializerSettings()
        {
            var serializerSettings = new JsonSerializerSettings
            {
                DateTimeZoneHandling = DateTimeZoneHandling.Utc,
                NullValueHandling = NullValueHandling.Ignore,
                Formatting = Formatting.Indented,
                DefaultValueHandling = DefaultValueHandling.Include,
                ContractResolver = new CamelCasePropertyNamesContractResolver()
            };

            serializerSettings.Converters.Add(new StringEnumConverter { NamingStrategy = new CamelCaseNamingStrategy() });
            serializerSettings.Converters.Add(new VersionConverter());

            return serializerSettings;
        }

        public static T? Deserialize<T>(string json)
            where T : class, new()
        {
            return JsonConvert.DeserializeObject<T>(json, SerializerSettings);
        }

        public static object? Deserialize(string json, Type type)
        {
            return JsonConvert.DeserializeObject(json, type, SerializerSettings);
        }

        public static bool TryDeserialize<T>(string json, out T? result)
            where T : class, new()
        {
            try
            {
                result = Deserialize<T>(json);
                return true;
            }
            catch (JsonReaderException)
            {
                result = null;
                return false;
            }
            catch (JsonSerializationException)
            {
                result = null;
                return false;
            }
        }

        public static string ToJson(object obj)
        {
            return JsonConvert.SerializeObject(obj, SerializerSettings);
        }

        public static void Serialize<TModel>(TModel model, TextWriter outputStream)
        {
            var jsonTextWriter = new JsonTextWriter(outputStream);
            Serializer.Serialize(jsonTextWriter, model);
            jsonTextWriter.Flush();
        }

        public static void Serialize<TModel>(TModel model, Stream outputStream, JsonSerializerSettings? settings = null)
        {
            var serializer = (settings == null) ? Serializer : JsonSerializer.Create(settings);
            using (var streamWriter = new StreamWriter(outputStream))
            using (var jsonWriter = new JsonTextWriter(streamWriter))
            {
                serializer.Serialize(jsonWriter, model);
            }
        }

        public static void RegisterConverter(JsonConverter converter)
        {
            SerializerSettings.Converters.Add(converter);
        }
    }
}
