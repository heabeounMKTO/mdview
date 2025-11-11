use comrak::{markdown_to_html, Options};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tokio::time;
use warp::Filter;

const OUTPUT_FILE: &str = "output.html";
const STATUS_FILE: &str = "status.json";

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

    let input_path_clone = input_path.clone();
    
    // Spawn file watcher in a separate task
    tokio::spawn(async move {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
        watcher.watch(&input_path_clone, RecursiveMode::NonRecursive).unwrap();

        println!("Watching '{}'... Output: {}", input_path_clone.display(), OUTPUT_FILE);

        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    if let Ok(event) = event {
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                // Debounce: wait for file changes to settle
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                if input_path_clone.exists() {
                                    if let Err(e) = render_markdown(&input_path_clone) {
                                        eprintln!("Render error: {}", e);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    });

    // Start simple HTTP server
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET"]);

    let status_route = warp::path("status.json")
        .and(warp::fs::file(STATUS_FILE))
        .with(cors.clone());

    let html_route = warp::path("output.html")
        .and(warp::fs::file(OUTPUT_FILE))
        .with(cors);

    let routes = status_route.or(html_route);

    println!("Server running at http://localhost:3030/output.html");
    println!("Press Ctrl+C to exit");

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

fn render_markdown(input_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let markdown = fs::read_to_string(input_path)?;
    let options = Options::default();
    let html = markdown_to_html(&markdown, &options);
    let timestamp = chrono::Local::now().timestamp_millis();
    
    // Write status file for browser to check
    let status = format!(r#"{{"timestamp":{}}}"#, timestamp);
    fs::write(STATUS_FILE, status)?;
    
    let output = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');
        
        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        }}
        
        /* Prose styling for markdown content */
        .prose {{
            color: #1f2937;
            max-width: 65ch;
        }}
        
        .prose h1 {{
            font-size: 2.25em;
            font-weight: 700;
            margin-top: 0;
            margin-bottom: 0.8888889em;
            line-height: 1.1111111;
            color: #111827;
        }}
        
        .prose h2 {{
            font-size: 1.5em;
            font-weight: 600;
            margin-top: 2em;
            margin-bottom: 1em;
            line-height: 1.3333333;
            color: #111827;
            padding-bottom: 0.3em;
            border-bottom: 1px solid #e5e7eb;
        }}
        
        .prose h3 {{
            font-size: 1.25em;
            font-weight: 600;
            margin-top: 1.6em;
            margin-bottom: 0.6em;
            line-height: 1.6;
            color: #111827;
        }}
        
        .prose p {{
            margin-top: 1.25em;
            margin-bottom: 1.25em;
            line-height: 1.75;
        }}
        
        .prose a {{
            color: #2563eb;
            text-decoration: underline;
            font-weight: 500;
            text-decoration-color: #93c5fd;
            transition: all 0.2s;
        }}
        
        .prose a:hover {{
            color: #1d4ed8;
            text-decoration-color: #2563eb;
        }}
        
        .prose strong {{
            font-weight: 600;
            color: #111827;
        }}
        
        .prose code {{
            font-family: 'JetBrains Mono', 'Courier New', monospace;
            font-size: 0.875em;
            background: #f3f4f6;
            color: #dc2626;
            padding: 0.2em 0.4em;
            border-radius: 0.375rem;
            font-weight: 500;
        }}
        
        .prose pre {{
            font-family: 'JetBrains Mono', 'Courier New', monospace;
            font-size: 0.875em;
            line-height: 1.7142857;
            margin-top: 1.7142857em;
            margin-bottom: 1.7142857em;
            border-radius: 0.5rem;
            padding: 1rem 1.25rem;
            overflow-x: auto;
            background: #1f2937;
            color: #f9fafb;
            box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
        }}
        
        .prose pre code {{
            background: transparent;
            color: inherit;
            padding: 0;
            font-weight: 400;
            border-radius: 0;
        }}
        
        .prose ul, .prose ol {{
            margin-top: 1.25em;
            margin-bottom: 1.25em;
            padding-left: 1.625em;
        }}
        
        .prose li {{
            margin-top: 0.5em;
            margin-bottom: 0.5em;
            line-height: 1.75;
        }}
        
        .prose blockquote {{
            font-style: italic;
            color: #4b5563;
            border-left: 4px solid #e5e7eb;
            padding-left: 1em;
            margin: 1.6em 0;
            background: #f9fafb;
            padding: 1em 1em 1em 1.5em;
            border-radius: 0 0.375rem 0.375rem 0;
        }}
        
        .prose img {{
            max-width: 100%;
            height: auto;
            border-radius: 0.5rem;
            margin-top: 2em;
            margin-bottom: 2em;
            box-shadow: 0 10px 15px -3px rgb(0 0 0 / 0.1);
        }}
        
        .prose table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 2em;
            margin-bottom: 2em;
        }}
        
        .prose th {{
            background: #f9fafb;
            font-weight: 600;
            text-align: left;
            padding: 0.75em 1em;
            border-bottom: 2px solid #e5e7eb;
        }}
        
        .prose td {{
            padding: 0.75em 1em;
            border-bottom: 1px solid #e5e7eb;
        }}
        
        .prose hr {{
            border: 0;
            border-top: 1px solid #e5e7eb;
            margin: 3em 0;
        }}
        
        /* Dark mode support */
        @media (prefers-color-scheme: dark) {{
            body {{
                background: #0f172a;
            }}
            
            .card {{
                background: #1e293b;
                border-color: #334155;
            }}
            
            .prose {{
                color: #e2e8f0;
            }}
            
            .prose h1, .prose h2, .prose h3, .prose strong {{
                color: #f1f5f9;
            }}
            
            .prose h2 {{
                border-bottom-color: #334155;
            }}
            
            .prose code {{
                background: #334155;
                color: #fca5a5;
            }}
            
            .prose blockquote {{
                color: #cbd5e1;
                border-left-color: #475569;
                background: #1e293b;
            }}
            
            .prose th {{
                background: #1e293b;
                border-bottom-color: #475569;
            }}
            
            .prose td {{
                border-bottom-color: #334155;
            }}
            
            .prose hr {{
                border-top-color: #334155;
            }}
        }}
        
        /* Loading animation */
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}
        
        .loading {{
            animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
        }}
    </style>
    <script>
        // Check for updates by polling status.json
        let lastUpdate = {};
        
        setInterval(async () => {{
            try {{
                const response = await fetch('/status.json', {{ cache: 'no-store' }});
                const data = await response.json();
                
                if (data.timestamp && data.timestamp !== lastUpdate) {{
                    console.log('Update detected, refreshing...');
                    document.body.style.opacity = '0.7';
                    setTimeout(() => window.location.reload(), 200);
                }}
            }} catch (e) {{
                console.log('Check failed:', e);
            }}
        }}, 500);
    </script>
