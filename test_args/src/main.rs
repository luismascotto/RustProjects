mod ff;

fn main() {
    if !ff::ffprobe_command_exists() {
        println!("ffprobe not found in PATH");
        return;
    }
    if !ff::ffmpeg_command_exists() {
        println!("ffmpeg not found in PATH");
        return;
    }
    let args: Vec<String> = std::env::args().collect();

    for (_, file_path) in args.iter().skip(1).enumerate() {
        println!("Checking file: {}", file_path);
        if !ff::valid_input_file(file_path) {
            println!("Invalid input file");
            continue;
        }
        match ff::probe_subtitles(file_path) {
            Ok(ffprobe_output) => {
                if ffprobe_output.streams.is_empty() {
                    println!("No subtitles found");
                    continue;
                }
                println!("Subtitles: {:?}", ffprobe_output);

                let eng_subtitles = ffprobe_output
                    .streams
                    .iter()
                    .filter(|s| s.tags.language == "eng")
                    .collect::<Vec<_>>();
                if eng_subtitles.is_empty() {
                    println!("No eng subtitles found");
                    continue;
                }

                let subtitle_to_extract = eng_subtitles
                    .iter()
                    .find(|s| s.tags.title == "English")
                    .unwrap_or_else(|| {
                        println!("No English subtitle found. Getting first eng");
                        eng_subtitles.first().unwrap()
                    });

                let subtitle_codec = &subtitle_to_extract.codec_name;

                let output_path = file_path.replace(".mkv", &format!(".{}", subtitle_codec));
                match ff::extract_subtitle(file_path, subtitle_to_extract.index, &output_path) {
                    Ok(_) => {
                        println!("Subtitle extracted successfully to {}", output_path);
                    }
                    Err(e) => {
                        println!("extract subtitle error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("ffprobe error: {}", e);
            }
        }

        println!();
    }
}
