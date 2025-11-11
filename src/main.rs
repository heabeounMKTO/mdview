use comrak::{markdown_to_html, Options};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::time;

const OUTPUT_FILE: &str = "output.html";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <markdown_file.md>", args[0]);
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    if !input_path.exists() {
        eprintln!("Error: File '{}' not found", input_path.display());
        std::process::exit(1);
    }

    // Initial render
    render_markdown(&input_path)?;

    // Setup file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(&input_path, RecursiveMode::NonRecursive)?;

    println!("Watching '{}'... Output: {}", input_path.display(), OUTPUT_FILE);
    println!("Press Ctrl+C to exit");

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if let Ok(event) = event {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                            // Debounce: wait for file changes to settle
                            time::sleep(Duration::from_millis(100)).await;
                            if input_path.exists() {
                                if let Err(e) = render_markdown(&input_path) {
                                    eprintln!("Render error: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(_) => continue, // Timeout, check for signals
        }
    }
}

fn render_markdown(input_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let markdown = fs::read_to_string(input_path)?;
    let options = Options::default();
    let html = markdown_to_html(&markdown, &options);
    
    let output = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>{}</title>
    <style>
        body {{ max-width: 800px; margin: 40px auto; padding: 0 20px; 
               font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto; }}
        code {{ background: #f0f0f0; padding: 2px 4px; border-radius: 3px; }}
        pre {{ background: #f8f8f8; padding: 10px; border-radius: 5px; overflow-x: auto; }}
        img {{ max-width: 100%; }}
    </style>
    <script>
        // Auto-reload every 1 second
        setTimeout(() => {{
            window.location.reload();
        }}, 1000);
    </script>
</head>
<body>
{}
</body>
</html>"#,
        input_path.file_name().unwrap().to_str().unwrap_or("Markdown"),
        html
    );
    
    fs::write(OUTPUT_FILE, output)?;
    println!("Rendered at {}", chrono::Local::now().format("%H:%M:%S"));
    Ok(())
}
