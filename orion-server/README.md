# Orion Server

Orion Server is a Buck2 build task scheduling service written in Rust. It provides distributed build task dispatching, status tracking, and log collection via WebSocket and HTTP APIs, making it suitable for scenarios requiring automated and distributed builds.

## Features

- **Buck2 Build Scheduling**: Supports dispatching build tasks to multiple build clients via WebSocket, enabling distributed builds.
- **Task Status Query**: Query the status of build tasks (in progress, completed, failed, interrupted, etc.) through RESTful APIs.
- **Real-time Log Output**: Pushes build logs in real time via Server-Sent Events (SSE), allowing frontends or other services to display build output live.
- **Query History by MR**: Supports querying historical build records by Merge Request (MR) number.
- **Task Deduplication and Assignment**: Automatically selects idle build clients for task assignment, supporting concurrent multi-client builds.
- **Persistent Storage**: Persists build tasks and their metadata in a database for later tracing and analysis.

## Main Modules

- `model::builds`: Defines the database model for build tasks, including task ID, output files, status codes, start/end times, repository name, build targets, parameters, MR number, and provides interfaces for querying by task ID.
- `api`: Implements all external APIs, including:
    - WebSocket connection management and message protocol (task dispatch, log reporting, status feedback, etc.)
    - Build task submission interface
    - Task status query interface
    - Real-time build log output interface (SSE)
    - Historical build query interface by MR

## Typical Workflow

1. **Clients (build agents) connect to the server via WebSocket and register as available build nodes.**
2. **Users submit new build tasks via HTTP API; the server automatically assigns them to idle build clients.**
3. **Build clients receive tasks, execute Buck2 builds, and report logs and final status in real time via WebSocket.**
4. **The server writes logs to files and pushes them in real time to frontends or other subscribers via SSE.**
5. **Upon task completion, the server persists the results to the database, supporting later queries by task ID or MR.**

## API


#### 1. WebSocket Endpoint

- **`/ws`**  
    Establishes a WebSocket connection for build clients (agents).  
    - **Purpose:** Register build clients, receive build tasks, and report build logs/status.
    - **Protocol:** Custom JSON messages (see `WSMessage` in code).

#### 2. Submit Build Task

- **`POST /task`**  
    Submits a new build task to the server.
    - **Request Body:**  
        ```json
        {
            "repo": "string",
            "target": "string",
            "args": ["string", ...],      // optional
            "mr": "string"                // optional, Merge Request number
        }
        ```
    - **Response:**  
        ```json
        {
            "task_id": "string",
            "client_id": "string"
        }
        ```
    - **Errors:**  
        - `{ "message": "No clients connected" }` if no build agents are available.
```bash
curl -X POST http://localhost:8004/task \
    -H "Content-Type: application/json" \
    -d '{ 
        "repo": "buck2-rust-third-party",
        "target": "root//:rust-third-party",
        "args": [""],
        "mr": "123"
    }'
```
#### 3. Query Task Status

- **`GET /task-status/{id}`**  
    Query the status of a build task by its ID.
    - **Response:**  
        ```json
        {
            "status": "Building|Interrupted|Failed|Completed|NotFound",
            "exit_code": 0,                // optional
            "message": "string"            // optional
        }
        ```
    - **Status Codes:**  
        - `200 OK` if found  
        - `404 Not Found` if task does not exist  
        - `400 Bad Request` if ID is invalid

#### 4. Real-time Task Output (Logs)

- **`GET /task-output/{id}`**  
    Streams real-time build logs for a task using Server-Sent Events (SSE).
    - **Response:**  
        - SSE stream of log lines as they are produced.
        - If the log file does not exist:  
            `data: Task output file not found`

#### 5. Query Builds by Merge Request

- **`GET /mr-task/{mr}`**  
    Query historical build records by Merge Request (MR) number.
    - **Response:**  
        - `200 OK` with a JSON array of build records if found.
        - `404 Not Found` with `{ "message": "No builds found for the given MR" }` if none.
        - `500 Internal Server Error` on database errors.

**Example:**
```bash
curl -X GET http://localhost:8004/mr-task/123
```

**Note:**  
- All endpoints except `/ws` are intended for HTTP clients (frontends, automation, etc.).
- WebSocket clients must implement the protocol defined in `orion::ws::WSMessage` for task handling and reporting.
- SSE endpoints require clients to support Server-Sent Events.
