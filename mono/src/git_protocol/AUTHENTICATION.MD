## Git HTTP Request Authentication Process

When Git initiates an HTTP request, the authentication process primarily involves how credentials are passed to verify the user’s identity, ensuring access to remote Git repositories. Below is an overview of the authentication process when Git communicates over HTTP/HTTPS.

### 1. Authentication Methods

Git supports several common HTTP/HTTPS authentication methods:

	•	Basic Authentication: Username and password/token are sent in the Authorization header encoded in Base64.
	•	Personal Access Token (PAT): Typically used as a replacement for username and password, especially on services like GitHub, GitLab, etc.
	•	OAuth: Allows for authentication using OAuth tokens, provided the service provider supports it.

Usually, Git uses the Authorization header to carry authentication information, but with HTTPS encryption, the credentials are secure during transmission.

### 2. Authentication Process

(1) Initial Request: No Authentication Information

When the Git client first sends an HTTP request to a remote repository, it typically does not immediately include the Authorization header. It sends a standard request to ask the server if the repository is available.

Example request (without authentication):

```http
GET /path/to/repo.git/info/refs?service=git-upload-pack HTTP/1.1
Host: git.gitmono.com
User-Agent: git/2.46.0
```

(2) Server Response: Requires Authentication

If the remote repository requires authentication to access, the server will respond with a 401 Unauthorized, including a WWW-Authenticate header, indicating that the client needs to authenticate.

Example response:

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="Mega"
```

This informs the Git client that it needs to use Basic Authentication or another supported authentication method.

(3) Client Sends Authentication Information

The Git client will respond to the server’s 401 Unauthorized by looking for locally stored credentials (like username and password or Personal Access Token) or prompting the user for credential input. It will then resend the request, this time including the Authorization header with the authentication information.

In Basic Authentication, Git will combine the username and password/token into a username:password format, encode it in Base64, and place it in the Authorization header.

Example request:

```http
GET /path/to/repo.git/info/refs?service=git-upload-pack HTTP/1.1
Host: git.gitmono.com
User-Agent: git/2.46.0
Authorization: Basic dXNlcm5hbWU6dG9rZW4=
```

Here, dXNlcm5hbWU6dG9rZW4= is the Base64 encoded username:token.

(4) Server Validates Authentication Information

The server will check the credentials in the Authorization header it received:

	•	If authentication is successful, the server will return 200 OK and continue with the Git operation (like pull, push, etc.).
	•	If authentication fails, the server will return 401 Unauthorized and may prompt the user to re-enter their credentials.

(5) Request Ends

Once the credential verification is successful, the Git client can interact with the remote repository. Credentials are typically cached for the session to avoid requiring re-authentication with each request.

### 3. Credential Storage and Management

(1) Credential Storage

To avoid having to manually enter usernames and passwords each time, Git provides various credential storage methods:

	•	Credential Helper: Git’s credential.helper allows users to save and cache credentials for future requests automatically.
	•	credential.helper=cache: Caches credentials for a period.
	•	credential.helper=osxkeychain: Uses the system’s keychain service to store credentials on macOS.
	•	credential.helper=store: Saves credentials in plain text in the user’s directory (not recommended).

git config --global credential.helper cache

### 4. How to Debug Authentication Requests

The following environment variables can be used to debug the Git HTTP request authentication process:

	•	GIT_CURL_VERBOSE=1: Used for debugging Git HTTP requests, including HTTPS requests and authentication processes.
	•	Although the Authorization header will be hidden for security reasons, you can check the response status codes (like 401 Unauthorized) to determine if authentication is successful.
	•	GIT_TRACE=1: Used for debugging Git’s internal operations, including credential lookup and handling.

Example Debug Command:

```bash
GIT_CURL_VERBOSE=1 GIT_TRACE=1 git push
```

### 5. Common Issues

	•	Credentials Not Saved: If you find that Git prompts for a username and password every time, check whether credential.helper is configured correctly.
	•	Authorization Header Not Displayed: For security reasons, the Git client does not display the full Authorization header in debug output, but it will send the correct header.
	•	Token Expiration: If using a Personal Access Token (PAT), ensure that the token has not expired and has sufficient permissions for the requested Git operations.
