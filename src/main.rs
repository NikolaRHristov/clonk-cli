use std::{fs, io, io::Write, path::PathBuf, sync::Arc};

use clap::{Parser, Subcommand};
use directories::BaseDirs;
use reqwest::{
	cookie::{CookieStore, Jar},
	header::HeaderValue,
	Client,
};
use serde::{Deserialize, Serialize};
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command:Commands,
}

#[derive(Subcommand)]
enum Commands {
	Auth {
		#[command(subcommand)]
		action:AuthCommands,
	},
	Redeem {
		name:String,
		#[arg(long)]
		input:Option<String>,
	},
}

#[derive(Subcommand)]
enum AuthCommands {
	Login,
}

#[derive(Serialize, Deserialize)]
struct AuthData {
	username:String,
	password:String,
	cookies:String,
}

#[derive(Serialize)]
struct LoginRequest {
	username:String,
	password:String,
	target_url:String,
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

	let Some(cookies) = jar.cookies(
		client
			.post("https://auth.colonq.computer/api/firstfactor")
			.json(&LoginRequest {
				username:username.clone(),
				password:password.clone(),
				target_url:"https://secure.colonq.computer/menu".to_string(),
			})
			.send()
			.await?
			.url(),
	) else {
		return Err("Failed to get cookies from response".to_string().into());
	};

	let mut auth_path = PathBuf::from(BaseDirs::new().unwrap().home_dir());

	auth_path.push(".clonk");

	fs::create_dir_all(&auth_path)?;

	auth_path.push("auth");

	fs::write(
		&auth_path,
		serde_json::to_string(&AuthData {
			username,
			password,
			cookies:cookies.to_str().unwrap().to_string(),
		})?,
	)?;

	println!("Successfully logged in.");

	Ok(())
}

async fn redeem(name:String, input:Option<String>) -> Result<(), Box<dyn std::error::Error>> {
	let mut auth_path = PathBuf::from(BaseDirs::new().unwrap().home_dir());

	auth_path.push(".clonk/auth");

	let jar = Arc::new(Jar::default());

	jar.set_cookies(
		&mut std::iter::once(
			&HeaderValue::from_str(
				&(serde_json::from_str(&fs::read_to_string(auth_path)?)? as AuthData).cookies,
			)
			.unwrap(),
		),
		&"https://secure.colonq.computer".parse().unwrap(),
	);

	let response = Client::builder()
		.cookie_provider(jar.clone())
		.build()?
		.post("https://secure.colonq.computer/api/redeem")
		.multipart(
			reqwest::multipart::Form::new()
				.text("name", name)
				.text("input", input.unwrap_or_else(|| "undefined".to_string())),
		)
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
	match Cli::parse().command {
		Commands::Auth { action } => {
			match action {
				AuthCommands::Login => login().await?,
			}
		},
		Commands::Redeem { name, input } => redeem(name, input).await?,
	}

	Ok(())
}
