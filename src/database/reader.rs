use std::io::{Cursor, Read};

use varuint::ReadVarint;
use ytpapi::Video;

use crate::consts::CACHE_DIR;

/// Reads the database
pub fn read() -> Option<Vec<Video>> {
    let mut buffer = Cursor::new(std::fs::read(CACHE_DIR.join("db.bin")).ok()?);
    let mut videos = Vec::new();
    while buffer.get_mut().len() > buffer.position() as usize {
        videos.push(read_video(&mut buffer)?);
    }
    Some(videos)
}

/// Reads a video from the cursor
fn read_video(buffer: &mut Cursor<Vec<u8>>) -> Option<Video> {
    Some(Video {
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
