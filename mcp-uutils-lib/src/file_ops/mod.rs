use crate::guardrails::validate_path_safety;
use std::path::Path;
use serde_json::{json, Value};
use std::time::SystemTime;

/// Safe wrapper around file and directory deletions.
pub fn delete_file_or_directory(path: &str) -> Result<(), String> {
    validate_path_safety(path)?;

    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(format!("Target does not exist: {}", path));
    }

    if path_buf.is_dir() {
        std::fs::remove_dir_all(path_buf).map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        std::fs::remove_file(path_buf).map_err(|e| format!("Failed to delete file: {}", e))
    }
}

/// Lists files, folders, and links in a directory.
pub fn list_directory_contents(path: Option<String>) -> Result<Value, String> {
    let target_path_str = path.unwrap_or_else(|| ".".to_string());
    validate_path_safety(&target_path_str)?;

    let target_path = Path::new(&target_path_str);
    if !target_path.is_dir() {
        return Err(format!("Path is not a directory: {}", target_path_str));
    }

    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut links = Vec::new();

    let entries = std::fs::read_dir(target_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let file_type = entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?;
        let name = entry.file_name().to_string_lossy().into_owned();

        if file_type.is_symlink() {
            links.push(name);
        } else if file_type.is_dir() {
            directories.push(name);
        } else {
            files.push(name);
        }
    }

    Ok(json!({
        "files": files,
        "directories": directories,
        "links": links
    }))
}

/// Retrieves realpath, size, permissions, and timestamps.
pub fn get_file_metadata(path: &str) -> Result<Value, String> {
    validate_path_safety(path)?;

    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let metadata = std::fs::metadata(path_buf)
        .map_err(|e| format!("Failed to retrieve metadata: {}", e))?;

    let absolute_path = std::fs::canonicalize(path_buf)
        .map_err(|e| format!("Failed to resolve absolute path: {}", e))?
        .to_string_lossy()
        .into_owned();

    let permissions = if metadata.permissions().readonly() {
        "readonly"
    } else {
        "readwrite"
    };

    let modified = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let accessed = metadata.accessed()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(json!({
        "absolute_path": absolute_path,
        "size_bytes": metadata.len(),
        "is_dir": metadata.is_dir(),
        "permissions": permissions,
        "modified_at": modified,
        "accessed_at": accessed
    }))
}

/// Copy a file or directory recursively.
pub fn copy_file_or_directory(source: &str, destination: &str) -> Result<(), String> {
    validate_path_safety(source)?;
    validate_path_safety(destination)?;

    let src = Path::new(source);
    let dest = Path::new(destination);

    if !src.exists() {
        return Err(format!("Source does not exist: {}", source));
    }

    if src.is_dir() {
        copy_dir_all(src, dest)
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
        std::fs::copy(src, dest)
            .map(|_| ())
            .map_err(|e| format!("Failed to copy file: {}", e))
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), String> {
    std::fs::create_dir_all(&dst).map_err(|e| format!("Failed to create directory: {}", e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let file_type = entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?;
        if file_type.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }
    Ok(())
}

/// Move a file or directory.
pub fn move_file_or_directory(source: &str, destination: &str) -> Result<(), String> {
    validate_path_safety(source)?;
    validate_path_safety(destination)?;

    let src = Path::new(source);
    let dest = Path::new(destination);

    if !src.exists() {
        return Err(format!("Source does not exist: {}", source));
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    std::fs::rename(src, dest).map_err(|e| format!("Failed to move target: {}", e))
}

/// Creates a directory, including all intermediate parents (equivalent to mkdir -p).
pub fn create_directory(path: &str) -> Result<(), String> {
    validate_path_safety(path)?;
    std::fs::create_dir_all(Path::new(path)).map_err(|e| format!("Failed to create directory: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;

    #[test]
    fn test_delete_file_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_delete.txt");
        File::create(&file_path).unwrap();
        let path_str = file_path.to_str().unwrap();
        assert!(delete_file_or_directory(path_str).is_ok());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_list_directory_contents() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("file.txt")).unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let list = list_directory_contents(Some(dir.path().to_str().unwrap().to_string())).unwrap();
        let files = list["files"].as_array().unwrap();
        let dirs = list["directories"].as_array().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(dirs.len(), 1);
        assert_eq!(files[0], "file.txt");
        assert_eq!(dirs[0], "subdir");
    }

    #[test]
    fn test_get_file_metadata() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        let meta = get_file_metadata(file_path.to_str().unwrap()).unwrap();
        assert_eq!(meta["size_bytes"], 0);
        assert_eq!(meta["is_dir"], false);
        assert_eq!(meta["permissions"], "readwrite");
    }

    #[test]
    fn test_copy_move_create_dir() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src_dir");
        let dest_dir = dir.path().join("dest_dir");

        assert!(create_directory(src_dir.to_str().unwrap()).is_ok());
        let test_file = src_dir.join("test.txt");
        std::fs::write(&test_file, b"data").unwrap();

        assert!(copy_file_or_directory(src_dir.to_str().unwrap(), dest_dir.to_str().unwrap()).is_ok());
        assert!(dest_dir.join("test.txt").exists());

        let moved_dir = dir.path().join("moved_dir");
        assert!(move_file_or_directory(dest_dir.to_str().unwrap(), moved_dir.to_str().unwrap()).is_ok());
        assert!(!dest_dir.exists());
        assert!(moved_dir.join("test.txt").exists());
    }
}
