// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaFiles.Import.Aggregation;

namespace Mouseion.Core.MediaFiles.Import;

public interface IImportDecisionMaker
{
    Task<List<ImportDecision>> GetImportDecisionsAsync(
        List<MusicFileInfo> musicFiles,
        CancellationToken ct = default);

    List<ImportDecision> GetImportDecisions(List<MusicFileInfo> musicFiles);
}

public class ImportDecisionMaker : IImportDecisionMaker
{
    private readonly IAggregationService _aggregationService;
    private readonly IEnumerable<IImportSpecification> _specifications;
    private readonly ILogger<ImportDecisionMaker> _logger;

    public ImportDecisionMaker(
        IAggregationService aggregationService,
        IEnumerable<IImportSpecification> specifications,
        ILogger<ImportDecisionMaker> logger)
    {
        _aggregationService = aggregationService;
        _specifications = specifications;
        _logger = logger;
    }

    public async Task<List<ImportDecision>> GetImportDecisionsAsync(
        List<MusicFileInfo> musicFiles,
        CancellationToken ct = default)
    {
        var decisions = new List<ImportDecision>();

        foreach (var musicFile in musicFiles)
        {
            var decision = await MakeDecisionAsync(musicFile, ct).ConfigureAwait(false);
            decisions.Add(decision);
        }

        _logger.LogInformation("Import decisions: {Approved} approved, {Rejected} rejected",
            decisions.Count(d => d.Approved),
            decisions.Count(d => !d.Approved));

        return decisions;
    }

    public List<ImportDecision> GetImportDecisions(List<MusicFileInfo> musicFiles)
    {
        var decisions = new List<ImportDecision>();

        foreach (var musicFile in musicFiles)
        {
            var decision = MakeDecision(musicFile);
            decisions.Add(decision);
        }

        _logger.LogInformation("Import decisions: {Approved} approved, {Rejected} rejected",
            decisions.Count(d => d.Approved),
            decisions.Count(d => !d.Approved));

        return decisions;
    }

    private async Task<ImportDecision> MakeDecisionAsync(MusicFileInfo musicFileInfo, CancellationToken ct)
    {
        var augmentedInfo = await _aggregationService.AugmentAsync(musicFileInfo, ct).ConfigureAwait(false);

        var decision = new ImportDecision(augmentedInfo);

        foreach (var specification in _specifications)
        {
            var rejection = await specification.IsSatisfiedByAsync(augmentedInfo, ct).ConfigureAwait(false);
            if (rejection != null)
            {
                decision.AddRejection(rejection);
            }
        }

        return decision;
    }

    private ImportDecision MakeDecision(MusicFileInfo musicFileInfo)
    {
        var augmentedInfo = _aggregationService.Augment(musicFileInfo);

        var decision = new ImportDecision(augmentedInfo);

        foreach (var specification in _specifications)
        {
            var rejection = specification.IsSatisfiedBy(augmentedInfo);
            if (rejection != null)
            {
                decision.AddRejection(rejection);
            }
        }

        return decision;
    }
}
