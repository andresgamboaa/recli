use super::{Command, SRData, Mode};
use sqlx::SqlitePool;
use sqlx::{migrate::MigrateDatabase, Sqlite};
use chrono::{Utc, TimeZone, DateTime};
use std::collections::VecDeque;
use super::utilities::get_current_date;
use std::path::PathBuf;

fn get_database_path() -> String {
    let mut path = match std::env::var_os("HOME") {
        Some(home) => {
            let mut buf = PathBuf::new();
            buf.push(home);
            buf
        }
        None => {
            // On Windows, use the user's profile directory
            match std::env::var_os("USERPROFILE") {
                Some(profile) => {
                    let mut buf = PathBuf::new();
                    buf.push(profile);
                    buf
                }
                None => {
                    // Fallback to a default path if neither HOME nor USERPROFILE is set
                    let mut buf = PathBuf::new();
                    buf.push("."); // Use the current directory as a fallback
                    buf
                }
            }
        }
    };

    // Append a subdirectory for your application
    path.push(".recli");

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&path).expect("Failed to create data directory");

    // Append the database file name
    path.push("recli.db");

    // Convert the PathBuf to a string
    path.to_string_lossy().to_string()
}


pub async fn create_if_not_exists() -> anyhow::Result<()> {
    let db_path : String = get_database_path();

    if !Sqlite::database_exists(&db_path).await.unwrap_or(false) {
        match Sqlite::create_database(&db_path).await {
            Ok(_) => {
                let db: sqlx::Pool<Sqlite> = SqlitePool::connect(&db_path).await.unwrap();

                let query = "
                    CREATE TABLE IF NOT EXISTS 'commands' (
                        'id'	INTEGER,
                        'task'	TEXT NOT NULL UNIQUE,
                        'clues' TEXT,
                        'context'	TEXT,
                        'prompt'	TEXT,
                        'command'	TEXT NOT NULL,
                        'command_alternatives' TEXT,
                        'response'	TEXT,
                        'extra' TEXT,
                        'created' TEXT NOT NULL,
                        'last_review' TEXT,
                        'mode' TEXT NOT NULL,
                        'review_count' INTEGER NOT NULL,
                        'n' INTEGER NOT NULL,
                        'e_factor'  REAL NOT NULL,
                        'interval'  INTEGER NOT NULL,
                        PRIMARY KEY('id' AUTOINCREMENT) 
                    )";

                sqlx::query(query).execute(&db).await.unwrap();

                let query = "
                    CREATE TABLE IF NOT EXISTS 'command_tags' (
                        'tag'	TEXT,
                        'command_id'	INTEGER,
                        PRIMARY KEY('tag', 'command_id'),
                        FOREIGN KEY ('command_id') REFERENCES commands('id') 
                    )";

                sqlx::query(query).execute(&db).await.unwrap();
            }
            Err(error) => panic!("error: {}", error),
        }
    }

    Ok(())
}

fn text_to_datetime(input: &str) -> DateTime<Utc> {
    let parsed_datetime = DateTime::parse_from_rfc3339(input)
        .expect("Failed to parse datetime");

    // Convert to Utc
    let utc_datetime: DateTime<Utc> = Utc.from_utc_datetime(&parsed_datetime.naive_utc());
    utc_datetime
}

pub async fn find_today_commands() -> anyhow::Result<VecDeque<Command>> {
    let mut max_per_day = 100;
    let db_path = get_database_path();
    let pool = SqlitePool::connect(&&db_path).await?;

    let mut commands:VecDeque<Command> = VecDeque::new();

    let results = sqlx::query!("SELECT * FROM commands")
        .fetch_all(&pool)
        .await?;

    for result in &results {
        if result.mode == "New" { continue }
        println!("- {}", result.last_review.clone().unwrap_or(String::from("-")));
        let last_review = text_to_datetime(&result.last_review.clone().unwrap());

        if last_review == get_current_date() {
            max_per_day -= 1;
        }
    }

    for result in results {
        if commands.len() >= max_per_day { break; }

        /*
        let tags= sqlx::query!("
            SELECT tag FROM command_tags WHERE command_id = ?", 
            result.id
        )
            .fetch_all(&pool)
            .await?;

        let tags:Vec<String> = tags.iter().map(|t| {
            t.tag.clone().unwrap()
        }).collect();
        */
        let command = Command { 
            id: Some(result.id), 
            task: result.task, 
            context: result.context, 
            prompt: result.prompt, 
            command: result.command, 
            response: result.response, 
            tags: None,
            sr_data: SRData {
                created: text_to_datetime(&result.created),
                last_review: result.last_review.map(|review| text_to_datetime(&review)),
                mode: match result.mode.as_str() {
                    "Learning" => Mode::Learning,
                    "Failed" => Mode::Failed,
                    _ => Mode::New
                },
                review_count: result.review_count,
                n: result.n,
                e_factor: result.e_factor,
                interval: result.interval,
            },
        };

        match &command.sr_data.mode {
            Mode::New | Mode::Failed => {
                commands.push_back(command);
            },
            Mode::Learning => {
                if command.is_pending() {
                    commands.push_back(command);
                }
            }
        }

    }

    Ok(commands)
}

