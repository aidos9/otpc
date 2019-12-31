mod term;
use term::Term;

pub fn run() {
    let err: Option<&'static str>;
    {
        let mut term = Term::new();

        match term.start() {
            Ok(_) => err = None,
            Err(e) => err = Some(e),
        }
    }

    // If we encounter an error we want the term object to go out of scope to clean up the terminal and then display the error.
    match err {
        Some(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
        None => (),
    }
}
