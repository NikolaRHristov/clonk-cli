use reqwest::cookie::CookieStore;

use clap::{Parser, Subcommand};
use directories::BaseDirs;
use reqwest::{cookie::Jar, Client};
use serde::{Deserialize, Serialize};
use std::{fs, io};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use reqwest::header::HeaderValue;
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },
    Redeem {
        name: String,
        #[arg(long)]
        input: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    Login,
}

#[derive(Serialize, Deserialize)]
struct AuthData {
    username: String,
    password: String,
    cookies: String,
}

#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
    target_url: String,
}

async fn login() -> Result<(), Box<dyn std::error::Error>> {
    print!("Enter username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Enter password: ");
    io::stdout().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();
    let jar = Arc::new(Jar::default());
    let client = Client::builder().cookie_provider(jar.clone()).build()?;

    let login_request = LoginRequest {
        username: username.clone(),
        password: password.clone(),
        target_url: "https://secure.colonq.computer/menu".to_string(),
    };

    let response = client
        .post("https://auth.colonq.computer/api/firstfactor")
        .json(&login_request)
        .send()
        .await?;

    let Some(cookies) = jar.cookies(response.url()) else {
        return Err("Failed to get cookies from response".to_string().into());
    };

    let auth_data = AuthData {
        username,
        password,
        cookies: cookies.to_str().unwrap().to_string(),
    };

    let base_dirs = BaseDirs::new().unwrap();
    let mut auth_path = PathBuf::from(base_dirs.home_dir());
    auth_path.push(".clonk");
    fs::create_dir_all(&auth_path)?;
    auth_path.push("auth");
    fs::write(&auth_path, serde_json::to_string(&auth_data)?)?;

    println!("Successfully logged in.");
    Ok(())
}

async fn redeem(name: String, input: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let base_dirs = BaseDirs::new().unwrap();
    let mut auth_path = PathBuf::from(base_dirs.home_dir());
    auth_path.push(".clonk/auth");
    let auth_data: AuthData = serde_json::from_str(&fs::read_to_string(auth_path)?)?;

    let jar = Arc::new(Jar::default());
    let client = Client::builder().cookie_provider(jar.clone()).build()?;

    jar.set_cookies(&mut std::iter::once(&HeaderValue::from_str(&auth_data.cookies).unwrap()), &"https://secure.colonq.computer".parse().unwrap());

    let form = reqwest::multipart::Form::new()
        .text("name", name)
        .text("input", input.unwrap_or_else(|| "undefined".to_string()));

    let response = client
        .post("https://secure.colonq.computer/api/redeem")
        .multipart(form)
        .send()
        .await?;
    if response.status().is_success() {
        println!("Successfully redeemed");
        Ok(())
    } else {
        Err(format!("Failed to redeem: {}", response.status()).into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { action } => match action {
            AuthCommands::Login => login().await?,
        },
        Commands::Redeem { name, input } => redeem(name, input).await?,
    }

    Ok(())
}
