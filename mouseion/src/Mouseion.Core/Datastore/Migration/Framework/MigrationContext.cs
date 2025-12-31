namespace Mouseion.Core.Datastore.Migration.Framework;

public class MigrationContext
{
    public MigrationType MigrationType { get; }

    public MigrationContext(MigrationType migrationType)
    {
        MigrationType = migrationType;
    }
}
