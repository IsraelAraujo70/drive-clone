# Google Drive Clone Challenge

## Context

You are going to design and build a cloud storage product inspired by Google Drive.

The goal is to practice software architecture, backend development, frontend product design, deployment, testing, and documentation. This is a portfolio challenge, so the final result should be understandable to another engineer, demonstrable in a browser, and supported by clear evidence that the hardest behaviors work.

Do not treat this challenge as a UI-only clone. The project must include real storage behavior, real metadata handling, real upload and download flows, and a credible architecture for scaling beyond a small demo.

## Product Goal

Build a web application where users can store, organize, download, share, and synchronize files.

The product should support a realistic first version while also explaining how the architecture could evolve toward a large-scale cloud storage system.

## Functional Requirements

### 1. File Upload

Users must be able to upload files to their personal drive.

The product should support:

- Uploading files from the web interface.
- Tracking upload progress.
- Rejecting uploads that exceed user or file-size limits.
- Preserving file metadata such as name, size, type, owner, location, and timestamps.
- Handling upload failures clearly.

### 2. File Download

Users must be able to download files from their drive.

The product should support:

- Downloading files owned by the current user.
- Downloading files shared with the current user.
- Preventing unauthorized file access.
- Returning the correct file bytes, filename, and content type.

### 3. File Synchronization

Users must be able to synchronize local files with the remote environment and vice versa.

The product should define:

- How clients discover remote changes.
- How creates, updates, moves, deletes, and restores are represented.
- How conflicts are handled when local and remote versions diverge.
- What information a future desktop or CLI sync client would need.

The first implementation may expose the synchronization contract before building a full desktop sync client.

### 4. File and Folder Management

Users must be able to organize their drive.

The product should support:

- Creating folders.
- Renaming files and folders.
- Moving files and folders.
- Deleting files and folders.
- Restoring deleted items.
- Preventing invalid folder structures.

### 5. Sharing

Users must be able to share files or folders with other people.

The product should define:

- What sharing modes are supported.
- How access is granted.
- How access is revoked.
- How private files remain private by default.

### 6. Search

Users must be able to search for files in their drive.

The product should support:

- Searching by filename.
- Returning useful metadata with results.
- Keeping search results scoped to files the user can access.
- Excluding deleted files unless the user is explicitly searching deleted items.

### 7. Resumable Uploads

Uploads must be resumable.

If the internet connection drops, the browser refreshes, or the client crashes, the system should be able to continue an upload from the last confirmed point instead of starting over from zero.

The product should define:

- How upload sessions are created.
- How partial progress is tracked.
- How the client discovers what still needs to be uploaded.
- How an upload is finalized.
- How abandoned uploads expire.

## Non-Functional Requirements

Design the system with these long-term scale targets in mind:

- 20 million registered users.
- 50 MB of free storage per user.
- Maximum file size of 50 MB per upload.
- 3 million uploads per day.
- Average upload size of 50 MB.

The system must also be designed for:

- High availability.
- Fault tolerance.
- Reliable upload recovery.
- Secure authorization.
- Clear observability.
- Deployability to a real cloud environment.

## Implementation Constraints

The project must use:

- Rust for the backend.
- TypeScript for the frontend.
- Railway as the preferred deployment target, unless a documented blocker makes another deployment target necessary.

The implementation should include a real database and real object/file storage. Do not store large file bytes directly in application memory or in a normal relational table as the primary storage mechanism.

## Expected Deliverables

The finished project should include:

- A deployed web application.
- Source code for the backend and frontend.
- A README explaining the chosen architecture and tradeoffs.
- Local setup instructions.
- Deployment instructions.
- API documentation.
- Database/schema documentation.
- Tests for core behaviors.
- Evaluation scenarios for resumable uploads, quota enforcement, sync behavior, and access control.
- A short demo script or demo video outline.

## Success Criteria

The challenge is successful when:

- A user can upload, view, organize, download, delete, restore, and share files.
- Upload limits and storage quota are enforced.
- Resumable uploads work after interruption.
- Private files cannot be accessed by unauthorized users.
- Search does not leak files across users.
- Synchronization behavior is specified and testable.
- The app is deployed or has a clear deploy path.
- The README explains the architecture well enough for another engineer to review it.
- The tests and evals provide evidence that the most important behaviors work.
