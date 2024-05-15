use curve25519_dalek::scalar::Scalar;
use libspartan::{Instance, SNARKGens, SNARK, InputsAssignment, VarsAssignment};
use merlin::Transcript;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs::File;
use std::io::{self, BufRead};

// Constants
const NUM_DIGITS: usize = 5;
const DIGIT_RANGE: usize = 26;

// Circuit inputs
struct GameInputs {
    hidden_word: Vec<u8>,
    guess: Vec<u8>,
}

// Circuit outputs
struct GameOutputs {
    letter_in_word: Vec<bool>,
    letter_correct: Vec<bool>,
}

// Circuit constraints
fn game_constraints(_inputs: &GameInputs, outputs: &GameOutputs) -> Instance {
    let mut A = Vec::new();
    let mut B = Vec::new();
    let mut C = Vec::new();

    for i in 0..NUM_DIGITS {
        // Constraint: letter_in_word[i] = (guess[i] in hidden_word)
        for j in 0..NUM_DIGITS {
            A.push((i, j, Scalar::from(1u64).to_bytes()));
            B.push((i, NUM_DIGITS + i, Scalar::from(1u64).to_bytes()));
            C.push((i, NUM_DIGITS * 2 + i, Scalar::from(outputs.letter_in_word[i] as u64).to_bytes()));
        }

        // Constraint: letter_correct[i] = (guess[i] == hidden_word[i])
        A.push((NUM_DIGITS + i, i, Scalar::from(1u64).to_bytes()));
        B.push((NUM_DIGITS + i, NUM_DIGITS + i, Scalar::from(1u64).to_bytes()));
        C.push((NUM_DIGITS + i, NUM_DIGITS * 3 + i, Scalar::from(outputs.letter_correct[i] as u64).to_bytes()));
    }

    Instance::new(2 * NUM_DIGITS, 2 * NUM_DIGITS, 2 * NUM_DIGITS, &A, &B, &C).unwrap()
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

    let letter_in_word: Vec<bool> = guess.iter().map(|&d| hidden_word.contains(&d)).collect();
    let letter_correct: Vec<bool> = hidden_word.iter().zip(guess).map(|(h, g)| h == g).collect();

    let outputs = GameOutputs {
        letter_in_word,
        letter_correct,
    };

    let instance = game_constraints(&inputs, &outputs);

    let num_vars = instance.inst.num_vars();
    let num_cons = instance.inst.num_cons();
    let num_inputs = instance.inst.num_inputs();
    let num_non_zero_entries = instance.inst.num_non_zero_entries();

    let gens = SNARKGens::new(num_cons, num_vars, num_inputs, num_non_zero_entries);

    let vars_assignment = VarsAssignment::new(&[
        vec_to_array_32(inputs.hidden_word.clone()),
        vec_to_array_32(inputs.guess.clone()),
        vec_to_array_32(outputs.letter_in_word.iter().map(|&b| b as u8).collect()),
        vec_to_array_32(outputs.letter_correct.iter().map(|&b| b as u8).collect()),
    ]).unwrap();

    let inputs_assignment = InputsAssignment::new(&[
        vec_to_array_32(inputs.hidden_word.clone()),
        vec_to_array_32(inputs.guess.clone()),
    ]).unwrap();

    let (comm, decomm) = SNARK::encode(&instance, &gens);

    let mut prover_transcript = Transcript::new(b"wordle_example");
    let proof = SNARK::prove(&instance, &comm, &decomm, vars_assignment, &inputs_assignment, &gens, &mut prover_transcript);

    let mut proof_bytes = Vec::new();
    proof.write(&mut proof_bytes).unwrap();

    (proof_bytes, outputs.letter_in_word, outputs.letter_correct)
}

// Verifier function
fn verify_game(hidden_word: &[u8], guess: &[u8], proof_bytes: &[u8]) -> bool {
    let inputs = GameInputs {
        hidden_word: hidden_word.to_vec(),
        guess: guess.to_vec(),
    };

    let instance = game_constraints(&inputs, &GameOutputs {
        letter_in_word: vec![false; NUM_DIGITS],
        letter_correct: vec![false; NUM_DIGITS],
    });

    let num_vars = instance.num_vars;
    let num_cons = instance.num_cons;
    let num_inputs = instance.num_inputs;
    let num_non_zero_entries = instance.num_non_zero_entries();

    let gens = SNARKGens::new(num_cons, num_vars, num_inputs, num_non_zero_entries);

    let inputs_assignment = InputsAssignment::new(&[
        vec_to_array_32(inputs.hidden_word.clone()),
        vec_to_array_32(inputs.guess.clone()),
    ]).unwrap();

    let (comm, _) = SNARK::encode(&instance, &gens);

    let mut proof = SNARK::empty();
    proof.read(proof_bytes).unwrap();

    let mut verifier_transcript = Transcript::new(b"wordle_example");
    proof.verify(&comm, &inputs_assignment, &mut verifier_transcript, &gens).is_ok()
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