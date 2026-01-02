// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Qualities;

public class QualityModel : IEmbeddedDocument, IEquatable<QualityModel>
{
    public Quality Quality { get; set; }

    public Revision Revision { get; set; }

    [JsonIgnore]
    public QualityDetectionSource SourceDetectionSource { get; set; }

    [JsonIgnore]
    public QualityDetectionSource ResolutionDetectionSource { get; set; }

    [JsonIgnore]
    public QualityDetectionSource ModifierDetectionSource { get; set; }

    [JsonIgnore]
    public QualityDetectionSource RevisionDetectionSource { get; set; }

    public QualityModel()
        : this(Quality.Unknown, new Revision())
    {
    }

    public QualityModel(Quality quality, Revision? revision = null)
    {
        Quality = quality;
        Revision = revision ?? new Revision();
    }

    public override string ToString()
    {
        return string.Format("{0} {1}", Quality, Revision);
    }

    public override int GetHashCode()
    {
        unchecked
        {
            var hash = 17;
            hash = (hash * 23) + Revision.GetHashCode();
            hash = (hash * 23) + Quality.GetHashCode();
            return hash;
        }
    }

    public bool Equals(QualityModel? other)
    {
        if (ReferenceEquals(null, other))
        {
            return false;
        }

        if (ReferenceEquals(this, other))
        {
            return true;
        }

        return other.Quality.Id.Equals(Quality.Id) && other.Revision.Equals(Revision);
    }

    public override bool Equals(object? obj)
    {
        if (ReferenceEquals(null, obj))
        {
            return false;
        }

        if (ReferenceEquals(this, obj))
        {
            return true;
        }

        return Equals(obj as QualityModel);
    }

    public static bool operator ==(QualityModel? left, QualityModel? right)
    {
        return Equals(left, right);
    }

    public static bool operator !=(QualityModel? left, QualityModel? right)
    {
        return !Equals(left, right);
    }
}
