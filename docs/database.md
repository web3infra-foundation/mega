# Database

You need to install and execute SQL files in a specific order. 

For example using `PostgreSQL`, execute the files under `sql\postgres` in the following sequence:

1. pg_20230803__init.sql


or if your are using `Mysql`, execute scripts:

1. mysql_20230523__init.sql



## Generating entities: 
`sea-orm-cli generate entity -u "mysql://${DB_USERNAME}:${DB_SECRET}@${DB_HOST}/mega"  -o database/entity/src` 


<!-- You can use `sea-orm-cli migrate generate create_commit_table --local-time` -->