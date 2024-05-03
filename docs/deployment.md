# Deployment

## DataBase

Mega supports database is PostgreSQL. You can find the corresponding SQL file in the `sql` folder and initialize the response database.

You can configure database connection information by directly modifying the `.env` file or modifying environment variables, such as  
- `MEGA_DB_POSTGRESQL_URL` 

Alternatively, you can configure the specified environment variables, such as `PG_ USERNAME`, `PG_ SECRET`, etc. Please refer to the `.env` file for details.

## Cache