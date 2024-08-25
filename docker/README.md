## Build Images

```bash
# cd root of the project

# build postgres image
docker buildx build -t mono-pg:0.1-pre-release -f ./docker/mono-pg-dockerfile .

# build backend mono engine image (default in release mode)
docker buildx build -t mono-engine:0.1-pre-release -f ./docker/mono-engine-dockerfile .

# build backend mono engine in debug mode
# docker buildx build -t mono-engine:0.1-pre-debug -f ./docker/mono-engine-dockerfile --build-arg BUILD_TYPE=debug .

# build frontend mono ui image
docker buildx build -t mono-ui:0.1-pre-release -f ./docker/mono-ui-dockerfile .
```

## Test Mono Engine

### Test Mono Engine with SQLite

```bash
# create network
docker network create mono-network

docker run --rm -it -d --network mono-network --name mono-engine -p 8000:8000 -p 22:9000 mono-engine:0.1-pre-release
docker run --rm -it -d --network mono-network --name mono-ui -e NEXT_PUBLIC_API_URL=http://mono-engine:8000 -p 3000:3000 mono-ui:0.1-pre-release
```

visit http://localhost:3000 to see the frontend

### Test Mono Engine with PostgreSQL

[1] Initiate volume for mono data and postgres data

```bash
./init-volume.sh /mnt/data ./config.toml
```

Check `db_type` to `postgres` and db_url to `postgres://mono:mono@mega-db:5432/mono` in the `config.toml`

```toml
[database]
db_type = "postgres"

# database connection url
db_url = "postgres://mono:mono@mono-pg:5432/mono"
```

[2] Start whole mono engine stack

```bash
# create network
docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network -v /mnt/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mono-pg:0.1-pre-release
docker run --rm -it -d --name mono-engine --network mono-network -v /mnt/data/mono/mono-data:/opt/mega -p 8000:8000 -p 22:9000 mono-engine:0.1-pre-release
docker run --rm -it -d --name mono-ui --network mono-network -e NEXT_PUBLIC_API_URL=http://mono-engine:8000 -p 3000:3000 mono-ui:0.1-pre-release
```