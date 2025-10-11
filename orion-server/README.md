# Orion Server

Orion Server is a Buck2 build task scheduling service written in Rust. It provides distributed build task dispatching, status tracking, and log collection via WebSocket and HTTP APIs, making it suitable for scenarios requiring automated and distributed builds.

## Features

- **Buck2 Build Scheduling**: Supports dispatching build tasks to multiple build clients via WebSocket, enabling distributed builds.
- **Task Status Query**: Query the status of build tasks (in progress, completed, failed, interrupted, etc.) through RESTful APIs.
- **Real-time Log Output**: Pushes build logs in real time via Server-Sent Events (SSE), allowing frontends or other services to display build output live.
- **Query History by CL**: Supports querying historical build records by Change List (CL) number.
- **Task Deduplication and Assignment**: Automatically selects idle build clients for task assignment, supporting concurrent multi-client builds.
- **Persistent Storage**: Persists build tasks and their metadata in a database for later tracing and analysis.

## Main Modules

- `model::builds`: Defines the database model for build tasks, including task ID, output files, status codes, start/end times, repository name, build targets, parameters, CL number, and provides interfaces for querying by task ID.
- `api`: Implements all external APIs, including:
    - WebSocket connection management and message protocol (task dispatch, log reporting, status feedback, etc.)
    - Build task submission interface
    - Task status query interface
    - Real-time build log output interface (SSE)
    - Historical build query interface by CL

## Typical Workflow

1. **Clients (build agents) connect to the server via WebSocket and register as available build nodes.**
2. **Users submit new build tasks via HTTP API; the server automatically assigns them to idle build clients.**
3. **Build clients receive tasks, execute Buck2 builds, and report logs and final status in real time via WebSocket.**
4. **The server writes logs to files and pushes them in real time to frontends or other subscribers via SSE.**
5. **Upon task completion, the server persists the results to the database, supporting later queries by task ID or CL.**

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
            "cl": "string"                // optional, Change List number
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
        "cl": "123"
    }'
