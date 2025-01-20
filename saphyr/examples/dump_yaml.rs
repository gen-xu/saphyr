use saphyr::Yaml;
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn print_indent(indent: usize) {
    for _ in 0..indent {
        print!("    ");
    }
}

fn dump_node(doc: &Yaml, indent: usize) {
    match *doc {
        Yaml::Sequence { ref value, .. } => {
            for x in value {
                dump_node(x, indent + 1);
            }
        }
        Yaml::Map { ref value, .. } => {
            for (k, v) in value {
                print_indent(indent);
                println!("{k:?}:");
                dump_node(v, indent + 1);
            }
        }
        _ => {
            print_indent(indent);
            println!("{doc:?}");
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut f = File::open(&args[1]).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let docs = Yaml::load_from_str(&s).unwrap();
    for doc in &docs {
        println!("---");
        dump_node(doc, 0);
    }
}
