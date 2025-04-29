# API Documentation for Fuse-based File System Management Server

## Overview
This server provides endpoints to manage file system mounting and configuration operations, as well as Git-related actions. Below is the detailed documentation of each endpoint, including request parameters and response structures.

## Endpoints

### 1. **Mount Directory**
**URL**: `/api/fs/mount`  
**Method**: POST  
**Description**: Mounts a directory at the specified path.

**Request Body (JSON)**:
```json
{
  "path": "path/to/directory"
}
```

**Response (JSON)**:
```json
{
  "status": "Success",
  "mount": {
    "hash": "unique_hash",
    "path": "path/to/directory",
    "inode": 12345
  },
  "message": "Directory mounted successfully"
}
```

### 2. **Get All Mounted Directories**
**URL**: `/api/fs/mpoint`  
**Method**: GET  
**Description**: Retrieves a list of all currently mounted directories.

**Response (JSON)**:
```json
{
  "status": "Success",
  "mounts": [
    {
      "hash": "hash1",
      "path": "path/to/directory1",
      "inode": 12345
    },
    {
      "hash": "hash2",
      "path": "path/to/directory2",
      "inode": 67890
    }
  ]
}
```

### 3. **Unmount Directory**
**URL**: `/api/fs/umount`  
**Method**: POST  
**Description**: Unmounts a directory using its path or inode.

**Request Body (JSON)**:
```json
{
  "path": "optional_path/to/directory",
  "inode": 12345
}
```

**Response (JSON)**:
```json
{
  "status": "Success",
  "message": "Directory unmounted successfully"
}
```

### 4. **Get Configuration**
**URL**: `/api/config`  
**Method**: GET  
**Description**: Retrieves the current configuration of the server.

**Response (JSON)**:
```json
{
  "status": "Success",
  "config": {
    "mega_url": "http://example.com",
    "mount_path": "path/to/mount",
    "store_path": "path/to/store"
  }
}
```

### 5. **Update Configuration**
**URL**: `/api/config`  
**Method**: POST  
**Description**: Updates the server configuration.

**Request Body (JSON)**:
```json
{
  "mega_url": "http://example.com",
  "mount_path": "new/mount/path",
  "store_path": "new/store/path"
}
```

**Response (JSON)**:
```json
{
  "status": "Success",
  "config": {
    "mega_url": "http://example.com",
    "mount_path": "new/mount/path",
    "store_path": "new/store/path"
  }
}
```

### 6. **Git Status**
**URL**: `/api/git/status`  
**Method**: GET  
**Description**: Retrieves the status of the Git repository.

**Query Parameters**:
- `filter` (optional): Filter specific output based on the input string.

**Response (JSON)**:
```json
{
  "status_code": 200,
  "output": "Git status output"
}
```

### 7. **Git Commit**
**URL**: `/api/git/commit`  
**Method**: POST  
**Description**: Commits changes in the Git repository with a given message.

**Request Body (JSON)**:
```json
{
  "message": "Commit message"
}
```

**Response (JSON)**:
```json
{
  "status_code": 200,
  "output": "Commit successful"
}
```

### 8. **Git Push**
**URL**: `/api/git/push`  
**Method**: POST  
**Description**: Pushes committed changes to the remote repository.

**Response (JSON)**:
```json
{
  "status_code": 200,
  "output": "Push successful"
}
```

## Data Structures
### MountRequest
```rust
struct MountRequest {
    path: String,
}
```

### MountResponse
```rust
struct MountResponse {
    status: String,
    mount: MountInfo,
    message: String,
}
```

### MountInfo
```rust
struct MountInfo {
    hash: String,
    path: String,
    inode: u64,
}
```

### UmountRequest
```rust
struct UmountRequest {
    path: Option<String>,
    inode: Option<u64>,
}
```

### ConfigRequest
```rust
struct ConfigRequest {
    mega_url: Option<String>,
    mount_path: Option<String>,
    store_path: Option<String>,
}
```

### GitStatusParams
```rust
struct GitStatusParams {
    filter: Option<String>,
}
```

