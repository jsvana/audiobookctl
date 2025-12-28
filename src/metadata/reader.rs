use crate::metadata::AudiobookMetadata;
use anyhow::{Context, Result};
use std::path::Path;

/// Read metadata from an m4b file
pub fn read_metadata(path: &Path) -> Result<AudiobookMetadata> {
    let mut tag = mp4ameta::Tag::read_from_path(path)
        .with_context(|| format!("Failed to read m4b file: {}", path.display()))?;

    Ok(AudiobookMetadata {
        title: tag.title().map(String::from),
        author: tag.artist().map(String::from),
        narrator: tag
            .take_strings_of(&mp4ameta::FreeformIdent::new(
                "com.apple.iTunes",
                "NARRATOR",
            ))
            .next(),
        series: tag.tv_show_name().map(String::from),
        series_position: tag.tv_episode(),
        year: tag.year().and_then(|s| s.parse().ok()),
        description: tag.description().map(String::from),
        publisher: None, // mp4ameta doesn't expose publisher directly
        genre: tag.genre().map(String::from),
        duration_seconds: tag.duration().map(|d| d.as_secs()),
        chapter_count: None, // Would need separate chapter parsing
        isbn: tag
            .take_strings_of(&mp4ameta::FreeformIdent::new("com.apple.iTunes", "ISBN"))
            .next(),
        asin: tag
            .take_strings_of(&mp4ameta::FreeformIdent::new("com.apple.iTunes", "ASIN"))
            .next(),
        cover_info: tag.artwork().map(|art| {
            let fmt = match art.fmt {
                mp4ameta::ImgFmt::Jpeg => "JPEG",
                mp4ameta::ImgFmt::Png => "PNG",
                mp4ameta::ImgFmt::Bmp => "BMP",
            };
            format!("embedded ({} bytes, {})", art.data.len(), fmt)
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_read_nonexistent_file_returns_error() {
        let path = PathBuf::from("/nonexistent/file.m4b");
        let result = read_metadata(&path);
        assert!(result.is_err());
    }
}