```
#### 3. Start Build Task

- **`POST /task-build/{id}`**
    Start a build task that was previously created.
    - **Request Body:**
        ```json
        {
            "repo": "string",
            "buck_hash": "string",
            "buckconfig_hash": "string",
            "args": ["string", ...],      // optional
            "cl": "string"                // optional, Change List number
        }
        ```
    - **Response:**
        ```json
        {
            "task_id": "string",
            "build_id": "string",
            "client_id": "string",
            "status": "dispatched"
        }
        ```
    - **Errors:**
        - `{ "message": "Invalid task ID format" }` if ID is invalid.
        - `{ "message": "Task ID does not exist" }` if task not found.
        - `{ "message": "No available workers at the moment" }` if no build agents are available.

#### 4. Query Task Status

- **`GET /task-status/{id}`**
    Query the status of a build task by its ID.
    - **Response:**
        ```json
        {
            "status": "Building|Interrupted|Failed|Completed|NotFound|Pending",
            "exit_code": 0,                // optional
            "message": "string"            // optional
        }
        ```
    - **Status Codes:**
        - `200 OK` if found
        - `404 Not Found` if task does not exist
        - `400 Bad Request` if ID is invalid

#### 5. Get Task Build IDs

- **`GET /task-build-list/{id}`**
    Get a list of build IDs associated with a task.
    - **Response:**
        ```json
        ["build_id1", "build_id2", ...]
        ```
    - **Status Codes:**
        - `200 OK` if found
        - `404 Not Found` if task does not exist
        - `400 Bad Request` if ID is invalid

**Example:**
```bash
curl -X GET http://localhost:8004/cl-task/123
```

**Note:**
- All endpoints except `/ws` are intended for HTTP clients (frontends, automation, etc.).
- WebSocket clients must implement the protocol defined in `orion::ws::WSMessage` for task handling and reporting.
- SSE endpoints require clients to support Server-Sent Events.
#### 6. Query Builds by Change List

- **`GET /cl-task/{cl}`**
    Query historical build records by Change List (CL) number.
    - **Response:**
        - `200 OK` with a JSON array of build records if found:
          ```json
          [
            {
              "task_id": "string",
              "build_ids": ["string", ...],
              "output_files": ["string", ...],
              "exit_code": 0,
              "start_at": "2023-01-01T00:00:00Z",
              "end_at": "2023-01-01T00:00:00Z",
              "repo_name": "string",
              "target": "string",
              "arguments": "string",
              "cl": "string"
            }
          ]
          ```
        - `404 Not Found` with `{ "message": "No builds found for the given CL" }` if none.
        - `500 Internal Server Error` on database errors.

#### 7. Query Builds with Status by Change List

- **`GET /tasks/{cl}`**
    Query all tasks with their current status by Change List (CL) number.
    - **Response:**
        - `200 OK` with a JSON array of build records with status if found:
          ```json
          [
            {
              "build_id": ["string", ...],
              "output_files": ["string", ...],
              "exit_code": 0,
              "start_at": "2023-01-01T00:00:00Z",
              "end_at": "2023-01-01T00:00:00Z",
              "repo_name": "string",
              "target": "string",
              "arguments": "string",
              "cl": "string",
              "status": "Building|Interrupted|Failed|Completed|NotFound|Pending"
            }
          ]
          ```
        - `500 Internal Server Error` on database errors.

#### 8. Query Historical Task Logs

- **`GET /task-history-output/{id}`**
    Provides the ability to read historical task logs, supporting either retrieving the **entire log at once** or **retrieving by line segments**.

    - **Path Parameters:**
        - `id` *(string)*: Task ID whose log to read.

    - **Query Parameters:**
        - `type` *(string, required)*: Type of log retrieval.
          - `"full"` → Return the entire log file.
          - `"segment"` → Return a portion of the log by line number and limit.
        - `offset` *(integer, optional)*: Starting line number (**1-based**). Defaults to `1`.
        - `limit` *(integer, optional)*: Maximum number of lines to return. Defaults to `4096`.

    - **Responses:**
        - `200 OK` → Returns the log content in JSON:
            ```json
            { 
              "data": ["line1", "line2", "..."], 
              "len": 100 
            }
            ```
          - `400 Bad Request` → Invalid parameters:
            ```json
            { "message": "Invalid type" }
            ```
        - `404 Not Found` → Log file does not exist:
            ```json
            { "message": "Error: Log File Not Found" }
            ```


    - **Examples:**

        Retrieve the full log:
        ```bash
        curl -X GET "http://localhost:8004/task-history-output/abc123?type=full"
        ```

        Retrieve log lines 100–150:
        ```bash
        curl -X GET "http://localhost:8004/task-history-output/abc123?type=segment&offset=100&limit=50"
        ```
#### 9. Real-time Task Output via SSE

- **`GET /task-output/{id}`**
    Streams the build output logs for a specific task in real time using Server-Sent Events (SSE).
    - **Path Parameter:**
        - `id` — Task ID for which to stream the logs.
    - **Response:**
        - `200 OK` — A continuous SSE stream of log lines as they are produced. Each log line is sent as an SSE `data` event.
        - `404 Not Found` — If the log file for the given task ID does not exist.
    - **Behavior:**
        - Starts streaming from the end of the log file.
        - Keeps the connection alive, sending heartbeat comments every 15 seconds to prevent client timeouts.
        - Continues streaming until the build completes and no new logs are appended.
    - **Example using `curl`:**
    ```bash
    curl -N http://localhost:8004/task-output/<task_id>
    ```
    - **Notes:**
        - Replace `<task_id>` with the actual task ID returned from the `/task` endpoint.
        - The SSE stream sends both new log lines (`data`) and periodic heartbeat comments (`: heartbeat`).
        - Frontend clients should handle incremental updates as log lines arrive.
