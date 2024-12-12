use crate::file_manager::chunker::ChunkMetadata;
use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug)]
pub enum StorageError {
    IoError(io::Error),
    InvalidPath(String),
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        StorageError::IoError(error)
    }
}

pub fn initialize_storage<P: AsRef<Path>>(
    storage_root: P,
    file_id: Uuid,
) -> Result<PathBuf, StorageError> {
    let file_dir = storage_root.as_ref().join(file_id.to_string());
    fs::create_dir_all(&file_dir)?;
    Ok(file_dir)
}

pub fn save_chunk<P: AsRef<Path>>(
    storage_dir: P,
    metadata: &ChunkMetadata,
    data: &[u8],
) -> Result<(), StorageError> {
    let chunk_filename = format!("chunk_{}.bin", metadata.chunk_index);
    let chunk_path = storage_dir.as_ref().join(chunk_filename);
    let mut file = File::create(chunk_path)?;
    file.write_all(data)?;
    Ok(())
}

pub fn get_chunk<P: AsRef<Path>>(
    storage_dir: P,
    chunk_index: usize,
) -> Result<Vec<u8>, StorageError> {
    let chunk_filename = format!("chunk_{}.bin", chunk_index);
    let chunk_path = storage_dir.as_ref().join(chunk_filename);
    let mut file = File::open(chunk_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok(data)
}

pub fn list_chunks<P: AsRef<Path>>(
    storage_dir: P,
) -> Result<Vec<usize>, StorageError> {
    let mut chunk_indices = Vec::new();
    for entry in fs::read_dir(storage_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("chunk_") && filename.ends_with(".bin") {
                    if let Some(index_str) = filename.strip_prefix("chunk_").and_then(|s| s.strip_suffix(".bin")) {
                        if let Ok(index) = index_str.parse::<usize>() {
                            chunk_indices.push(index);
                        }
                    }
                }
            }
        }
    }
    chunk_indices.sort_unstable();
    Ok(chunk_indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_manager::chunker::{ChunkMetadata, split_file_into_chunks};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_get_chunk() {
        let temp_dir = TempDir::new().unwrap();
        let storage_root = temp_dir.path();
        let file_id = Uuid::new_v4();

        let storage_dir = initialize_storage(storage_root, file_id).unwrap();

        let metadata = ChunkMetadata::new(file_id, 0, 5, 1);
        let data = b"Hello";

        save_chunk(&storage_dir, &metadata, data).unwrap();

        let retrieved_data = get_chunk(&storage_dir, 0).unwrap();
        assert_eq!(retrieved_data, data);
    }

    #[test]
    fn test_list_chunks() {
        let temp_dir = TempDir::new().unwrap();
        let storage_root = temp_dir.path();
        let file_id = Uuid::new_v4();

        let storage_dir = initialize_storage(storage_root, file_id).unwrap();

        for i in 0..5 {
            let metadata = ChunkMetadata::new(file_id, i, 5, 5);
            let data = format!("Chunk{}", i).into_bytes();
            save_chunk(&storage_dir, &metadata, &data).unwrap();
        }

        let chunks = list_chunks(&storage_dir).unwrap();
        assert_eq!(chunks, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_split_and_save_chunks() {
        let temp_dir = TempDir::new().unwrap();
        let storage_root = temp_dir.path();

        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"HelloShareSphereFileChunkingTest!";
        temp_file.write_all(content).unwrap();

        let chunk_size = 5;
        let (file_id, chunks) = split_file_into_chunks(temp_file.path(), chunk_size).unwrap();
        let storage_dir = initialize_storage(storage_root, file_id).unwrap();

        for (metadata, data) in &chunks {
            save_chunk(&storage_dir, metadata, data).unwrap();
        }

        let chunk_indices = list_chunks(&storage_dir).unwrap();
        assert_eq!(chunk_indices.len(), chunks.len());

        for (metadata, data) in &chunks {
            let retrieved_data = get_chunk(&storage_dir, metadata.chunk_index).unwrap();
            assert_eq!(&retrieved_data, data);
        }
    }
}
