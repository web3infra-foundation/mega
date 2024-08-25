## Build Images

```bash
# cd root of the project

# build postgres image
docker buildx build -t mono-pg:0.1-pre-release -f ./docker/mono-pg-dockerfile .

# build backend mono image (default in release mode)
docker buildx build -t mono-engine:0.1-pre-release -f ./docker/mono-engine-dockerfile .

# build backend mono in debug mode
# docker buildx build -t mono-engine:0.1-pre-debug -f ./docker/mono-engine-dockerfile --build-arg BUILD_TYPE=debug .

# build frontend moon image
docker buildx build -t mono-ui:0.1-pre-release -f ./docker/mono-ui-dockerfile .
```

## Test mono and moon


### Test with SQLite

```bash
# create network
docker network create mono-network

docker run --rm -it -d --network mono-network --name mono-engine -p 8000:8000 -p 22:9000 mono-engine:0.1-pre-release
docker run --rm -it -d --network mono-network --name mono-ui -e NEXT_PUBLIC_API_URL=http://mono-engine:8000 -p 3000:3000 mono-ui:0.1-pre-release
```

visit http://localhost:3000 to see the frontend

## Test with PostgreSQL

[1] start postgres

```bash
# create network
docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network -v /mnt/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mono-pg:0.1-pre-release
docker run --rm -it -d --name mono-engine --network mono-network -v /mnt/data/mono/mono-data:/opt/mega -p 8000:8000 -p 22:9000 mono-engine:0.1-pre-release
```

[3] edit `config.toml`, change `db_type` to `postgres` and db_url to `postgres://mega:mega@mega-db:5432/mega`

```toml
[database]
db_type = "postgres"

# database connection url
db_url = "postgres://mono:mono@mono-pg:5432/mono"
```

[2] create default config

```bash
docker run --rm -it -d --network mono-network --name mono-engine -v ./mega_base:/opt/mega/etc mega-mono:0.1-pre-release
docker stop mega-mono
```

[3] edit `mega_base/config.toml`, change `db_type` to `postgres` and db_url to `postgres://mega:mega@mega-db:5432/mega`

```toml
[database]
db_type = "postgres"

# used for sqlite
db_path = "${base_dir}/mega.db"

# database connection url
db_url = "postgres://mega:mega@mega-db:5432/mega"
```

[4] Start the mono again, and run the frontend.

```bash
docker run --rm -it -d --network mega-network --name mega-mono -v ./mega_base:/opt/mega/etc mega-mono:0.1-pre-release
docker run --rm -it -d --network mega-network -e NEXT_PUBLIC_API_URL=http://mega-mono:8000 -p 3000:3000 mega-moon:0.1-pre-release
```