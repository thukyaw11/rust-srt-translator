# SRT Translation Tool in RUST

AI-powered subtitle translation to Burmese using Google Gemini. Handles large files with chunked processing and automatic progress saving.

## Setup

```bash
# Build
cargo build --release

# Set API key
export GOOGLE_API_KEY='your-api-key-here'
```

Get API key: https://makersuite.google.com/app/apikey

## Usage

### Translate Subtitles

```bash
# Basic
./target/release/translate_srt input.srt output.srt

# With movie name for better context
./target/release/translate_srt input.srt output.srt 20 "Movie Name"
```

Parameters: `<input.srt> [output.srt] [chunk_size] [movie_name]`

### Remove Citations

```bash
./target/release/clean_srt input.srt output.srt
```

Removes `[cite_start]` and `[cite: N]` tags.

## Examples

```bash
# Translate with context
./target/release/translate_srt english.srt burmese.srt 20 "Idiocracy"

# Smaller chunks for accuracy
./target/release/translate_srt english.srt burmese.srt 10

# Clean citations
./target/release/clean_srt subtitles.srt clean.srt
```

**Note:** Progress auto-saves to `.progress/` folder. Re-run to resume interrupted translations.
