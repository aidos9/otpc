extern crate clap;
use clap::{App, Arg};
extern crate otpc;

fn main() {
    let matches = App::new("otpc")
        .about("A Command Line One-Time Password client.")
        .version("0.1.0")
        .arg(
            Arg::with_name("new")
                .short("n")
                .long("new")
                .help("Add a new item"),
        )
        .arg(
            Arg::with_name("list")
                .short("l")
                .long("list")
                .help("List the stored items and their current code"),
        )
        .arg(
            Arg::with_name("remove")
                .short("r")
                .long("remove")
                .help("Remove the specified item")
                .value_name("LABEL")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("interactive")
                .short("i")
                .help("Enter interactive mode"),
        )
        .arg(
            Arg::with_name("code")
                .short("c")
                .help("Get the current code of an item")
                .takes_value(true)
                .value_name("LABEL"),
        )
        .arg(
            Arg::with_name("edit")
                .short("e")
                .help("Edit an item")
                .takes_value(true)
                .value_name("LABEL"),
        )
        .get_matches();

    match otpc::run_startup_checks() {
        Some(s) => {
            println!("{}", s);
            std::process::exit(1);
        }
        None => (),
    }

    if matches.is_present("new") {
        otpc::run_new();
    }
}
