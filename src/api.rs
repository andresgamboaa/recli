use std::collections::VecDeque;
use std::io::{self, Write};
use colored::*;
use crate::utilities::get_current_date;
use crate::spaced_repetition::{Evaluation, compute_sr_data};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Color, *};
use super::database;
use super::Command;
use super::Mode;

fn clear() {
    print!("{esc}c", esc = 27 as char);
}

fn get_eval_from_user(is_correct: bool) -> Evaluation {
    let mut user_input = String::new();

    loop {
        user_input.clear();
        io::stdin().read_line(&mut user_input).unwrap();

        match user_input.trim().parse::<i64>() {
            Ok(num) if num >= 1 && num <= 3 => {
                let num = num - 1 + if is_correct { 3 } else { 0 }; // offset
                let evaluation: Evaluation = num.into();
                return evaluation;
            },
            Ok(_) => println!("Number must be 1, 2 or 3. Try again."),
            Err(_) => println!("Invalid input. Please enter a valid number."),
        }
    }
}

pub async fn practice(commands: &mut VecDeque<Command>) -> anyhow::Result<()> {
    let mut user_input:String = String::new();
    let original_size = commands.len();

    while commands.len() > 0 {
        clear();
        let progress = format!("{}/{}", original_size - commands.len(), original_size);

        let command: Command = commands.pop_front().unwrap();

        println!("{} {} \nTask: {}\n", 
            "RECLI".magenta().bold(),
            progress,
            command.task.cyan()
        );

        if let Some(context) = &command.context {
            print!("{}\n\n", context.trim());
        }

        match &command.prompt {
            Some(prompt) => print!("{} ", prompt.trim()),
            None => print!("> ")
        }

        user_input = String::from("");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut user_input).unwrap();

        let is_correct = command.command.eq(&user_input.trim());

        if is_correct {
            if let Some(response) = &command.response {
                println!("{}\n", response.trim());
            }
            println!("{}", "CORRECT!".green());
        }
        else {
            println!("\n{}", "INCORRECT".red());
            println!("Expected: {}\n", command.command.green());
            commands.push_back(command);
        }
        println!("{}", "Press 'Enter' to continue.".cyan());
        io::stdin().read_line(&mut user_input).unwrap();
    }
    clear();
    println!("RECLI: Your have finished your practice.");

    Ok(())
}

pub async fn review(commands: &mut VecDeque<Command>) -> anyhow::Result<()> {
    let mut user_input:String = String::new();
    let original_size = commands.len();

    while commands.len() > 0 {
        clear();
        let progress = format!("{}/{}", original_size - commands.len(), original_size);

        let mut command: Command = commands.pop_front().unwrap();

        println!("{} {} \nTask: {}\n", 
            "RECLI".magenta().bold(),
            progress,
            command.task.cyan()
        );

        if let Some(context) = &command.context {
            print!("{}\n", context.trim());
        }

        match &command.prompt {
            Some(prompt) => print!("{} ", prompt.trim()),
            None => print!("> ")
        }

        user_input = String::from("");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut user_input).unwrap();

        let is_correct = command.command.eq(&user_input.trim());
        let user_eval;

        if let Mode::Learning = command.sr_data.mode {
            if is_correct {
                if let Some(response) = &command.response {
                    println!("{}\n", response.trim());
                }

                let eval1 = compute_sr_data(&command.sr_data, &Evaluation::CorrectButHard, false);
                let eval2= compute_sr_data(&command.sr_data, &Evaluation::CorrectWithHesitation, false);
                let eval3 = compute_sr_data(&command.sr_data, &Evaluation::Perfect, false);

                let current_e_factor = (command.sr_data.e_factor * 100.0).floor();
                let hard_e_factor_decrease = format!("-{}%",(current_e_factor - (eval1.e_factor * 100.0)).abs().floor());
                let perfect_e_factor_increase = format!("+{}%", (current_e_factor - (eval3.e_factor * 100.0)).abs().floor());

                println!("{}", "CORRECT!".green());
                println!("Ease: {}", format!("{}%", current_e_factor.to_string()).cyan());
                println!("1: Hard       {} days  {}", eval1.interval, hard_e_factor_decrease.red());
                println!("2: Good       {} days", eval2.interval);
                println!("3: Perfect    {} days  {}", eval3.interval, perfect_e_factor_increase.green());
            }
            else {
                let eval1 = compute_sr_data(&command.sr_data, &Evaluation::Blackout, false);
                let eval2 = compute_sr_data(&command.sr_data, &Evaluation::IncorrectButRemembered, false);
                let eval3 = compute_sr_data(&command.sr_data, &Evaluation::IncorrectWithEasyRecall, false);

                let current_e_factor = (command.sr_data.e_factor * 100.0).floor();
                let blackout_e_factor_decrease = format!("-{}%",(current_e_factor - (eval1.e_factor * 100.0)).abs().floor());
                let remembered_e_factor_decrease = format!("-{}%",(current_e_factor - (eval2.e_factor * 100.0)).abs().floor());
                let easy_e_factor_decrease = format!("-{}%", (current_e_factor - (eval3.e_factor * 100.0)).abs().floor());
                println!("{}", "INCORRECT".red());
                println!("Expected: {}\n", command.command.green());
                println!("{}", "Command scheduled for tomorrow".yellow());
                println!("Ease: {}", format!("{}%", current_e_factor.to_string()).cyan());
                println!("1: Complete blackout  {}", blackout_e_factor_decrease.red());
                println!("2: Remembered         {}", remembered_e_factor_decrease.red());
                println!("3: Easy recall        {}", easy_e_factor_decrease.red());
            }
            user_eval = get_eval_from_user(is_correct);
        }
        else { // If command is new or failed evalutate automatically
            if is_correct {
                if let Some(response) = &command.response {
                    println!("{}\n", response);
                }
                user_eval = Evaluation::CorrectButHard;
            }
            else {
                println!("{}", "Incorrect".red());
                println!("Expected: {}\n", command.command.green());

                user_eval = Evaluation::Blackout;
            }
            println!("{}", "Press 'Enter' to continue.".cyan());
            io::stdin().read_line(&mut user_input).unwrap();
        }

        command.sr_data.review_count += 1;
        command.sr_data.last_review = Some(get_current_date());
        command.sr_data = compute_sr_data(&command.sr_data, &user_eval, true);

        database::update_command(&command).await?;

        if user_eval.get_num() < 3 {
            commands.push_back(command);
        }

    }
    clear();
    println!("RECLI: No more commands to review today.");

    Ok(())
}

pub fn show_commands(commands: &Vec<Command>) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(80)
        .set_header(vec![
            Cell::new("Id"),
            Cell::new("Task"),
            Cell::new("Command"),
            Cell::new("Ease"),
            Cell::new("Interval"),
        ]);
    
    for command in commands {
        let ease = (command.sr_data.e_factor*100.0).floor();
        table.add_row(vec![
             Cell::new(command.id.unwrap_or(0)),
             Cell::new(&command.task).fg(Color::Cyan),
             Cell::new(&command.command),
             Cell::new(ease),
             Cell::new(&command.sr_data.interval),
        ]);
    }
    println!("Showing {} commands", commands.len());
    println!("{table}");
}