pub async fn find_commands() -> anyhow::Result<VecDeque<Command>> {
    let db_path = get_database_path();
    let pool = SqlitePool::connect(&db_path).await?;

    let mut commands:VecDeque<Command> = VecDeque::new();

    let results = sqlx::query!("
        SELECT c.*
        FROM commands AS c
        INNER JOIN command_tags AS ct ON c.id = ct.command_id
    ",)
        .fetch_all(&pool)
        .await?;

    for result in results {
        let command = Command { 
            id: Some(result.id), 
            task: result.task, 
            context: result.context, 
            prompt: result.prompt, 
            command: result.command, 
            response: result.response, 
            tags: None,
            sr_data: SRData {
                created: text_to_datetime(&result.created),
                last_review: result.last_review.map(|review| text_to_datetime(&review)),
                mode: match result.mode.as_str() {
                    "Learning" => Mode::Learning,
                    "Failed" => Mode::Failed,
                    _ => Mode::New
                },
                review_count: result.review_count,
                n: result.n,
                e_factor: result.e_factor,
                interval: result.interval,
            },
        };
        commands.push_back(command);
    }

    Ok(commands)
}

pub async fn find_commands_with_tag(tag:&str) -> anyhow::Result<VecDeque<Command>> {
    let db_path = get_database_path();
    let pool = SqlitePool::connect(&db_path).await?;

    let mut commands:VecDeque<Command> = VecDeque::new();

    let results = sqlx::query!("
        SELECT c.*
        FROM commands AS c
        INNER JOIN command_tags AS ct ON c.id = ct.command_id
        WHERE ct.tag = ?
    ", tag)
        .fetch_all(&pool)
        .await?;

    for result in results {
        let command = Command { 
            id: Some(result.id), 
            task: result.task, 
            context: result.context, 
            prompt: result.prompt, 
            command: result.command, 
            response: result.response, 
            tags: None,
            sr_data: SRData {
                created: text_to_datetime(&result.created),
                last_review: result.last_review.map(|review| text_to_datetime(&review)),
                mode: match result.mode.as_str() {
                    "Learning" => Mode::Learning,
                    "Failed" => Mode::Failed,
                    _ => Mode::New
                },
                review_count: result.review_count,
                n: result.n,
                e_factor: result.e_factor,
                interval: result.interval,
            },
        };
        commands.push_back(command);
    }

    Ok(commands)
}

pub async fn save_commands(commands: &Vec<Command>) -> anyhow::Result<()> {
    let db_path = get_database_path();
    let pool = SqlitePool::connect(&db_path).await?;

    let sr_data = SRData::default();

    for command in commands {
        let id = sqlx::query!(
            r#"
                INSERT OR IGNORE INTO 'commands' (task, context, prompt, command, response, created, last_review, mode, review_count, n, e_factor, interval) VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12);
            "#,
            command.task,
            command.context,
            command.prompt,
            command.command,
            command.response,
            sr_data.created,
            sr_data.last_review,
            sr_data.mode,
            sr_data.review_count,
            sr_data.n,
            sr_data.e_factor,
            sr_data.interval
        )
        .execute(&pool).await?.last_insert_rowid();

        if id == 0 { continue; }

        if let Some(tags) = &command.tags {
            for tag in tags {
                sqlx::query!(
                    r#"
                        INSERT OR IGNORE INTO 'command_tags' (tag, command_id) VALUES 
                        (?1, ?2);
                    "#,
                    tag,
                    id
                )
                .execute(&pool).await?;
            }
        }
    }

    println!("{} commands saved.", commands.len());

    Ok(())
}

pub async fn update_command(command: &Command) -> anyhow::Result<()> {
    let db_path = get_database_path();
    let pool = SqlitePool::connect(&db_path).await?;

    let query = "
        UPDATE 'commands' SET 
            last_review = $1,
            mode = $2,
            review_count = $3,
            n = $4,
            e_factor = $5,
            interval = $6 
        WHERE id = $7;
    ";

    let sr_data = &command.sr_data;

    sqlx::query(query)
        .bind(sr_data.last_review)
        .bind(sr_data.mode.to_string())
        .bind(sr_data.review_count)
        .bind(sr_data.n)
        .bind(sr_data.e_factor)
        .bind(sr_data.interval)
        .bind(&command.id.expect("The id is expected in order to update."))
        .execute(&pool).await?;

    Ok(())
}