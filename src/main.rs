use clap::{Arg, Command};

fn main() {
    let mut app = Command::new("otpc")
        .arg_required_else_help(true)
        .about("A Command Line One-Time Password client.")
        .version("2.0.0")
        .arg(
            Arg::new("new")
                .short('n')
                .long("new")
                .help("Add a new item"),
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("List the stored items and their current code"),
        )
        .arg(
            Arg::new("remove")
                .short('r')
                .long("remove")
                .help("Remove the specified item")
                .value_name("LABEL")
                .takes_value(true),
        )
        .arg(
            Arg::new("code")
                .short('c')
                .long("code")
                .help("Get the current code of an item")
                .takes_value(true)
                .value_name("LABEL"),
        );

    if cfg!(feature = "interactive") {
        app = app.arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Enter interactive mode"),
        );
    }

    let matches = app.get_matches();

    match otpc::modes::run_startup_checks() {
        Some(s) => {
            println!("{}", s);
            std::process::exit(1);
        }
        None => (),
    }

    if matches.is_present("new") {
        otpc::modes::run_new();
        return;
    } else if matches.is_present("list") {
        otpc::modes::run_list();
        return;
    } else if matches.is_present("remove") {
        match matches.value_of("remove") {
            Some(label) => otpc::modes::run_remove(&String::from(label)),
            None => {
                eprintln!("A value is required to remove an item.");
                std::process::exit(1);
            }
        }

        return;
    } else if matches.is_present("code") {
        match matches.value_of("code") {
            Some(label) => otpc::modes::run_display_code(&String::from(label)),
            None => {
                eprintln!("A value is required to show the code of an item.");
                std::process::exit(1);
            }
        }

        return;
    } else if cfg!(feature = "interactive") {
        if matches.is_present("interactive") {
            #[cfg(feature = "interactive")]
            otpc::modes::run_interactive();
        }

        return;
    }
}
