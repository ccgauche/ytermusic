use std::{
    io::{Cursor, Read},
    sync::RwLockReadGuard,
};

use varuint::ReadVarint;
use ytpapi2::YoutubeMusicVideoRef;

use crate::YTLocalDatabase;

impl YTLocalDatabase {
    /// Loads `db.bin` into memory. Returns false if the file is missing or corrupted.
    pub fn load(&self) -> bool {
        match self.read_file() {
            Some(videos) => {
                *self.references.write().unwrap() = videos;
                true
            }
            None => false,
        }
    }

    /// Returns the in-memory list of downloaded videos.
    pub fn read(&self) -> RwLockReadGuard<'_, Vec<YoutubeMusicVideoRef>> {
        self.references.read().unwrap()
    }

    fn read_file(&self) -> Option<Vec<YoutubeMusicVideoRef>> {
        let mut buffer = Cursor::new(std::fs::read(self.cache_dir.join("db.bin")).ok()?);
        let mut videos = Vec::new();
        while buffer.get_mut().len() > buffer.position() as usize {
            videos.push(read_video(&mut buffer)?);
        }
        Some(videos)
    }
}

/// Reads a video from the cursor
fn read_video(buffer: &mut Cursor<Vec<u8>>) -> Option<YoutubeMusicVideoRef> {
    Some(YoutubeMusicVideoRef {
        title: read_str(buffer)?,
        author: read_str(buffer)?,
        album: read_str(buffer)?,
        video_id: read_str(buffer)?,
        duration: read_str(buffer)?,
    })
}

/// Reads a string from the cursor
fn read_str(cursor: &mut Cursor<Vec<u8>>) -> Option<String> {
    let mut buf = vec![0u8; read_u32(cursor)? as usize];
    cursor.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/// Reads a u32 from the cursor
fn read_u32(cursor: &mut Cursor<Vec<u8>>) -> Option<u32> {
    ReadVarint::<u32>::read_varint(cursor).ok()
}
