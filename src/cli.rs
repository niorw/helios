use crate::config;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = config::APP_NAME)]
#[command(about = config::APP_ABOUT)]
#[command(version = config::APP_VERSION)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Send an HTTP request")]
    Send {
        #[arg(help = "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)")]
        method: String,
        #[arg(help = "Request URL")]
        url: String,
        #[arg(short, long, help = "Request headers (Key:Value)")]
        header: Vec<String>,
        #[arg(short, long, help = "Request body")]
        body: Option<String>,
        #[arg(short, long, help = "Content-Type (json, form, text, xml)")]
        content_type: Option<String>,
    },
    #[command(about = "Manage collections")]
    Collection {
        #[command(subcommand)]
        action: CollectionAction,
    },
    #[command(about = "Manage environments")]
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
    #[command(about = "Run a collection")]
    Run {
        #[arg(help = "Collection name or ID")]
        name: String,
    },
    #[command(about = "Export a collection")]
    Export {
        #[arg(help = "Collection name or ID")]
        name: String,
        #[arg(short, long, default_value = "json", help = "Export format (json, postman)")]
        format: String,
        #[arg(short, long, help = "Output file path")]
        output: Option<String>,
    },
    #[command(about = "Import a collection")]
    Import {
        #[arg(help = "File path to import")]
        file: String,
        #[arg(short, long, default_value = "auto", help = "Import format (auto, json, postman)")]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum CollectionAction {
    #[command(about = "List all collections")]
    List,
    #[command(about = "Create a new collection")]
    Add {
        #[arg(help = "Collection name")]
        name: String,
    },
    #[command(about = "Delete a collection")]
    Remove {
        #[arg(help = "Collection name or ID")]
        name: String,
    },
    #[command(about = "Add a request to collection")]
    AddReq {
        #[arg(help = "Collection name or ID")]
        collection: String,
        #[arg(help = "HTTP method")]
        method: String,
        #[arg(help = "Request URL")]
        url: String,
        #[arg(short, long, help = "Request name")]
        name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum EnvAction {
    #[command(about = "List environments")]
    List,
    #[command(about = "Create environment")]
    Add {
        #[arg(help = "Environment name")]
        name: String,
    },
    #[command(about = "Set variable")]
    Set {
        #[arg(help = "Environment name or ID")]
        env: String,
        #[arg(help = "Variable key")]
        key: String,
        #[arg(help = "Variable value")]
        value: String,
    },
    #[command(about = "Delete environment")]
    Remove {
        #[arg(help = "Environment name or ID")]
        name: String,
    },
}
