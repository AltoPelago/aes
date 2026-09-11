use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use aes_telex::film_candidate_a::{FilmStream, encode_film_candidate_a};
use aes_telex::film_candidate_b::encode_film_candidate_b;
use aes_telex::parse_telex;

fn main() {
    let paths = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        eprintln!("usage: compare_film_layouts <canonical.telex.aes> [...]");
        std::process::exit(2);
    }
    println!(
        "fixture,events,telex_bytes,candidate_a_bytes,candidate_b_bytes,a_over_telex,b_over_a"
    );
    for path in paths {
        compare(&path);
    }
}

fn compare(path: &Path) {
    let telex = fs::read_to_string(path).expect("Telex fixture must be readable UTF-8");
    let parsed = parse_telex(&telex).expect("Telex fixture must parse");
    assert!(
        parsed.canonical,
        "fixture must be canonical: {}",
        path.display()
    );
    let stream = FilmStream::from(&parsed);
    let candidate_a = encode_film_candidate_a(&stream, &[])
        .unwrap_or_else(|error| panic!("Candidate A failed for {}: {error}", path.display()));
    let candidate_b = encode_film_candidate_b(&stream, &[])
        .unwrap_or_else(|error| panic!("Candidate B failed for {}: {error}", path.display()));
    println!(
        "{},{},{},{},{},{:.4},{:.4}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("fixture"),
        stream.records.len(),
        telex.len(),
        candidate_a.len(),
        candidate_b.len(),
        candidate_a.len() as f64 / telex.len() as f64,
        candidate_b.len() as f64 / candidate_a.len() as f64,
    );
    if let Ok(directory) = env::var("FILM_COMPARE_OUTPUT_DIR") {
        let stem = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("fixture");
        let directory = Path::new(&directory);
        fs::create_dir_all(directory).expect("comparison output directory must be creatable");
        fs::write(
            directory.join(format!("{stem}.candidate-a.bin")),
            candidate_a,
        )
        .expect("Candidate A output must be writable");
        fs::write(
            directory.join(format!("{stem}.candidate-b.bin")),
            candidate_b,
        )
        .expect("Candidate B output must be writable");
        fs::write(directory.join(stem), telex).expect("Telex output must be writable");
    }
}
