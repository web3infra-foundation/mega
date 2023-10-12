# Deployment

## DataBase

Mega supports databases such as MySQL, postgreSql, mariadb. You can find the corresponding SQL file in the `sql` folder and initialize the response database.

You can configure database connection information by directly modifying the `.env` file or modifying environment variables,such as  
- `MEGA_DB_POSTGRESQL_URL` 
- `MEGA_DB_MYSQL_URL`.

Alternatively, you can configure the specified environment variables, such as `PG_ USERNAME`, `PG_ SECRET`, etc. Please refer to the `.env` file for details.


## Cache

The git decoding process relies on the git object cache to accelerate the parsing of delta objects. There are currently two types of caching: 
 - relies on the LRU algorithm directly placed in memory.
 - relies on kv databases such as Redis.
 
Please configure `GIT_INTERNATIONAL_DECODE_CACHE_TYEP` for selection (optional types include `lru`, `redis`, and if there is no configuration or incorrect configuration, lru is selected by default). If you choose Redis caching, please use `REDIS_CONFIG` Redis connection address using Config.

For example ,
`REDIS_CONFIG = redis://:{password}@{host}:{port}/0 `