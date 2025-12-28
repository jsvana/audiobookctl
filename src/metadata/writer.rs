use crate::metadata::AudiobookMetadata;
use anyhow::{Context, Result};
use std::path::Path;

/// Write metadata to an m4b file
pub fn write_metadata(path: &Path, metadata: &AudiobookMetadata) -> Result<()> {
    let mut tag = mp4ameta::Tag::read_from_path(path)
        .with_context(|| format!("Failed to read m4b file for writing: {}", path.display()))?;

    // Title
    if let Some(ref title) = metadata.title {
        tag.set_title(title);
    } else {
        tag.remove_title();
    }

    // Author (artist)
    if let Some(ref author) = metadata.author {
        tag.set_artist(author);
    } else {
        tag.remove_artists();
    }

    // Narrator (freeform iTunes atom)
    let narrator_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "NARRATOR");
    if let Some(ref narrator) = metadata.narrator {
        tag.set_data(narrator_ident, mp4ameta::Data::Utf8(narrator.clone()));
    } else {
        tag.remove_data_of(&narrator_ident);
    }

    // Series (TV show name)
    if let Some(ref series) = metadata.series {
        tag.set_tv_show_name(series);
    } else {
        tag.remove_tv_show_name();
    }

    // Series position (TV episode)
    if let Some(pos) = metadata.series_position {
        tag.set_tv_episode(pos);
    } else {
        tag.remove_tv_episode();
    }

    // Year
    if let Some(year) = metadata.year {
        tag.set_year(year.to_string());
    } else {
        tag.remove_year();
    }

    // Description
    if let Some(ref desc) = metadata.description {
        tag.set_description(desc);
    } else {
        tag.remove_descriptions();
    }

    // Genre
    if let Some(ref genre) = metadata.genre {
        tag.set_genre(genre);
    } else {
        tag.remove_genres();
    }

    // ISBN (freeform iTunes atom)
    let isbn_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "ISBN");
    if let Some(ref isbn) = metadata.isbn {
        tag.set_data(isbn_ident, mp4ameta::Data::Utf8(isbn.clone()));
    } else {
        tag.remove_data_of(&isbn_ident);
    }

    // ASIN (freeform iTunes atom)
    let asin_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "ASIN");
    if let Some(ref asin) = metadata.asin {
        tag.set_data(asin_ident, mp4ameta::Data::Utf8(asin.clone()));
    } else {
        tag.remove_data_of(&asin_ident);
    }

    // Note: We don't write duration, chapter_count, or cover_info as they are read-only
    // Publisher is also not written as mp4ameta doesn't support it directly

    tag.write_to_path(path)
        .with_context(|| format!("Failed to write metadata to: {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require actual m4b files
    // These tests just verify the module compiles correctly

    #[test]
    fn test_write_to_nonexistent_fails() {
        let metadata = AudiobookMetadata::default();
        let result = write_metadata(Path::new("/nonexistent/file.m4b"), &metadata);
        assert!(result.is_err());
    }
}
