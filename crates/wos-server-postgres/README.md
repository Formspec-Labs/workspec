# wos-server-postgres

PostgreSQL storage backend for `wos-server`.

## Existing databases and `kernels` primary key

`migrate()` still uses `CREATE TABLE IF NOT EXISTS kernels (...) PRIMARY KEY (url, version)` for greenfield installs. For databases left on the historical **`PRIMARY KEY (url)`** layout, `migrate()` then runs an in-place upgrade: it drops the old primary-key constraint and adds `PRIMARY KEY (url, version)` when the table exists, the current PK is exactly on `url`, and there are no duplicate `(url, version)` rows.

After upgrading from a very old binary, verify the constraint:

```sql
SELECT conname, pg_get_constraintdef(oid)
FROM pg_constraint
WHERE conrelid = 'kernels'::regclass AND contype = 'p';
```

If the automatic upgrade fails (for example duplicate `(url, version)` data), repair rows manually before calling `migrate()` again.
