use std::io::{self, Read};

fn usage() -> ! {
    eprintln!("usage: sigmaos_genoffice_html <sanitize|text|validate> [FILE|-]");
    eprintln!("  reads stdin when FILE is omitted or '-'");
    std::process::exit(2);
}

fn read_input(path: Option<&str>) -> String {
    match path {
        None | Some("-") => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s).expect("failed to read stdin");
            s
        }
        Some(p) => std::fs::read_to_string(p).unwrap_or_else(|e| {
            eprintln!("error: cannot read {p}: {e}");
            std::process::exit(1);
        }),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else { usage() };
    let path = args.next();
    if args.next().is_some() {
        usage();
    }
    let input = read_input(path.as_deref());

    match cmd.as_str() {
        "sanitize" => print!("{}", sigmaos_genoffice_html::sanitize_html(&input)),
        "text" => println!("{}", sigmaos_genoffice_html::html_to_text(&input)),
        "validate" => {
            let errors = sigmaos_genoffice_html::validate_html(&input);
            if errors.is_empty() {
                println!("ok");
            } else {
                for e in &errors {
                    println!("{e}");
                }
                std::process::exit(1);
            }
        }
        _ => usage(),
    }
}
