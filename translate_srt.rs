use std::env;
use std::fs::{self, File};
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
struct SrtEntry {
    index: String,
    timestamp: String,
    text_lines: Vec<String>,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <input_srt_file> [output_srt_file] [chunk_size] [movie_name]", args[0]);
        eprintln!("  chunk_size: number of subtitle entries per chunk (default: 20)");
        eprintln!("  movie_name: name of the movie for better context (optional)");
        eprintln!("\nExample: {} english.srt burmese.srt 20 \"Idiocracy\"", args[0]);
        eprintln!("\nNote: This script requires 'gemini' CLI to be installed.");
        std::process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = if args.len() >= 3 {
        args[2].clone()
    } else {
        format!("{}.translated", input_file)
    };
    let chunk_size: usize = if args.len() >= 4 {
        args[3].parse().unwrap_or(20)
    } else {
        20
    };
    let movie_name: Option<String> = if args.len() >= 5 {
        Some(args[4].clone())
    } else {
        None
    };
    
    if !Path::new(input_file).exists() {
        eprintln!("Error: Input file '{}' does not exist.", input_file);
        std::process::exit(1);
    }
    
    if !is_gemini_available() {
        println!("WARNING: Gemini CLI not found on your system.");
        println!("\nWould you like to install it now? (y/n): ");
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
            println!("\nInstalling Gemini CLI...");
            if let Err(e) = install_gemini_cli() {
                eprintln!("Failed to install Gemini CLI: {}", e);
                eprintln!("\nPlease install manually:");
                eprintln!("  npm install -g @google/gemini-cli");
                eprintln!("  or visit: https://github.com/google-gemini/gemini-cli");
                std::process::exit(1);
            }
            println!("Gemini CLI installed successfully!");
            println!("\nIMPORTANT: You may need to configure your API key:");
            println!("  export GOOGLE_API_KEY='api-key-here'");
            println!("  Get your API key from: https://aistudio.google.com/api-keys\n");
        } else {
            println!("\nInstallation cancelled. Cannot proceed without Gemini CLI.");
            eprintln!("\nTo install manually:");
            eprintln!("  npm install -g @google/gemini-cli");
            std::process::exit(1);
        }
    }
    
    println!("Reading SRT file: {}", input_file);
    if let Some(ref name) = movie_name {
        println!("Movie: {}", name);
    }
    let entries = parse_srt_file(input_file)?;
    println!("Found {} subtitle entries", entries.len());
    
    let chunks: Vec<Vec<SrtEntry>> = entries.chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();
    
    println!("Split into {} chunks of ~{} entries each\n", chunks.len(), chunk_size);
    println!("Starting translation...");
    println!("{}", "=".repeat(60));
    
    let progress_dir = format!("{}.progress", input_file);
    fs::create_dir_all(&progress_dir)?;
    
    let mut translated_entries = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_file = format!("{}/chunk_{:03}.srt", progress_dir, i + 1);
        
        if Path::new(&chunk_file).exists() {
            println!("[{}/{}] Loading cached chunk {}...", 
                     i + 1, chunks.len(), i + 1);
            match parse_srt_file(&chunk_file) {
                Ok(cached) => {
                    translated_entries.extend(cached);
                    continue;
                },
                Err(_) => {
                    println!("  Cache corrupted, retranslating...");
                }
            }
        }
        
        println!("\n[{}/{}] Translating chunk {} ({} entries)...", 
                 i + 1, chunks.len(), i + 1, chunk.len());
        
        match translate_chunk(chunk, i + 1, movie_name.as_deref()) {
            Ok(translated) => {
                println!("  Successfully translated chunk {}", i + 1);
                
                // chunk progress
                if let Err(e) = write_srt_file(&chunk_file, &translated) {
                    eprintln!("  Could not save progress: {}", e);
                }
                
                translated_entries.extend(translated);
            },
            Err(e) => {
                eprintln!("  Error translating chunk {}: {}", i + 1, e);
                eprintln!("  Keeping original English text for this chunk");
                translated_entries.extend(chunk.clone());
            }
        }
        
        if i < chunks.len() - 1 {
            println!("  Waiting 3 seconds before next chunk...\n");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
    // final result
    println!("\n");
    println!("{}", "=".repeat(60));
    println!("writing translated srt to ===> {}", output_file);
    write_srt_file(&output_file, &translated_entries)?;
    
    println!("translation complete! output saved ===> {}", output_file);
    println!("progress saved ===> {}", progress_dir);
    println!("{}", "=".repeat(60));
    
    Ok(())
}

