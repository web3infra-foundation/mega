# SQL File Update Guide

Whenever making any updates to the SQL content in the project, **make sure to also update** both the `pg_YYYYMMDD__init.sql`, `sqlite_YYYYMMDD__init.sql` files, and the **Dockerfile**. These files are used to initialize the database for PostgreSQL and SQLite respectively, ensuring that SQL changes are properly applied across different environments. Failing to update these files may lead to database inconsistencies, disrupting the system’s operation.

## Filename Date Convention

The middle part of the filename (`YYYYMMDD`, e.g., `20240205` in `pg_20240205__init.sql`) represents the **date of the last modification**. When updating these SQL files, you must update this date to the **current modification date**. This ensures that the file accurately reflects when the last changes were made, aiding in version control and troubleshooting.

For example, if you make modifications on September 5, 2024, the filenames should be updated to:
- `pg_20240905__init.sql`
- `sqlite_20240905__init.sql`

## Dockerfile Update

Mega's Docker setup relies on these SQL files for initializing the database, you **must also update the `Dockerfile`** to ensure the new SQL initialization files are copied and applied correctly. Make sure the Docker build includes the latest `pg_YYYYMMDD__init.sql` and `sqlite_YYYYMMDD__init.sql` filenames and paths, reflecting the current modification date.

For example, update the `Dockerfile` to reference the newly updated filenames:

```Dockerfile
# PostgreSQL initialization
COPY pg_20240905__init.sql /docker-entrypoint-initdb.d/

# SQLite initialization
COPY sqlite_20240905__init.sql /app/sqlite/
```

## Update Process

1. **Modify SQL Statements**  
   When modifying the database structure (such as tables, indexes, constraints, etc.) or data, first make the changes in the corresponding SQL file in the project.

2. **Sync Changes to `pg_YYYYMMDD__init.sql`, `sqlite_YYYYMMDD__init.sql`, and the Dockerfile**  
   Apply the SQL changes to both initialization files:
   - `pg_YYYYMMDD__init.sql` for PostgreSQL
   - `sqlite_YYYYMMDD__init.sql` for SQLite
   
   Update the `YYYYMMDD` part of the filename to reflect the current date of modification. Additionally, ensure that the `Dockerfile` is updated to reference the new SQL files.

3. **Test Database Migration**  
   Ensure the updated SQL files and the new `Dockerfile` are applied in the development and test environments. Verify that the database changes are successful for both PostgreSQL and SQLite.

4. **Commit and Update Documentation**  
   When committing your code, ensure the updates to both `pg_YYYYMMDD__init.sql`, `sqlite_YYYYMMDD__init.sql`, and the `Dockerfile` are included, with the correct date in the filenames, and document the related database changes in the project’s change log.

## Example

For instance, if you add a new table to the project:

```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    email VARCHAR(100) UNIQUE
);
```

You need to add this SQL to both the `pg_YYYYMMDD__init.sql` and `sqlite_YYYYMMDD__init.sql` files, adapting the SQL syntax as needed for each database system. If the modification is made on September 5, 2024, the filenames should be updated to `pg_20240905__init.sql` and `sqlite_20240905__init.sql`. Update the `Dockerfile` to reference these new filenames for PostgreSQL and SQLite initialization.
