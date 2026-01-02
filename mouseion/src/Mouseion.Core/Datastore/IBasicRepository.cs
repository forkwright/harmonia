// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Datastore;

public interface IBasicRepository<TModel> where TModel : ModelBase, new()
{
    // Async methods (primary)
    Task<IEnumerable<TModel>> AllAsync(CancellationToken ct = default);
    Task<int> CountAsync(CancellationToken ct = default);
    Task<IEnumerable<TModel>> GetPageAsync(int page, int pageSize, CancellationToken ct = default);
    Task<TModel> GetAsync(int id, CancellationToken ct = default);
    Task<TModel?> FindAsync(int id, CancellationToken ct = default);
    Task<TModel> InsertAsync(TModel model, CancellationToken ct = default);
    Task<TModel> UpdateAsync(TModel model, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);
    Task<IEnumerable<TModel>> GetAsync(IEnumerable<int> ids, CancellationToken ct = default);
    Task InsertManyAsync(IList<TModel> models, CancellationToken ct = default);
    Task UpdateManyAsync(IList<TModel> models, CancellationToken ct = default);

    // Synchronous methods (legacy - will be removed)
    IEnumerable<TModel> All();
    int Count();
    TModel Get(int id);
    TModel? Find(int id);
    TModel Insert(TModel model);
    TModel Update(TModel model);
    void Delete(int id);
    IEnumerable<TModel> Get(IEnumerable<int> ids);
    void InsertMany(IList<TModel> models);
    void UpdateMany(IList<TModel> models);
}
