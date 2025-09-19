# API

## Mega HTTP API

### git protocol related API

HTTP implement for git transfer data between two repositories

1. When the client initially connects the server will immediately respond with a version number, and a listing of each reference it has (all branches and tags) along with the object name that each reference currently points to.

    ```bash
    GET **/info/refs/
    ```

2. Pushing data to a server will invoke the receive-pack process on the server, which will allow the client to tell it which references it should update and then send all the data the server will need for those new references to be complete. Once all the data is received and validated, the server will then update its references to what the client specified.

    ```bash
    GET **/git-receive-pack
    ```

3. When one Git repository wants to get data that a second repository has, the first can fetch from the second. This operation determines what data the server has that the client does not then streams that data down to the client in Pack file format.

    ```bash
    GET **/git-upload-pack
    ```

### git lfs API

The Git LFS client uses an HTTPS server to coordinate fetching and storing large binary objects separately from a Git server.

1. Downloading the Git objects required by the LFS protocol using an object ID.

    ```bash
    GET **/objetcs/:object_id
    ```

2. The client uploads objects through individual PUT requests. The URL and headers are provided by an upload action object.

    ```bash
    PUT **/objetcs/:object_id
    ```

3. The client can request the current active locks for a repository by sending a GET to /locks

    ```bash
    GET **/locks
    ```

4. List Locks for Verification。The client can use the Lock Verification endpoint to check for active locks that can affect a Git push

    ```bash
    POST **/locks/verify
    ```

5. Create Lock: The client sends the following to create a lock by sending a POST to /locks.Servers should ensure that users have push access to the repository, and that files are locked exclusively to one user.

    ```bash
    POST **/locks
    ```

6. Delete Lock: The client can delete a lock, given its ID, by sending a POST to /locks/:id/unlock

    ```bash
    POST **/locks/:id/unlock
    ```

7. The Batch API is used to request the ability to transfer LFS objects with the LFS server. The Batch URL is built by adding /objects/batch to the LFS server URL.

    ```bash
    POST **/objects/batch
    ```

### git objects retrieval API

This part of the API, prefixed with /api/v1, is primarily for fetching Git raw objects and displaying web project hierarchies.

> Suppose the mega server is running on `MEGA_URL`, while placeholders surrounded by `<>` is necessary and `[]` is optional, but both are needed to be replaced if chosen.

1. Retrieve original information of a Git object by object ID and return as String.

    ```bash
    curl -X GET ${MEGA_URL}/api/v1/blob?object_id=<id>
    ```

2. Retrieve a Git object by object ID and return it as a file stream
   
    ```bash
    curl -X GET ${MEGA_URL}/api/v1/object?object_id=<id>&repo_path=<path/to/repo>
    ```

3. Retrieve directory hierarchy via path or object ID. The default value for `object_id` is `None` and `repo_path` is `/`
   
    ```bash
    curl -X GET ${MEGA_URL}/api/v1/tree?[object_id=<id>][&][repo_path=<path/to/repo>]
    ```

4. Check `API service` status

    ```bash
    curl -X GET ${MEGA_URL}/api/v1/status
    ```

5. Count number of objects of a given repository

    ```bash
    curl -X GET ${MEGA_URL}/api/v1/count-objs?repo_path=<path/to/repo>
    ```
    
6. Update commit binding to associate a commit with a specific user or mark as anonymous

    ```bash
    curl -X PUT ${MEGA_URL}/api/v1/commits/<commit_sha>/binding \
      -H "Content-Type: application/json" \
      -d '{"username": "<username>", "is_anonymous": false}'
    ```
