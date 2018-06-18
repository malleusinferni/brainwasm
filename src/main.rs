#[macro_use]
extern crate structopt;

#[macro_use]
extern crate failure_derive;
extern crate failure;

extern crate parity_wasm;

pub mod bf;
pub mod wasm;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name="brainwasm")]
struct Opt {
    #[structopt(short="c", long="compile")]
    compile: bool,

    #[structopt(parse(from_os_str))]
    infile: PathBuf,

    #[structopt(short="o", parse(from_os_str))]
    outfile: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();

    let source = std::fs::read_to_string(&opt.infile).unwrap_or_else(|err| {
        panic!("Can't read {}: {}", opt.infile.display(), err);
    });

    let ast = bf::parse(&source).unwrap_or_else(|err| {
        panic!("{}", err);
    });

    if !opt.compile && opt.outfile.is_none() {
        bf::interpret(&ast).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        return;
    }

    let c = ast.into_c();

    if let Some(outpath) = &opt.outfile {
        let mut outfile = File::create(outpath).unwrap();
        writeln!(outfile, "{}", c).unwrap();
    } else {
        println!("{}", c);
    }
}
