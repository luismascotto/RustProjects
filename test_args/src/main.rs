use std::fs;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::io::ErrorKind;

fn main() {
    if !command_exists("ffprobe") {
        println!("ffprobe not found in PATH");
        return;
    }
    let args: Vec<String> = std::env::args().collect();

    //println!("Received {} argument(s):", args.len());
    for (_i, arg) in args.iter().skip(1).enumerate() {
        println!("Checking file: {}", arg);
        match fs::metadata(arg) {
            Ok(_metad) => {
                match ffprobe_subtitles(arg) {
                    Ok(ffprobe_output) => {
                        println!("Subtitles: {:?}", ffprobe_output);
                    }
                    Err(e) => {
                        println!("FFProbe Error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("File Error: {}", e);
            }
        }
        println!();
    }

    // print!("Press Enter to exit...");
    // io::stdout().flush().unwrap();
    // let mut _input = String::new();
    // io::stdin().read_line(&mut _input).unwrap();
}


#[derive(Debug, Serialize, Deserialize)]
struct FfprobeOutput {
    streams: Vec<Subtitle>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Subtitle {
    index: u32,
    codec_name: String,
    codec_tag_string: String,
    codec_long_name: String,
    tags: Tag,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tag {
    language: String,
    title: String,
}

fn ffprobe_subtitles(file_path: &str) -> Result<FfprobeOutput, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("s")
        .arg("-show_entries")
        .arg("stream=index,codec_name,codec_tag_string,codec_long_name:stream_tags=language,title")
        .arg("-of")
        .arg("json")
        .arg(file_path)
        .output()?;
    

    let stdout = String::from_utf8(output.stdout)?;
    let ffprobe_output: FfprobeOutput = serde_json::from_str(&stdout)?;
    
    Ok(ffprobe_output)
}

fn command_exists(cmd: &str) -> bool {
    match Command::new(cmd).spawn() {
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