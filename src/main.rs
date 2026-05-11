mod cli;
mod config;
mod export_import;
mod history;
mod http_client;
mod models;
mod storage;
mod tui;
mod utils;

use anyhow::Result;
use clap::Parser;
use models::{Auth, BodyType, Collection, Environment, HttpMethod, Request};
use storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        Some(cmd) => run_cli(cmd).await,
        None => {
            tui::run().await?;
            Ok(())
        }
    }
}

async fn run_cli(cmd: cli::Commands) -> Result<()> {
    let storage = Storage::new()?;
    let mut data = storage.load()?;

    match cmd {
        cli::Commands::Send {
            method,
            url,
            header,
            body,
            content_type,
        } => {
            let method = parse_method(&method)?;
            let body_type = content_type
                .map(|ct| parse_content_type(&ct))
                .unwrap_or(BodyType::None);

            let req = Request {
                id: uuid::Uuid::new_v4().to_string(),
                name: "CLI Request".to_string(),
                method,
                url,
                headers: http_client::parse_headers(&header),
                params: vec![],
                body: body.unwrap_or_default(),
                body_type,
                auth: Auth::None,
            };

            println!("Sending {} {}", req.method, req.url);
            let resp = http_client::send_request(&req, None).await?;
            println!(
                "Status: {} {} | Time: {}ms | Size: {} bytes",
                resp.status,
                resp.status_text,
                resp.duration_ms,
                resp.body.len()
            );
            println!();
            println!("{}", utils::format_json(&resp.body));
        }
        cli::Commands::Collection { action } => match action {
            cli::CollectionAction::List => {
                if data.collections.is_empty() {
                    println!("No collections found.");
                } else {
                    for c in &data.collections {
                        println!("{} - {} ({} requests)", c.id, c.name, c.requests.len());
                    }
                }
            }
            cli::CollectionAction::Add { name } => {
                let col = Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    requests: vec![],
                    created_at: chrono::Local::now(),
                };
                data.collections.push(col);
                storage.save(&data)?;
                println!("Collection created.");
            }
            cli::CollectionAction::Remove { name } => {
                let before = data.collections.len();
                data.collections.retain(|c| c.name != name && c.id != name);
                if data.collections.len() < before {
                    storage.save(&data)?;
                    println!("Collection removed.");
                } else {
                    println!("Collection not found.");
                }
            }
            cli::CollectionAction::AddReq {
                collection,
                method,
                url,
                name,
            } => {
                let method = parse_method(&method)?;
                if let Some(col) = data
                    .collections
                    .iter_mut()
                    .find(|c| c.name == collection || c.id == collection)
                {
                    let req = Request {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.unwrap_or_else(|| format!("{} {}", method, url)),
                        method,
                        url,
                        ..Default::default()
                    };
                    col.requests.push(req);
                    storage.save(&data)?;
                    println!("Request added to collection.");
                } else {
                    println!("Collection not found.");
                }
            }
        },
        cli::Commands::Env { action } => match action {
            cli::EnvAction::List => {
                if data.environments.is_empty() {
                    println!("No environments found.");
                } else {
                    for e in &data.environments {
                        let active = data.active_env_id.as_ref() == Some(&e.id);
                        println!(
                            "{} {} - {} ({} vars)",
                            if active { "*" } else { " " },
                            e.id,
                            e.name,
                            e.variables.len()
                        );
                    }
                }
            }
            cli::EnvAction::Add { name } => {
                let env = Environment {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    variables: std::collections::HashMap::new(),
                };
                data.environments.push(env);
                storage.save(&data)?;
                println!("Environment created.");
            }
            cli::EnvAction::Set { env, key, value } => {
                if let Some(e) = data
                    .environments
                    .iter_mut()
                    .find(|e| e.name == env || e.id == env)
                {
                    e.variables.insert(key, value);
                    storage.save(&data)?;
                    println!("Variable set.");
                } else {
                    println!("Environment not found.");
                }
            }
            cli::EnvAction::Remove { name } => {
                let before = data.environments.len();
                data.environments.retain(|e| e.name != name && e.id != name);
                if data.environments.len() < before {
                    storage.save(&data)?;
                    println!("Environment removed.");
                } else {
                    println!("Environment not found.");
                }
            }
        },
        cli::Commands::Run { name } => {
            if let Some(col) = data
                .collections
                .iter()
                .find(|c| c.name == name || c.id == name)
            {
                println!("Running collection: {} ({} requests)", col.name, col.requests.len());
                let mut passed = 0;
                let mut failed = 0;
                for req in &col.requests {
                    print!("  {} {} ... ", req.method, req.url);
                    match http_client::send_request(req, None).await {
                        Ok(resp) => {
                            if resp.status < 400 {
                                println!("OK ({})", resp.status);
                                passed += 1;
                            } else {
                                println!("FAIL ({})", resp.status);
                                failed += 1;
                            }
                        }
                        Err(e) => {
                            println!("ERROR: {}", e);
                            failed += 1;
                        }
                    }
                }
                println!("\nResults: {} passed, {} failed", passed, failed);
            } else {
                println!("Collection not found.");
            }
        }
        cli::Commands::Export { name, format, output } => {
            if let Some(col) = data
                .collections
                .iter()
                .find(|c| c.name == name || c.id == name)
            {
                let content = match format.as_str() {
                    "postman" => export_import::export_collection_postman(col)?,
                    _ => export_import::export_collection_json(col)?,
                };
                if let Some(path) = output {
                    std::fs::write(&path, &content)?;
                    println!("Exported to {}", path);
                } else {
                    println!("{}", content);
                }
            } else {
                println!("Collection not found.");
            }
        }
        cli::Commands::Import { file, format } => {
            let content = std::fs::read_to_string(&file)?;
            let fmt = if format == "auto" {
                export_import::guess_format(&content)
            } else {
                &format
            };
            let col = match fmt {
                "postman" => export_import::import_postman(&content)?,
                _ => export_import::import_json(&content)?,
            };
            data.collections.push(col);
            storage.save(&data)?;
            println!("Imported collection from {}", file);
        }
    }

    Ok(())
}

fn parse_method(s: &str) -> Result<HttpMethod> {
    match s.to_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "PUT" => Ok(HttpMethod::PUT),
        "DELETE" => Ok(HttpMethod::DELETE),
        "PATCH" => Ok(HttpMethod::PATCH),
        "HEAD" => Ok(HttpMethod::HEAD),
        "OPTIONS" => Ok(HttpMethod::OPTIONS),
        _ => Err(anyhow::anyhow!("Unknown HTTP method: {}", s)),
    }
}

fn parse_content_type(s: &str) -> BodyType {
    match s.to_lowercase().as_str() {
        "json" | "application/json" => BodyType::Json,
        "form" | "application/x-www-form-urlencoded" => BodyType::Form,
        "text" | "text/plain" => BodyType::Text,
        "xml" | "application/xml" => BodyType::Xml,
        _ => BodyType::None,
    }
}
