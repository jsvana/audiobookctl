use crate::metadata::{read_metadata, AudiobookMetadata};
use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

pub fn run(path: &Path, json: bool, field: Option<&str>, quiet: bool) -> Result<()> {
    let metadata = read_metadata(path)?;

    if let Some(field_name) = field {
        print_single_field(&metadata, field_name)?;
    } else if json {
        print_json(&metadata)?;
    } else {
        print_pretty(&metadata, path, quiet)?;
    }

    Ok(())
}

fn print_single_field(metadata: &AudiobookMetadata, field: &str) -> Result<()> {
    let value = match field {
        "title" => metadata.title.as_deref(),
        "author" => metadata.author.as_deref(),
        "narrator" => metadata.narrator.as_deref(),
        "series" => metadata.series.as_deref(),
        "description" => metadata.description.as_deref(),
        "publisher" => metadata.publisher.as_deref(),
        "genre" => metadata.genre.as_deref(),
        "isbn" => metadata.isbn.as_deref(),
        "asin" => metadata.asin.as_deref(),
        "cover_info" => metadata.cover_info.as_deref(),
        "year" => {
            if let Some(y) = metadata.year {
                println!("{}", y);
            }
            return Ok(());
        }
        "series_position" => {
            if let Some(p) = metadata.series_position {
                println!("{}", p);
            }
            return Ok(());
        }
        "duration_seconds" => {
            if let Some(d) = metadata.duration_seconds {
                println!("{}", d);
            }
            return Ok(());
        }
        "chapter_count" => {
            if let Some(c) = metadata.chapter_count {
                println!("{}", c);
            }
            return Ok(());
        }
        _ => bail!("Unknown field: {}. Valid fields: title, author, narrator, series, series_position, year, description, publisher, genre, isbn, asin, duration_seconds, chapter_count, cover_info", field),
    };

    if let Some(v) = value {
        println!("{}", v);
    }
    Ok(())
}

fn print_json(metadata: &AudiobookMetadata) -> Result<()> {
    let json = serde_json::to_string_pretty(metadata)?;
    println!("{}", json);
    Ok(())
}

fn print_pretty(metadata: &AudiobookMetadata, path: &Path, quiet: bool) -> Result<()> {
    if !quiet {
        println!("{}", path.display().to_string().bold());
        println!("{}", "─".repeat(40));
    }

    print_field("Title", metadata.title.as_deref());
    print_field("Author", metadata.author.as_deref());
    print_field("Narrator", metadata.narrator.as_deref());

    if metadata.series.is_some() || metadata.series_position.is_some() {
        let series_str = match (&metadata.series, metadata.series_position) {
            (Some(s), Some(p)) => format!("{} #{}", s, p),
            (Some(s), None) => s.clone(),
            (None, Some(p)) => format!("#{}", p),
            (None, None) => unreachable!(),
        };
        print_field("Series", Some(&series_str));
    }

    if let Some(year) = metadata.year {
        print_field("Year", Some(&year.to_string()));
    }

    print_field("Genre", metadata.genre.as_deref());
    print_field("Publisher", metadata.publisher.as_deref());

    if let Some(duration) = metadata.duration_seconds {
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;
        print_field("Duration", Some(&format!("{:02}:{:02}:{:02}", hours, minutes, seconds)));
    }

    if let Some(chapters) = metadata.chapter_count {
        print_field("Chapters", Some(&chapters.to_string()));
    }

    print_field("ISBN", metadata.isbn.as_deref());
    print_field("ASIN", metadata.asin.as_deref());
    print_field("Cover", metadata.cover_info.as_deref());

    if let Some(desc) = &metadata.description {
        println!();
        println!("{}", "Description:".cyan());
        for line in textwrap_simple(desc, 80) {
            println!("  {}", line);
        }
    }

    Ok(())
}

fn print_field(label: &str, value: Option<&str>) {
    if let Some(v) = value {
        println!("{:>12}: {}", label.cyan(), v);
    }
}

/// Simple text wrapping without external dependency
fn textwrap_simple(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}
