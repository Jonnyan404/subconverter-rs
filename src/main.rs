use std::sync::Arc;

use actix_web::{test, web, App, HttpServer};
use clap::Parser;
use env_logger::Env;
use log::{error, info};
use std::fs;

use libsubconverter::settings::settings::settings_struct::init_settings;
use libsubconverter::{web_handlers, Settings};

/// A more powerful utility to convert between proxy subscription format
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    /// Listen address (e.g., 127.0.0.1 or 0.0.0.0)
    #[arg(short, long, value_name = "ADDRESS")]
    address: Option<String>,

    /// Listen port
    #[arg(short, long, value_name = "PORT")]
    port: Option<u32>,

    /// Subscription URL to process directly instead of starting the server
    #[arg(long, value_name = "URL")]
    url: Option<String>,

    /// Output file path for subscription conversion (must be used with --url)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Parse command line arguments
    let args = Args::parse();

    // Check if only one of url or output is provided
    if args.url.is_some() != args.output.is_some() {
        eprintln!("Error: --url and -o/--output must be used together");
        std::process::exit(1);
    }

    // Initialize settings with config file path if provided
    init_settings(args.config.as_deref().unwrap_or(""))
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Check if URL is provided for direct processing
    if let Some(url) = args.url {
        let output_file = args
            .output
            .as_ref()
            .expect("Output file must be provided with URL");
        info!(
            "Processing subscription from URL: {} to file: {}",
            url, output_file
        );

        // Create a test app with the same configuration as the web app
        let app = test::init_service(App::new().configure(web_handlers::config)).await;

        // Create a test request with the correct URI
        let req = test::TestRequest::get().uri(&url).to_request();

        // Execute the request
        let resp = test::call_service(&app, req).await;

        // Check if the response is successful
        if resp.status().is_success() {
            // Get response body
            let body = test::read_body(resp).await;

            // Write the response to the output file
            fs::write(output_file, body)?;
            info!("Successfully wrote result to {}", output_file);
        } else {
            error!("API request failed with status: {}", resp.status());
            std::process::exit(1);
        }

        Ok(()) // Exit after processing the URL
    } else {
        // Proceed with starting the web server
        // Ensure we have a valid listen address
        let listen_address = {
            // Get a mutable reference to the current settings
            let mut settings_guard = Settings::current_mut();
            let settings = Arc::make_mut(&mut *settings_guard);

            // Override settings with command line arguments if provided
            if let Some(address) = args.address {
                settings.listen_address = address;
            }
            if let Some(port) = args.port {
                settings.listen_port = port;
            }
            if settings.listen_address.trim().is_empty() {
                error!("Empty listen_address in settings, defaulting to 127.0.0.1");
                format!("127.0.0.1:{}", settings.listen_port)
            } else {
                // Check if the address contains a port
                if settings.listen_address.contains(':') {
                    // Already has a port, use as is
                    settings.listen_address.clone()
                } else {
                    // No port specified, use the one from settings
                    format!("{}:{}", settings.listen_address, settings.listen_port)
                }
            }
        };

        let max_concur_threads = Settings::current().max_concur_threads;

        info!("Subconverter starting on {}", listen_address);

        // Start web server
        HttpServer::new(move || {
            App::new()
                // Register web handlers
                .configure(web_handlers::config)
                // For health check
                .route("/", web::get().to(|| async { "Subconverter is running!" }))
        })
        .bind(listen_address)?
        .workers(max_concur_threads as usize)
        .run()
        .await
    }
}
