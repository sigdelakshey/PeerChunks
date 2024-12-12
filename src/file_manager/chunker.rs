use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub file_id: Uuid,        // Unique identifier for the file
    pub chunk_index: usize,   // Index of the chunk within the file
    pub chunk_size: usize,    // Size of the chunk in bytes
    pub total_chunks: usize,  // Total number of chunks for the file
}

impl ChunkMetadata {
    pub fn new(file_id: Uuid, chunk_index: usize, chunk_size: usize, total_chunks: usize) -> Self {
        Self {
            file_id,
            chunk_index,
            chunk_size,
            total_chunks,
        }
    }
}

pub fn split_file_into_chunks<P: AsRef<Path>>(
    file_path: P,
    chunk_size: usize,
) -> io::Result<(Uuid, Vec<(ChunkMetadata, Vec<u8>)>)> {
    let mut file = File::open(&file_path)?;
    let file_id = Uuid::new_v4();
    let mut chunks = Vec::new();
    let mut buffer = vec![0u8; chunk_size];
    let mut chunk_index = 0;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let data = buffer[..bytes_read].to_vec();
        let metadata = ChunkMetadata::new(
            file_id,
            chunk_index,
            bytes_read,
            (bytes_read as f64 / chunk_size as f64).ceil() as usize,
        );

        chunks.push((metadata, data));
        chunk_index += 1;
    }

    Ok((file_id, chunks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_split_file_into_chunks() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, ShareSphere! This is a test file for chunking.";
        temp_file.write_all(content).unwrap();

        let chunk_size = 10;

        let (file_id, chunks) = split_file_into_chunks(temp_file.path(), chunk_size).unwrap();

        assert_eq!(chunks.len(), 5);

        for (i, (metadata, data)) in chunks.iter().enumerate() {
            assert_eq!(metadata.file_id, file_id);
            assert_eq!(metadata.chunk_index, i);
            if i < 3 {
                assert_eq!(metadata.chunk_size, chunk_size);
                assert_eq!(data.len(), chunk_size);
            } else {
                assert_eq!(metadata.chunk_size, 7);
                assert_eq!(data.len(), 7);
            }
        }
    }
}
