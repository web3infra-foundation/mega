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

4. Pull the vector database Qdrant and run the `chat` and `index` modules:

   - Run Qdrant:
     ```bash
     docker run --rm -it -d --name qdrant --network mono-network -p 6333:6333 -p 6334:6334 \
         -e QDRANT__SERVICE__GRPC_PORT="6334" \
         qdrant/qdrant
     ```

   - Run the `chat` module:
     ```bash
     docker build -t rag-chat -f ./extensions/rag/chat/Dockerfile .
     docker run --rm -it -d --name rag-chat --network mono-network rag-chat
     ```

   - Run the `index` module:
     ```bash
     docker build -t rag-index -f ./extensions/rag/index/Dockerfile .
     docker run --rm -it -d --name rag-index --network mono-network  -v /mnt/data:/opt/data rag-index \
     ```

5. Install and configure Ollama in the container:
   ```bash
   docker-compose up --build -d

   ```

Please adjust and supplement according to the specific functions and needs of the project. If there is any other specific information that needs to be added, please let me know! I will update the `README.md` file.