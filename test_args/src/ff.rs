use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FfprobeOutput {
    pub(crate)  streams: Vec<Subtitle>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Subtitle {
    pub(crate) index: u32,
    pub(crate) codec_name: String,
    pub(crate) codec_tag_string: String,
    pub(crate) codec_long_name: String,
    pub(crate) tags: Tag,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Tag {
    pub(crate) language: String,
    pub(crate) title: String,
}

const FFPROBE_EXE: &'static str = "ffprobe";
const FFPROBE_ARGS: &'static [&'static str] = &[
    "-v",
    "error",
    "-select_streams",
    "s",
    "-show_entries",
    "stream=index,codec_name,codec_tag_string,codec_long_name:stream_tags=language,title",
    "-of",
    "json",
];

pub(crate) fn probe_subtitles(
    file_path: &str,
) -> Result<FfprobeOutput, Box<dyn std::error::Error>> {
    let output = Command::new(FFPROBE_EXE)
        .args(FFPROBE_ARGS)
        .arg(file_path)
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let ffprobe_output: FfprobeOutput = serde_json::from_str(&stdout)?;

    Ok(ffprobe_output)
}

pub(crate) fn ffprobe_command_exists() -> bool {
    command_exists(FFPROBE_EXE)
}


const FFMPEG_EXE: &'static str = "ffmpeg";
const FFMPEG_ARGS: &'static [&'static str] = &[
    "-v",
    "error",
    "-y",
];

pub(crate) fn ffmpeg_command_exists() -> bool {
    command_exists(FFMPEG_EXE)
}



pub(crate) fn extract_subtitle(
    file_path: &str,
    subtitle_index: u32,
    output_path: &str
) -> Result<(), Box<dyn std::error::Error>> {
    //println!("Extracting subtitle {} from {} to {}", subtitle_index, file_path, output_path);
    let output = Command::new(FFMPEG_EXE)
        .args(FFMPEG_ARGS)
        .arg("-i")
        .arg(file_path)
        .arg("-map")
        .arg(format!("0:{}", subtitle_index))
        .arg("-c:s")
        .arg("copy")
        .arg(output_path)
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    if !stderr.is_empty() {
        return Err(Box::new(std::io::Error::new(ErrorKind::Other, stderr)));
    }
    Ok(())
}


pub(crate) fn valid_input_file(file_path: &str) -> bool {
    match std::fs::metadata(file_path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                eprintln!("File is not a MKV file");
                return false
            }
            if get_extension(file_path).unwrap_or_else(|| "") != "mkv" {
                eprintln!("File is not a MKV file");
                return false
            }
            true
        }
        Err(e) => {
            eprintln!("File Error: {}", e);
            false
        }
    }
}

fn get_extension(path_str: &str) -> Option<&str> {
    Path::new(path_str)
        .extension()          // Returns Option<&OsStr>
        .and_then(OsStr::to_str) // Converts safe UTF-8 to Option<&str>
}

fn command_exists(program: &str) -> bool {
    match Command::new(program).spawn() {
        Ok(mut child) => {
            // The command exists and started. Kill it immediately to avoid side effects.
            let _ = child.kill();
            true
        }
        Err(e) => {
            // ErrorKind::NotFound explicitly means the executable is missing
            e.kind() != ErrorKind::NotFound
        }
    }
}