fn is_gemini_available() -> bool {
    Command::new("gemini")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn install_gemini_cli() -> io::Result<()> {
    println!("  Checking for npm...");
    
    // check npm
    let npm_check = Command::new("npm")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    
    if npm_check.is_err() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "npm not found"
        ));
    }
    
    println!("  running: npm install -g @google/gemini-cli");
    
    let output = Command::new("npm")
        .args(&["install", "-g", "@google/gemini-cli"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    
    if !output.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "npm install failed"
        ));
    }
    
    if !is_gemini_available() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "installation completed but gemini command still not found"
        ));
    }
    
    Ok(())
}

fn parse_srt_file(filepath: &str) -> io::Result<Vec<SrtEntry>> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    let mut current_entry: Option<SrtEntry> = None;
    let mut current_text_lines = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        
        if line.trim().is_empty() {
            if let Some(mut entry) = current_entry.take() {
                entry.text_lines = current_text_lines.clone();
                entries.push(entry);
                current_text_lines.clear();
            }
        } else if line.contains("-->") {
            if let Some(ref mut entry) = current_entry {
                entry.timestamp = line;
            }
        } else if line.trim().chars().all(|c| c.is_numeric()) {
            current_entry = Some(SrtEntry {
                index: line,
                timestamp: String::new(),
                text_lines: Vec::new(),
            });
        } else {
            current_text_lines.push(line);
        }
    }
    
    if let Some(mut entry) = current_entry {
        entry.text_lines = current_text_lines;
        entries.push(entry);
    }
    
    Ok(entries)
}

fn translate_chunk(chunk: &[SrtEntry], _chunk_num: usize, movie_name: Option<&str>) -> io::Result<Vec<SrtEntry>> {
    let mut subtitles = Vec::new();
    for (i, entry) in chunk.iter().enumerate() {
        let text = entry.text_lines.join(" ");
        subtitles.push(format!("SUBTITLE_{}: {}", i + 1, text));
    }
    
    let chunk_text = subtitles.join("\n");
    
    let movie_context = if let Some(name) = movie_name {
        format!("This is from the movie \"{}\".\n", name)
    } else {
        String::new()
    };
    
    let prompt = format!(
        "You are a professional translator. {}Translate the following English movie subtitles to Burmese (Myanmar language). \
        Each line starts with SUBTITLE_N: followed by the text to translate.\n\n\
        IMPORTANT RULES:\n\
        1. Translate naturally and meaningfully in Burmese, preserving the context and emotion\n\
        2. Keep the SUBTITLE_N: prefix exactly as is\n\
        3. Maintain the same number of lines as input\n\
        4. For speaker labels like [Man Narrating] or [Reporter], translate them to Burmese too\n\
        5. Preserve meaning over literal word-by-word translation\n\
        6. Use natural Burmese expressions and idioms where appropriate\n\
        7. Keep character names and proper nouns in English (transliterated if needed)\n\
        8. Consider the movie's genre and tone in your translation\n\n\
        Subtitles to translate:\n\n{}",
        movie_context,
        chunk_text
    );
    
    println!("  Calling Gemini CLI for translation...");
    let output = Command::new("gemini")
        .arg("chat")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    
    let mut child = match output {
        Ok(child) => child,
        Err(e) => {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to spawn gemini: {}", e)));
        }
    };
    
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(prompt.as_bytes()) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to write to gemini stdin: {}", e)));
        }
    }
    
    let output = child.wait_with_output()?;
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Gemini CLI failed: {}", error_msg)
        ));
    }
    
    let translated_text = String::from_utf8_lossy(&output.stdout).to_string();
    
    let mut result = Vec::new();
    let mut translations: Vec<String> = Vec::new();
    
    for line in translated_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("SUBTITLE_") && trimmed.contains(":") {
            if let Some(colon_pos) = trimmed.find(':') {
                let translation = trimmed[colon_pos + 1..].trim().to_string();
                if !translation.is_empty() {
                    println!("    [{}] {}", translations.len() + 1, translation);
                    translations.push(translation);
                }
            }
        }
    }
    
    println!("  Translated {}/{} entries", translations.len(), chunk.len());
    
    for (i, entry) in chunk.iter().enumerate() {
        let text_lines = if i < translations.len() {
            vec![translations[i].clone()]
        } else {
            eprintln!("  Warning: Missing translation for entry {}, keeping original", i + 1);
            entry.text_lines.clone()
        };
        
        result.push(SrtEntry {
            index: entry.index.clone(),
            timestamp: entry.timestamp.clone(),
            text_lines,
        });
    }
    
    Ok(result)
}

fn write_srt_file(filepath: &str, entries: &[SrtEntry]) -> io::Result<()> {
    let mut file = File::create(filepath)?;
    
    for entry in entries {
        writeln!(file, "{}", entry.index)?;
        writeln!(file, "{}", entry.timestamp)?;
        for line in &entry.text_lines {
            writeln!(file, "{}", line)?;
        }
        writeln!(file)?;
    }
    
    Ok(())
}
