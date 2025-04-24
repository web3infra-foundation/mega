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
**Install Ollama, Large Models, and Embedding Tools**:
   - Install Ollama:
     ```bash
     curl -sSL https://ollama.com/install.sh | bash
     ```

   - Install Large Models:
     ```bash
     ollama install  deepseek-r1:1.5b
     ```

   - Install Embedding Tools:
     ```bash
     ollama install bge-m3
     ```


**Pull the vector database Qdrant and run the `chat` and `index` modules**:
   - Create a network:
     ```bash
     docker network create rag-network
     ```

   - Run Qdrant:
     ```bash
     docker run --rm -it -d --name qdrant --network rag-network -p 6333:6333 -p 6334:6334 \
         -e QDRANT__SERVICE__GRPC_PORT="6334" \
         qdrant/qdrant
     ```

   - Run the `chat` module:
     ```bash
     docker build -t rag-chat -f chat/Dockerfile .
     docker run --rm -it -d --name rag-chat --network rag-network rag-chat
     ```

   - Run the `index` module:
     ```bash
     docker build -t rag-index -f index/Dockerfile .
     docker run --rm -it -d --name rag-index --network rag-network rag-index
     ```

Please adjust and supplement according to the specific functions and needs of the project. If there is any other specific information that needs to be added, please let me know! I will update the `README.md` file.