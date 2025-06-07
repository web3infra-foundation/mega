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
  "status":  "Success",
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
    "mega_url":   "http://example.com",
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
  "mega_url":   "http://example.com",
  "mount_path": "new/mount/path",
  "store_path": "new/store/path"
}
```

**Response (JSON)**:
```json
{
  "status": "Success",
  "config": {
    "mega_url":   "http://example.com",
    "mount_path": "new/mount/path",
    "store_path": "new/store/path"
  }
}
```

### 6. **Git Add**
**URL**: `/api/git/add`  
**Method**: POST  
**Description**: Add added, deleted, and modified files to the temporary storage area.

**Request Body (JSON)**:
```json
{
  "mono_path": "path/to/add",
}
```

**Response (JSON)**:
```json
{
	"status_code": 200,
}
```

### 7. **Git Status**
**URL**: `/api/git/status`  
**Method**: GET  
**Description**: Retrieves the status of the Git repository.

**Query Parameters**:
- `path` : The target path whose status needs to be checked.

**Response (JSON)**:
```json
{
	"status":     "Success",
	"mono_path":  "target/path",
	"upper_path": "upper/folder/of/mono_path",
	"lower_path": "lower/folder/of/mono_path",
	"message":    "Status of mono_path",
}
```

### 8. **Git Commit**
**URL**: `/api/git/commit`
**Method**: POST
**Description**: Commits changes in the Git repository with a given message.

**Request Body (JSON)**:
```json
{
	"mono_path": "commit/path",
	"message":   "Commit message",
}
```

**Response (JSON)**:
```json
{
	"status": "Success",
	"commit": {
		"id":                "The Commit hash",
		"tree_id":           "New hash of root tree",
		"parent_commit_ids": "The hash of last version",
		"author":            "The author of this repository",
		committer:           "The committer of current Commit",
		message:             "Commit message",
	},
	"msg":    "Detailed information",
}
```

### 9. **Git Push**
**URL**: `/api/git/push`
**Method**: POST
**Description**: Pushes committed changes to the remote repository.

**Request Body (JSON)**:
```json
{
	"mono_path": "push/path",
}
```

**Response (JSON)**:
```json
{
  "status_code": 200,
  "output": "Push successful"
}
```

### 10. **Git Reset**
**URL**: `/api/git/reset`
**Method**: POST
**Description**: Reset the repository, undoing all modifications.

**Request Body (JSON)**:
```json
{
	"path": "reset/path",
}
```

**Response (JSON)**:
```json
{
  "status_code": 200,
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

### AddReq
```rust
struct AddReq {
	mono_path: String,
}
```

### GitStatus
```rust
struct GitStatus {
	status: String,
	mono_path: String,
	upper_path: String,
	lower_path: String,
	message: String,
}
```

### GitStatusParams
```rust
struct GitStatusParams {
	path: String,
}
```

### CommitPayload
```rust
struct CommitPayload {
	mono_path: String,
	message: String,
}
```

### CommitResp
```rust
struct CommitResp {
	status: String,
	commit: Option<Commit>,
	msg: String,
}
```

### PushRequest
```rust
struct PushRequest {
	mono_path: String,
}
```

### ResetReq
```rust
struct ResetReq {
	path: String,
}
```