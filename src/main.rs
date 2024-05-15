use curve25519_dalek::scalar::Scalar;
use libspartan::{Instance, SNARKGens, SNARK, InputsAssignment, VarsAssignment};
use merlin::Transcript;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs::File;
use std::io::{self, BufRead};

use bincode;


// Constants
const NUM_DIGITS: usize = 5;
const DIGIT_RANGE: usize = 26;

// Circuit inputs
struct GameInputs {
    hidden_word: Vec<u8>,
    guess: Vec<u8>,
}

struct GameOutputs {
    letter_in_word: Vec<bool>,
    letter_correct: Vec<bool>,
}

// Circuit constraints
fn game_constraints(inputs: &GameInputs, outputs: &GameOutputs) -> Instance {
    let num_cons = NUM_DIGITS * 2;
    let num_vars = NUM_DIGITS * 2;
    let num_inputs = NUM_DIGITS * 2;
    let num_non_zero_entries = NUM_DIGITS * 2;

    let mut A = Vec::new();
    let mut B = Vec::new();
    let mut C = Vec::new();

    for i in 0..NUM_DIGITS {
        let hidden_char = inputs.hidden_word[i] as usize;
        let guess_char = inputs.guess[i] as usize;

        A.push((i, hidden_char, Scalar::from(1u8).to_bytes()));
        B.push((i, guess_char + DIGIT_RANGE, Scalar::from(1u8).to_bytes()));
        C.push((i, i, Scalar::from(outputs.letter_in_word[i] as u8).to_bytes()));

        A.push((i + NUM_DIGITS, hidden_char, Scalar::from(1u8).to_bytes()));
        B.push((i + NUM_DIGITS, guess_char, Scalar::from(1u8).to_bytes()));
        C.push((i + NUM_DIGITS, i + NUM_DIGITS, Scalar::from(outputs.letter_correct[i] as u8).to_bytes()));

        // Replace Scalar::zero() with Scalar::from(0u8)
        let mut vars = vec![Scalar::from(0u8).to_bytes(); num_vars];
        for i in 0..NUM_DIGITS {
            // Replace Scalar::one() with Scalar::from(1u8)
            vars[inputs.hidden_word[i] as usize] = Scalar::from(1u8).to_bytes();
            vars[inputs.guess[i] as usize + DIGIT_RANGE] = Scalar::from(1u8).to_bytes();
        }

    }

    Instance::new(num_cons, num_vars, num_inputs, &A, &B, &C).unwrap()
}

// Helper function to convert Vec<u8> to [u8; 32]
fn vec_to_array_32(vec: Vec<u8>) -> [u8; 32] {
    let mut array = [0u8; 32];
    array[..vec.len()].copy_from_slice(&vec);
    array
}

// Prover function
fn prove_game(hidden_word: &[u8], guess: &[u8]) -> (Vec<u8>, Vec<bool>, Vec<bool>) {
    let inputs = GameInputs {
        hidden_word: hidden_word.to_vec(),
        guess: guess.to_vec(),
    };

    let letter_in_word: Vec<bool> = inputs.hidden_word
        .iter()
        .map(|&h| inputs.guess.contains(&h))
        .collect();

    let letter_correct: Vec<bool> = inputs.hidden_word
        .iter()
        .zip(inputs.guess.iter())
        .map(|(h, g)| h == g)
        .collect();

    let outputs = GameOutputs {
        letter_in_word,
        letter_correct,
    };

    let inst = game_constraints(&inputs, &outputs);
    let num_cons = NUM_DIGITS * 2;
    let num_vars = NUM_DIGITS * 2;
    let num_inputs = NUM_DIGITS * 2;
    let num_non_zero_entries = NUM_DIGITS * 2;

    let gens = SNARKGens::new(num_cons, num_vars, num_inputs, num_non_zero_entries);

    let (comm, decomm) = SNARK::encode(&inst, &gens);

    let mut vars = vec![Scalar::from(0u8).to_bytes(); num_vars];
    for i in 0..NUM_DIGITS {
        // Replace Scalar::one() with Scalar::from(1u8)
        vars[inputs.hidden_word[i] as usize] = Scalar::from(1u8).to_bytes();
        vars[inputs.guess[i] as usize + DIGIT_RANGE] = Scalar::from(1u8).to_bytes();
    }

    let assignment_vars = VarsAssignment::new(&vars).unwrap();
    let assignment_inputs = InputsAssignment::new(&[vec_to_array_32(inputs.hidden_word.clone())]).unwrap();


    let mut prover_transcript = Transcript::new(b"zk_wordle");
    let proof = SNARK::prove(
        &inst,
        &comm,
        &decomm,
        assignment_vars,
        &assignment_inputs,
        &gens,
        &mut prover_transcript,
    );

    let proof_bytes = bincode::serialize(&proof).unwrap();
    (proof_bytes, outputs.letter_in_word, outputs.letter_correct)
}

// Verifier function
fn verify_game(hidden_word: &[u8], guess: &[u8], proof_bytes: &[u8]) -> bool {
    let inputs = GameInputs {
        hidden_word: hidden_word.to_vec(),
        guess: guess.to_vec(),
    };

    let outputs = GameOutputs {
        letter_in_word: vec![false; NUM_DIGITS],
        letter_correct: vec![false; NUM_DIGITS],
    };

    let inst = game_constraints(&inputs, &outputs);
    let num_cons = NUM_DIGITS * 2;
    let num_vars = NUM_DIGITS * 2;
    let num_inputs = NUM_DIGITS * 2;
    let num_non_zero_entries = NUM_DIGITS * 2;

    let gens = SNARKGens::new(num_cons, num_vars, num_inputs, num_non_zero_entries);

    let (comm, _) = SNARK::encode(&inst, &gens);

    let proof: SNARK = bincode::deserialize(proof_bytes).unwrap();

    let assignment_inputs = InputsAssignment::new(&[vec_to_array_32(inputs.hidden_word.clone())]).unwrap();

    let mut verifier_transcript = Transcript::new(b"zk_wordle");
    proof
        .verify(&comm, &assignment_inputs, &mut verifier_transcript, &gens)
        .is_ok()
}

fn main() {
    let file = File::open("/usr/share/dict/words").expect("Failed to open file");
    let lines = io::BufReader::new(file).lines();
    let five_letter_words: Vec<String> = lines
        .filter_map(Result::ok)
        .filter(|line| line.len() == 5 && line.chars().all(|c| c.is_ascii_alphabetic()))
        .collect();

    let random_word = five_letter_words
        .choose(&mut thread_rng())
        .expect("No words found")
        .to_string();

    let hidden_word: Vec<u8> = random_word.chars().map(|c| c as u8 - b'a').collect();

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

        let guess_word: Vec<u8> = guess.trim().chars().map(|c| c as u8 - b'a').collect();
        let (proof_bytes, letter_in_word, letter_correct) = prove_game(&hidden_word, &guess_word);

        println!("Letter in word: {:?}", letter_in_word);
        println!("Letter correct: {:?}", letter_correct);

        let verified = verify_game(&hidden_word, &guess_word, &proof_bytes);
        println!("Verification result: {}", verified);

        if letter_correct.iter().all(|&b| b) {
            println!("Congrats! You guessed the wordle!");
            break;
        }
    }

    println!("The word was {}", random_word);
}