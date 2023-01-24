use clap::Parser;

#[derive(Parser)]
#[command(author, version, about = "A Command Line One-Time Password client.", long_about = None, arg_required_else_help(true))]
struct Cli {
    #[arg(long, short = 'n', help = "Add a new item", conflicts_with_all = ["list", "remove", "code", "interactive"])]
    new: bool,
    #[arg(
        long,
        short = 'l',
        help = "List the stored items and their current code", conflicts_with_all = ["new", "remove", "code", "interactive"]
    )]
    list: bool,
    #[arg(
        long,
        short = 'r',
        value_name = "LABEL",
        help = "Remove the specified item", conflicts_with_all = ["list", "new", "code", "interactive"]
    )]
    remove: Option<String>,
    #[arg(
        long,
        short = 'c',
        value_name = "LABEL",
        help = "Get the current code of an item", conflicts_with_all = ["list", "remove", "new", "interactive"]
    )]
    code: Option<String>,
    #[cfg(feature = "interactive")]
    #[arg(long, short = 'i', help = "Enter interactive mode", conflicts_with_all = ["list", "remove", "code", "new"])]
    interactive: bool,
}

fn main() {
    let cli = Cli::parse();

    match otpc::modes::run_startup_checks() {
        Some(s) => {
            println!("{}", s);
            std::process::exit(1);
        }
        None => (),
    }

    if cli.new {
        otpc::modes::run_new();
        return;
    } else if cli.list {
        otpc::modes::run_list();
        return;
    } else if let Some(label) = cli.remove {
        otpc::modes::run_remove(&String::from(label));

        return;
    } else if let Some(label) = cli.code {
        otpc::modes::run_display_code(&String::from(label));

        return;
    }

    #[cfg(feature = "interactive")]
    if cli.interactive {
        otpc::modes::run_interactive();

        return;
    }
}
