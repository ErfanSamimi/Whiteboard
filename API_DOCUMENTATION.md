# Whiteboard API Documentation

## Overview
This API provides functionality for a collaborative whiteboard application with user authentication, project management, and real-time drawing capabilities.

## Base URL
```
http://localhost:3000/api
```

## Authentication
The API uses JWT (JSON Web Token) based authentication. Most endpoints require a valid Bearer token in the Authorization header.

### Token Format
```
Authorization: Bearer <token>
```

## REST Endpoints

### 1. User Authentication

#### Login
- **Endpoint**: `POST /auth/login/`
- **Description**: Authenticates a user and returns a JWT token
- **Request Body**:
```json
{
    "username": "string",
    "password": "string"
}
```
- **Response**:
```json
{
    "access": "string",         // JWT token
    "username": "string",       // Username of authenticated user
    "token_type": "Bearer"
}
```
- **Error Responses**:
  - 401: Wrong credentials
  - 400: Missing credentials
  - 500: Token creation error

### 2. User Management

#### Register User
- **Endpoint**: `POST /users/`
- **Description**: Creates a new user account
- **Request Body**:
```json
{
    "username": "string",
    "email": "string",
    "password": "string",
    "confirm_password": "string"
}
```
- **Response**:
```json
{
    "id": "number",
    "username": "string",
    "email": "string",
    "created_at": "datetime",
    "updated_at": "datetime"
}
```
- **Error Responses**:
  - 400: Username exists
  - 400: Passwords do not match
  - 500: Internal server error

#### List Users
- **Endpoint**: `GET /projects/users/`
- **Description**: Returns list of active users
- **Authentication**: Required
- **Response**:
```json
[
    {
        "id": "number",
        "username": "string",
        "email": "string",
        "created_at": "datetime",
        "updated_at": "datetime"
    }
]
```

### 3. Project Management

#### Create Project
- **Endpoint**: `POST /projects/`
- **Authentication**: Required
- **Description**: Creates a new project
- **Request Body**:
```json
{
    "name": "string"
}
```
- **Response**:
```json
{
    "id": "number",
    "collaborators": [],
    "name": "string",
    "owner": {
        "id": "number",
        "username": "string",
        "email": "string",
        "created_at": "datetime",
        "updated_at": "datetime"
    },
    "created_at": "datetime",
    "updated_at": "datetime"
}
```

#### List Owned Projects
- **Endpoint**: `GET /projects/`
- **Authentication**: Required
- **Description**: Returns list of projects owned by authenticated user
- **Response**:
```json
[
    {
        "id": "number",
        "collaborators": [
            {
                "id": "number",
                "username": "string",
                "email": "string",
                "created_at": "datetime",
                "updated_at": "datetime"
            }
        ],
        "name": "string",
        "owner": {
            "id": "number",
            "username": "string",
            "email": "string",
            "created_at": "datetime",
            "updated_at": "datetime"
        },
        "created_at": "datetime",
        "updated_at": "datetime"
    }
]
```

#### Update Project Collaborators
- **Endpoint**: `POST /projects/{project_id}/update_collaborators/`
- **Authentication**: Required
- **Description**: Updates the list of collaborators for a project
- **URL Parameters**:
  - project_id: Project ID (number)
- **Request Body**:
```json
{
    "collaborator_ids": ["number"]
}
```
- **Response**: Returns updated project details
- **Error Responses**:
  - 403: Not owner of the project
  - 404: Project not found

#### Get Whiteboard Data
- **Endpoint**: `GET /projects/{project_id}/drawing/`
- **Authentication**: Required
- **Description**: Retrieves the current state of the whiteboard
- **URL Parameters**:
  - project_id: Project ID (number)
- **Response**:
```json
{
    "lines": [
        {
            "p": [[x1, y1], [x2, y2]], // Points array
            "c": "string",              // Color
            "w": "number"              // Width
        }
    ],
    "cursorPosition": {
        "x": "number",
        "y": "number",
        "userId": "string",
        "color": "string"
    }
}
```
- **Error Responses**:
  - 403: Not collaborator of the project
  - 404: Project not found

## WebSocket API

### Whiteboard Real-time Connection

#### Connect to Whiteboard
- **Endpoint**: `ws://localhost:3000/ws/whiteboard/{project_id}/`
- **Description**: Establishes WebSocket connection for real-time whiteboard collaboration
- **Authentication**: Required via initial authentication message

#### WebSocket Message Types

1. **Authentication Message**
- **Direction**: Client → Server
- **Format**:
```json
{
    "type": "auth",
    "token": "string"  // JWT token
}
```

2. **Drawing Update**
- **Direction**: Bidirectional
- **Format**:
```json
{
    "type": "drawing_update",
    "data": {
        "lines": [
            {
                "p": [[x1, y1], [x2, y2]],
                "c": "string",
                "w": "number"
            }
        ],
        "cursorPosition": null
    },
    "user": "string"    // Only in client → server messages
}
```

3. **Cursor Update**
- **Direction**: Bidirectional
- **Format**:
```json
{
    "type": "cursor_update",
    "data": {
        "x": "number",
        "y": "number",
        "userId": "string",
        "color": "string"
    },
    "user": "string"    // Only in client → server messages
}
```

4. **Server Messages**
- **Authentication Success**:
```json
{
    "type": "auth_success",
    "message": "string",
    "user_token": "string"
}
```
- **Error Message**:
```json
{
    "type": "error",
    "message": "string"
}
```

### WebSocket Connection Lifecycle

1. Client connects to WebSocket endpoint
2. Client sends authentication message within 5 seconds
3. Server validates token and project access
4. Server responds with auth_success or error
5. After successful authentication:
   - Client can send drawing_update and cursor_update messages
   - Server broadcasts updates to all connected clients
   - Server persists drawing updates in Redis (cache) and MongoDB (permanent storage)
6. Connection is automatically closed if:
   - Client loses project access
   - Authentication fails or times out
   - Client disconnects

### Data Persistence
- Drawing updates are cached in Redis for 1 hour
- Updates are permanently stored in MongoDB
- Redis cache is refreshed on each access
- System uses a write-through caching strategy for drawing updates

## Rate Limiting and Security
- JWT tokens expire after a fixed time period
- WebSocket connections require authentication within 5 seconds
- Project access is verified for each operation
- Owner-only operations are enforced for critical actions
