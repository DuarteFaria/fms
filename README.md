# FMS - Fast File Manager with Tags

A fast, responsive file system explorer for macOS built with Rust and GPUI. Combines traditional folder navigation with tag-based organization for efficient file management.

## Features

- **Hybrid Navigation**: Switch between folder view and tag view
- **Fast Search**: Metadata search across file names, paths, and tags using SQLite FTS
- **macOS Tag Support**: Automatically reads and displays macOS file tags
- **Responsive UI**: Built with GPUI for smooth, native performance
- **Read-Only**: Safe file browsing without modification capabilities

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

## Usage

- **Folders Tab**: Traditional folder tree navigation with breadcrumb navigation
- **Tags Tab**: Browse files by tags, with file counts and filtering
- **Search Bar**: Real-time search across all indexed files
- **Click Files**: Reveal files in Finder (read-only)

## Keyboard Shortcuts

- `Cmd+F`: Focus search bar
- `Cmd+1`: Switch to Folders view
- `Cmd+2`: Switch to Tags view

## Architecture

- **SQLite Database**: In-memory database for fast file metadata and tag queries
- **Background Indexing**: Files are indexed asynchronously on startup
- **FTS Search**: Full-text search using SQLite FTS5 for fast queries

## Dependencies

- `gpui`: UI framework
- `rusqlite`: SQLite database with FTS5
- `walkdir`: Directory traversal
- `xattr`: macOS extended attributes (tags)
- `plist`: Parse macOS tag plist data
