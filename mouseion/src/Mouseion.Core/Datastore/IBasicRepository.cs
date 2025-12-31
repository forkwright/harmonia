namespace Mouseion.Core.Datastore;

public interface IBasicRepository<TModel> where TModel : ModelBase, new()
{
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
