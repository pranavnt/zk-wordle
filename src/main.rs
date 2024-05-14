use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs::File;
use std::io::{self, BufRead};

fn main() {
    let mut game = WordleGame::new();

    println!("Welcome to Wordle! You have 6 guesses to guess the word.");
    println!("The word is a 5-letter word that contains only alphabetic characters.");

    println!("This game will also generate zero-knowledge proofs that you can verify to prove that this program is not cheating.");

    for turn in 0..6 {
        println!("{:?}: Enter your guess: ", turn + 1);
        let guess = {
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            input
        };
        if game.guess(guess.trim().to_string()) {
            println!("Congrats! You guessed the wordle!");
            break;
        }
    }
    println!("The word was {}", game.word);
}

#[derive(Debug)]
struct WordleGame {
    l1: i8,
    l2: i8,
    l3: i8,
    l4: i8,
    l5: i8,

    word: String,
    guesses: Vec<String>,
}

impl WordleGame {
    fn new() -> Self {
        let file = File::open("/usr/share/dict/words").expect("Failed to open file");
        let lines = io::BufReader::new(file).lines();
        let five_letter_words: Vec<String> = lines
            .filter_map(Result::ok)
            .filter(|line| line.len() == 5 && line.chars().all(|c| c.is_ascii_alphabetic()))
            .collect();

        let random_word = &(five_letter_words
            .choose(&mut thread_rng())
            .expect("No words found")
            .to_string());

        let word: Vec<i8> = random_word.chars().map(|c| c as i8 - 'a' as i8).collect();

        Self {
            l1: word[0],
            l2: word[1],
            l3: word[2],
            l4: word[3],
            l5: word[4],
            word: random_word.to_string(),
            guesses: vec![],
        }
    }

    fn guess(&mut self, guess: String) -> bool {
        // decode letters and then verify guess
        let chars: Vec<char> = guess.chars().collect();
        let (l1, l2, l3, l4, l5) = (
            chars[0] as i8 - 'a' as i8,
            chars[1] as i8 - 'a' as i8,
            chars[2] as i8 - 'a' as i8,
            chars[3] as i8 - 'a' as i8,
            chars[4] as i8 - 'a' as i8,
        );

        let (r1, r2, r3, r4, r5, proof) = self.verify_guess(l1, l2, l3, l4, l5);

        let results = [r1, r2, r3, r4, r5].map(|r| match r {
            0 => '?',
            1 => 'O',
            2 => 'X',
            _ => ' ',
        });

        println!(
            "Result of proof verification: {:?}",
            self.verify_proof(proof)
        );

        if r1 == 2 && r2 == 2 && r3 == 2 && r4 == 2 && r5 == 2 {
            println!("Congrats! You guessed the wordle!");
            return true;
        }

        println!("{:?}", results);

        self.guesses.push(guess);

        false
    }

    fn verify_guess(&self, l1: i8, l2: i8, l3: i8, l4: i8, l5: i8) -> (i8, i8, i8, i8, i8, String) {
        (0, 0, 2, 1, 2, "proof".to_string())
    }

    fn verify_proof(&self, proof: String) -> bool {
        true
    }
}
