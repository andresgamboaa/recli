use crate::utilities::get_current_date;
use rand::Rng;

use super::{
    SRData,
    Mode
};

pub enum Evaluation {
    Blackout,
    IncorrectButRemembered,
    IncorrectWithEasyRecall,
    CorrectButHard,
    CorrectWithHesitation,
    Perfect
}

impl From<i64> for Evaluation {
    fn from(value: i64) -> Self {
        match value {
            0 => Evaluation::Blackout,
            1 => Evaluation::IncorrectButRemembered,
            2 => Evaluation::IncorrectWithEasyRecall,
            3 => Evaluation::CorrectButHard,
            4 => Evaluation::CorrectWithHesitation,
            5 => Evaluation::Perfect,
            _ => panic!("Invalid value for Evaluation"),
        }
    }
}

impl Evaluation {
    pub fn get_num(&self) -> i64 {
        match self {
            Evaluation::Blackout => 0,
            Evaluation::IncorrectButRemembered => 1,
            Evaluation::IncorrectWithEasyRecall => 2,
            Evaluation::CorrectButHard => 3,
            Evaluation::CorrectWithHesitation => 4,
            Evaluation::Perfect => 5,
        }
    }
}

pub fn compute_sr_data(sr_data: &SRData, evaluation: &Evaluation, add_noise_to_interval: bool) -> SRData {
    let mut output = sr_data.clone();

    if let Mode::Failed = sr_data.mode {
        if sr_data.last_review.unwrap() == get_current_date() {
            if evaluation.get_num() >= 3 {
                output.mode = Mode::Learning; 
            }
            return output;
        }
    }

    if let Mode::Learning = sr_data.mode {
        output.e_factor = 1.3_f64.max(
            sr_data.e_factor + (0.1 - (5.0-(evaluation.get_num() as f64)) * (0.08 + (5.0 - evaluation.get_num() as f64) * 0.02))
        );
    }

    if evaluation.get_num() >= 3 {
        output.mode = Mode::Learning;
        
        output.interval = match sr_data.n {
            0 => 1,
            1 => 6,
            _ => {
                (sr_data.interval as f64 * output.e_factor).ceil() as i64 + if add_noise_to_interval { 
                    generate_noise(sr_data.interval)} else { 0 }
            }
        };

        output.n += 1;
    }
    else if evaluation.get_num() < 3 {
        if let Mode::Learning = sr_data.mode {
            output.mode = Mode::Failed;
            output.n = 1;
            output.interval = 1;
        }
    }

    output
}


fn generate_noise(input: i64) -> i64 {
    if input <= 4 {
        return 0;
    }

    let factor = 0.1;
    let mut rng = rand::thread_rng();
    let noise_range = (input as f64).abs() * factor; // Adjust the factor as needed
    let noise = rng.gen_range(-noise_range..noise_range);

    noise.ceil() as i64
}


#[cfg(test)]
mod tests {
    use chrono::Duration;

    use crate::spaced_repetition::{compute_sr_data, Evaluation};
    use crate::{SRData, Mode};
    use crate::utilities::get_current_date;

    #[test]
    fn it_works() {
        println!("{:#?}", 
            compute_sr_data(&SRData {
                last_review: Some(get_current_date() - Duration::days(30)),
                interval: 1,
                n: 0,
                mode: Mode::Learning,
                ..Default::default() 
            }, &Evaluation::CorrectButHard, false)
        )
    }
}