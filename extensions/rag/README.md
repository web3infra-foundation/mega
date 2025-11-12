# Rag

## What's the Rag

Rag is a framework for retrieval-augmented generation based on large models. It can traverse specified code repositories, extracting information such as functions and structures, providing a foundation for subsequent code vectorization and retrieval enhancement.

The Rag project consists of multiple modules, each responsible for different functionalities:
- `index`: Used for updating the knowledge base.
- `chat`: Provides an interface for user interaction, used for retrieval enhancement based on the knowledge base.

## What are the features?

- **Flow Programming Paradigm Based on dagrs**: Implements parallel task processing to improve efficiency and performance.
  
- **Dynamic Knowledge Fusion**: 
  - Injects the latest code repository content (such as Mega's Git commits, document changes) into the generation process in real-time, breaking the static knowledge limitations of traditional LLMs.

- **Verifiable Credibility**: 
  - All generated results come with traceability information, allowing direct location to the source code.

- **Incremental Learning Capability**: 
  - Automatically synchronizes changes in the Mega repository without the need for full retraining, keeping the knowledge base up to date.

## How to use Rag

1. Pull docker images from Docker Hub

```bash
$ docker pull genedna/mega:mono-pg-latest
$ docker pull genedna/mega:mono-engine-latest
$ docker pull genedna/mega:mono-ui-latest
```

2. Initialize for mono-engine and PostgreSQL

```bash
$ git clone https://github.com/web3infra-foundation/mega.git
$ cd mega
# Linux or MacOS
sudo ./docker/init-volume.sh /mnt/data ./config/config.toml
```

3. Run the mono-engine and PostgreSQL with docker, and open the mono-ui in your browser with `http://localhost:3000`.

```bash
# create network
$ docker network create mono-network

# run postgres
docker run --rm -it -d --name mono-pg --network mono-network --memory=4g -v /mnt/data/mono/pg-data:/var/lib/postgresql/data -p 5432:5432 mega:mono-pg-latest
docker run --rm -it -d --name mono-engine --network mono-network --memory=8g -v /mnt/data/mono/mono-data:/opt/mega -p 8000:8000 -p 22:9000 mega:mono-engine-latest
docker run --rm -it -d --name mono-ui --network mono-network --memory=1g -e MEGA_INTERNAL_HOST=http://mono-engine:8000 -e MEGA_HOST=https://git.gitmega.net -p 3000:3000 mega:mono-ui-latest
```

4. Run Kafka for message queueing.

   ```bash
   # Pull the image (optional, docker run will do it automatically)
   docker pull bitnami/kafka:3.5

   # Run the Kafka container
   docker run --rm -d \
    --name kafka \
    -p 9092:9092 \
    --network mono-network \
    -e KAFKA_KRAFT_CLUSTER_ID=zCG7EfxhRg6MgefynF9sEw== \
    -e KAFKA_CFG_NODE_ID=1 \
    -e KAFKA_CFG_PROCESS_ROLES=broker,controller \
    -e KAFKA_CFG_CONTROLLER_QUORUM_VOTERS=1@kafka:9093 \
    -e KAFKA_CFG_LISTENERS=PLAINTEXT://:9092,CONTROLLER://:9093 \
    -e KAFKA_CFG_ADVERTISED_LISTENERS=PLAINTEXT://kafka:9092 \
    -e KAFKA_CFG_LISTENER_SECURITY_PROTOCOL_MAP=CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT \
    -e KAFKA_CFG_CONTROLLER_LISTENER_NAMES=CONTROLLER \
    -e ALLOW_PLAINTEXT_LISTENER=yes \
    bitnami/kafka:3.5
   ```

5. Install and configure Ollama for local LLM inference:
   ```bash
   # Pull the Ollama image
   docker pull ollama/ollama
   
   # Run the Ollama container
   docker run -d --network mono-network -v ollama:/root/.ollama -p 11434:11434 --name ollama ollama/ollama

   # Pull models into the Ollama container
   docker exec -it ollama ollama pull bge-m3
   docker exec -it ollama ollama pull deepseek-r1
   ```

6. Pull the vector database Qdrant and run the `chat` and `index` modules:

   - Run Qdrant:
     ```bash
     docker run --rm -it -d --name qdrant --network mono-network -p 6333:6333 -p 6334:6334 \
         -e QDRANT__SERVICE__GRPC_PORT="6334" \
         qdrant/qdrant
     ```

   - Run the `index` module:
     ```bash
     docker build -t rag-index -f ./extensions/rag/index/Dockerfile .
     docker run --rm  -d --name rag-index --network mono-network  -v /mnt/data:/opt/data --add-host=git.gitmega.nju:172.17.0.1 --env-file ./extensions/rag/index/.env  rag-index \
     ```

   - Run the `chat` module:
     ```bash
     docker build -t rag-chat -f ./extensions/rag/chat/Dockerfile .
     docker run --rm -it  --name rag-chat --network mono-network --env-file ./extensions/rag/env -p 30088:30088 rag-chat
     ```

     **Note**: The `chat` module listens on port 30088 inside the container. The `-p 30088:30088` flag maps the container's port 30088 to the host's port 30088, allowing external access to the chat API.

     **Access the chat API**:
     ```bash
     # Test the chat endpoint
     curl -X POST http://localhost:30088/chat \
       -H "Content-Type: application/json" \
       -d '{"prompt": "your question here"}'
     
     # Or from another container in the same network
     curl -X POST http://rag-chat:30088/chat \
       -H "Content-Type: application/json" \
       -d '{"prompt": "your question here"}'

    curl -X POST http://0.0.0.0:30088/get_cve_full_info \
      -H "Content-Type: application/json" \
      -d '{
      "id": "RUSTSEC-2025-0045",
      "subtitle": "ConstStaticCell passes non-Send values across threads",
      "reported": "2025-07-17",
      "issued": "2025-07-17",
      "package": "static_cell",
      "ttype": "INFO Unsound",
      "keywords": "#send #thread-safety",
      "aliases": "",
      "reference": "https://github.com/embassy-rs/static-cell/issues/19",
      "patched": ">=2.1.1",
      "unaffected": "<=2.0.0",
      "description": "ConstStaticCell<T> could have been used to pass non-Send values to another thread, because T was not required to be Send while ConstStaticCell is Send. This was corrected by introducing a T: Send bound.",
      "affected": "static_cell::ConstStaticCell::new (version 2.1.0)"
    }'
     ```

Please adjust and supplement according to the specific functions and needs of the project. If there is any other specific information that needs to be added, please let me know! I will update the `README.md` file.