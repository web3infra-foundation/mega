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

# build aries engine image
docker buildx build -t aries-engine:0.1-pre-release -f ./docker/aries-engine-dockerfile .

# build mega engine image
docker buildx build -t mega-engine:0.1-pre-release -f ./docker/mega-engine-dockerfile .
```

## Test Mono Engine

[1] Initiate volume for mono data and postgres data

```bash
# Linux or MacOS
./init-volume.sh /mnt/data ./config.toml

# Windows
# Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
# .\init-volume.ps1 -baseDir "D:\" -configFile ".\config.toml"
```

[2] Start whole mono engine stack on local for testing
```bash
# create network
docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network -v /tmp/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mono-pg:0.1-pre-release
docker run --rm -it -d --name mono-engine --network mono-network -v /tmp/data/mono/mono-data:/opt/mega -p 8000:8000 mono-engine:0.1-pre-release
docker run --rm -it -d --name mono-ui --network mono-network -e MEGA_INTERNAL_HOST=http://mono-engine:8000 -e MEGA_HOST=http://localhost:8000 -p 3000:3000 mono-ui:0.1-pre-release
```

[3] Start whole mono engine stack on server with domain

```bash
# create network
docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network --memory=4g -v /mnt/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mono-pg:0.1-pre-release
docker run --rm -it -d --name mono-engine --network mono-network --memory=8g -v /mnt/data/mono/mono-data:/opt/mega -p 8000:8000 -p 22:9000 mono-engine:0.1-pre-release
docker run --rm -it -d --name mono-ui --network mono-network --memory=1g -e MEGA_INTERNAL_HOST=http://mono-engine:8000 -e MEGA_HOST=https://git.gitmono.com -p 3000:3000 mono-ui:0.1-pre-release
```

[4] Nginx configuration for Mono

```Nginx
server {
    listen 443;
    listen [::]:443;

    server_name git.gitxxx.org;

    ssl_certificate /etc/letsencrypt/live/gitxxx.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/gitxxx.org/privkey.pem;

    access_log /var/log/nginx/git.gitxxx.access.log;
    error_log /var/log/nginx/git.gitxxx.error.log;

    location / {
        proxy_pass  http://127.0.0.1:8000;
    }
}

server {
    listen 443;
    listen [::]:443;

    server_name console.gitxxx.org;

    ssl_certificate /etc/letsencrypt/live/gitxxx.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/gitxxx.org/privkey.pem;

    access_log /var/log/nginx/console.gitxxx.access.log;
    error_log /var/log/nginx/console.gitxxx.error.log;

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

## Test Aries engine

[1] Initiate volume for aries and postgres data

```bash
# Linux or MacOS
./init-volume.sh /mnt/data ./config.toml

# Windows
# Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
# .\init-volume.ps1 -baseDir "D:\" -configFile ".\config.toml"
```

[2] Start whole aries engine stack on local for testing

```bash
# create network
docker network create aries-network

# run postgres and aries engine
docker run --rm -it -d --name mono-pg --network aries-network -v /mnt/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mono-pg:0.1-pre-release
docker run --rm -it -d --name aries-engine --network aries-network -e HUB_HOST=aries-engine  -v /mnt/data/mono/mono-data:/opt/mega -p 8001:8001 -p 8888:8888 aries-engine:0.1-pre-release
docker run --rm -it -d --name mega-engine --network aries-network -v /mnt/data/mono/mono-data:/opt/mega -p 8000:8000 mega-engine:0.1-pre-release
```

[3] Nginx configuration for Aries

```Nginx
  server {
    listen       443 ssl;
    server_name  gitxxx.org;

    ssl_certificate /home/ubuntu/ssl/gitxxx-org-fullchain.pem;
    ssl_certificate_key /home/ubuntu/ssl/gitxxx-org-key.pem;

    location /relay/ {
        proxy_pass  http://127.0.0.1:8001/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
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

## Deply Mono Engine with Kubernetes

[1] Create Persistent Volume and Persistent Volume Claim
```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: pg-data-pv
spec:
  capacity:
    storage: 1Gi
  accessModes:
    - ReadWriteOnce
  hostPath:
    path: /tmp/data/mono/pg-data
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: pg-data-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
```

```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: mono-data-pv
spec:
  capacity:
    storage: 1Gi
  accessModes:
    - ReadWriteOnce
  hostPath:
    path: /tmp/data/mono/mono-data
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: mono-data-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
```

[2] Create Deployment and Service for Mono Engine and Postgres
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mono-pg
spec:
  replicas: 1
  selector:
    matchLabels:
      app: mono-pg
  template:
    metadata:
      labels:
        app: mono-pg
    spec:
      containers:
        - name: mono-pg
          image: mono-pg:0.1-pre-release
          ports:
            - containerPort: 5432
          volumeMounts:
            - name: pg-data
              mountPath: /var/lib/postgresql/data
      volumes:
        - name: pg-data
          persistentVolumeClaim:
            claimName: pg-data-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: mono-pg-service
spec:
  selector:
    app: mono-pg
  ports:
    - protocol: TCP
      port: 5432
      targetPort: 5432
  type: NodePort
```

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mono-engine
spec:
  replicas: 1
  selector:
    matchLabels:
      app: mono-engine
  template:
    metadata:
      labels:
        app: mono-engine
    spec:
      containers:
        - name: mono-engine
          image: mono-engine:0.1-pre-release
          ports:
            - containerPort: 8000
          volumeMounts:
            - name: mono-data
              mountPath: /opt/mega
      volumes:
        - name: mono-data
          persistentVolumeClaim:
            claimName: mono-data-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: mono-engine-service
spec:
  selector:
    app: mono-engine
  ports:
    - protocol: TCP
      port: 8000
      targetPort: 8000
  type: NodePort
```

[3] Create Deployment and Service for Mono UI
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mono-ui
spec:
  replicas: 1
  selector:
    matchLabels:
      app: mono-ui
  template:
    metadata:
      labels:
        app: mono-ui
    spec:
      containers:
        - name: mono-ui
          image: mono-ui:0.1-pre-release
          ports:
            - containerPort: 3000
          env:
            - name: MEGA_INTERNAL_HOST
              value: http://mono-engine-service:8000
            - name: MEGA_HOST
              value: http://localhost:8000
---
apiVersion: v1
kind: Service
metadata:
  name: mono-ui-service
spec:
  selector:
    app: mono-ui
  ports:
    - protocol: TCP
      port: 3000
      targetPort: 3000
  type: NodePort
```