</head>
<body class="bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-800 min-h-screen py-8 px-4 transition-colors">
    <div class="max-w-4xl mx-auto">
        <!-- Header Card -->
        <div class="card bg-white dark:bg-slate-800 rounded-xl shadow-lg border border-slate-200 dark:border-slate-700 p-6 mb-6">
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                    <div class="w-2 h-2 bg-green-500 rounded-full loading"></div>
                    <h1 class="text-sm font-medium text-slate-600 dark:text-slate-400">
                        Live Preview: <span class="text-slate-900 dark:text-slate-100 font-semibold">{}</span>
                    </h1>
                </div>
                <div class="text-xs text-slate-500 dark:text-slate-500">
                    Watching for changes
                </div>
            </div>
        </div>
        
        <!-- Content Card -->
        <article class="card bg-white dark:bg-slate-800 rounded-xl shadow-lg border border-slate-200 dark:border-slate-700 p-8 md:p-12 transition-all">
            <div class="prose prose-slate max-w-none">
{}
            </div>
        </article>
        
        <!-- Footer -->
        <div class="text-center mt-6 text-sm text-slate-500 dark:text-slate-500">
            Generated at {} • Powered by comrak
        </div>
    </div>
</body>
</html>"#,
        input_path.file_name().unwrap().to_str().unwrap_or("Markdown"),
        timestamp,
        input_path.file_name().unwrap().to_str().unwrap_or("document.md"),
        html,
        chrono::Local::now().format("%H:%M:%S")
    );
    
    fs::write(OUTPUT_FILE, output)?;
    println!("✨ Rendered at {}", chrono::Local::now().format("%H:%M:%S"));
    Ok(())
}
