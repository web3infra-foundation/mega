## Build Images

```bash
# cd root of the project

# build postgres image
docker buildx build -t mega:mono-pg-latest -f ./docker/mono-pg-dockerfile .

# build backend mono engine image (default in release mode)
docker buildx build -t mega:mono-engine-latest -f ./docker/mono-engine-dockerfile .

# build backend mono engine in debug mode
# docker buildx build -t mega:mono-engine-latest-debug -f ./docker/mono-engine-dockerfile --build-arg BUILD_TYPE=debug .

# build frontend mono ui image
docker buildx build -t mega:mono-ui-latest-release -f ./docker/mono-ui-dockerfile .

## Test Mono Engine

[1] Initiate volume for mono data and postgres data

```bash
# Linux or MacOS
sudo ./docker/init-volume.sh /mnt/data ./config/config.toml

# Windows
# Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
# .\init-volume.ps1 -baseDir "D:\" -configFile ".\config.toml"
```

[2] Start whole mono engine stack on server with domain

```bash
# create network
docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network --memory=4g -v /mnt/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mega:mono-pg-latest-release
docker run --rm -it -d --name mono-engine --network mono-network --memory=8g -v /mnt/data/mono/mono-data:/opt/mega -p 8000:8000 -p 22:9000 mega:mono-engine-latest-release
docker run --rm -it -d --name mono-ui --network mono-network --memory=1g -e MEGA_INTERNAL_HOST=http://mono-engine:8000 -e MEGA_HOST=https://git.gitmega.net -p 3000:3000 mega:mono-ui-latest-release
```

[3] Nginx configuration for Mono

```Nginx
server {
    listen 80;
    listen [::]:80;
    server_name git.gitmega.org;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl;
    listen [::]:443 ssl;

    server_name git.gitmega.org;

    ssl_certificate /etc/letsencrypt/live/git.gitmega.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/git.gitmega.org/privkey.pem;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305;
    ssl_prefer_server_ciphers on;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    ssl_session_tickets off;

    ssl_stapling on;
    ssl_stapling_verify on;
    resolver 8.8.8.8 8.8.4.4 valid=300s;
    resolver_timeout 5s;

    add_header Strict-Transport-Security "max-age=63072000" always;

    client_max_body_size 5G;

    access_log /var/log/nginx/git.gitmega.access.log;
    error_log /var/log/nginx/git.gitmega.error.log;

    location / {
        proxy_pass  http://127.0.0.1:8000;
    }

}

server {
    listen 80;
    listen [::]:80;
    server_name console.gitmega.org;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl;
    listen [::]:443 ssl;

    server_name console.gitmega.org;

    ssl_certificate /etc/letsencrypt/live/console.gitmega.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/console.gitmega.org/privkey.pem;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305;
    ssl_prefer_server_ciphers on;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    ssl_session_tickets off;

    ssl_stapling on;
    ssl_stapling_verify on;
    resolver 8.8.8.8 8.8.4.4 valid=300s;
    resolver_timeout 5s;

    add_header Strict-Transport-Security "max-age=63072000" always;

    access_log /var/log/nginx/console.gitmega.access.log;
    error_log /var/log/nginx/console.gitmega.error.log;

    location / {
        proxy_pass  http://127.0.0.1:3000;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;

        proxy_set_header Origin $scheme://$host;
    }

}

```

## Certbot for SSL Certificate

[1] Install certbot

```bash
sudo apt install certbot
```

[2] Get SSL certificate

```bash
certbot certonly -d "*.gitxxx.org" -d gitxxx.org --manual --preferred-challenges dns-01 --server https://acme-v02.api.letsencrypt.org/directory
```

[3] List SSL certificate

```bash
ls /etc/letsencrypt/live/gitxxx.org
```