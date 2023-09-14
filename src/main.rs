mod api;
mod database;
mod spaced_repetition;
pub mod utilities;
use clap::{Parser, Subcommand};
use serde_derive::Deserialize;
use dotenv::dotenv;
use chrono::{DateTime, Utc};
use utilities::get_current_date;
use api::{review, practice, show_commands};

#[derive(sqlx::Type, Debug, Clone)]
pub enum Mode {
    New,
    Learning,
    Failed
}

impl ToString for Mode {
    fn to_string(&self) -> String {
        match self {
           Mode::New => String::from("New"), 
           Mode::Learning => String::from("Learning"), 
           Mode::Failed => String::from("Failed"), 
        }
    }
}

#[derive(sqlx::Type, Debug, Clone)]
pub struct SRData {
    pub created: DateTime<Utc>,
    pub last_review: Option<DateTime<Utc>>,
    pub mode: Mode,
    pub review_count: i64,
    pub n: i64,
    pub e_factor: f64,
    pub interval: i64,
}

impl Default for SRData {
    fn default() -> Self {
        Self {
            created: utilities::get_current_date(),
            last_review: None,
            review_count: 0,
            mode: Mode::New,
            n: 0,
            e_factor: 2.5,
            interval: 1
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Command {
    pub id: Option<i64>,
    pub task: String,
    //pub clues: Option<String>,
    pub context: Option<String>, 
    pub prompt: Option<String>,
    pub command: String,
    //pub command_alternatives: Option<Vec<String>>,
    pub response: Option<String>,
    //pub extra: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(skip)]
    pub sr_data: SRData
}

impl Command {
    fn is_pending(&self) -> bool {
        let current = get_current_date();
        let duration = current - self.sr_data.last_review.unwrap();
        duration.num_days() >= self.sr_data.interval
    }
}

/// A program that helps you remember cli commands
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Review commands schedule for today
    Review,
    /// Add new commands from a toml file
    Import {
        #[arg(value_name = "FILE")]
        file_path: String
    },
    /// Practice commands
    Practice {
        #[arg(value_name = "TAG")]
        tag: String
    },
    /// Show saved commands
    Show { 
        #[arg(value_name = "TAG")]
        tag: Option<String>
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    dotenv().ok(); 

    let cli = Cli::parse();
    database::create_if_not_exists().await?;

    match &cli.command {
        Commands::Review => {
            let mut commands = database::find_today_commands().await?;
            review(&mut commands).await?;
        },
        Commands::Import { file_path } =>  {
            let commands = utilities::get_commands_from_toml(&file_path);
            database::save_commands(&commands).await?; 
        },
        Commands::Practice { tag } => {
            let mut commands = database::find_commands_with_tag(tag).await?;
            practice(&mut commands).await?;
        },
        Commands::Show { tag } => {
            let commands = match tag {
                Some(tag) => database::find_commands_with_tag(tag).await?,
                None => database::find_commands().await?
            };
            let vec: Vec<Command> = Vec::from(commands);
            show_commands(&vec);
        },
    }
    Ok(())
}
