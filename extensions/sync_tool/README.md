# Sync Tool

## Overview

Sync Tool is a utility designed to synchronize code repositories with the Mega system. It provides the following key features:

1. Upload code repositories to the Mega repository system
2. Send Kafka messages to trigger RAG knowledge base updates
3. Ensure code repositories stay synchronized with the Mega system

## Usage

### 1. Build Docker Image

```bash
docker build -t sync-tool -f Dockerfile .
```

### 2. Run Container

```bash
docker run --rm -it -d --name sync-tool --network mono-network sync-tool
```

### 3. Configuration

The container needs to be connected to the `mono-network` to communicate with other services (such as Kafka, Mega, etc.).

## Workflow

1. Receive code repository information
2. Upload code repository to the Mega system
3. Send Kafka messages to notify RAG system for knowledge base updates
4. Complete synchronization process

## Prerequisites

- Ensure the container is properly connected to the `mono-network`
- Ensure Mega system services are running
- Ensure Kafka services are running
