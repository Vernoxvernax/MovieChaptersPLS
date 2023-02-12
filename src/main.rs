use clap::{Arg, Command, ArgAction};
use std::fs::File;
use std::io::Write;
use std::path::Path;

mod mpls;
use mpls::serialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct XMLChapter {
	title: String,
	start: String,
}

#[derive(Debug, Clone)]
pub struct M2ts {
	path: String,
	chapters: Vec<XMLChapter>
}


fn main () {
	let matches = Command::new("moviechapterspls")
		.about("read chapters from mpls and export them as ffmetadata")
		.version(VERSION)
		.arg_required_else_help(true)
		.author("Vernox Vernax")
		.arg(
			Arg::new("00000.mpls")
			.help("MPLS-Playlist from your Blu-ray.")
			.required(true)
			.action(ArgAction::Set)
			.num_args(1)
		)
	.get_matches();
	match matches.args_present() {
		true => {
			let file = matches.get_one::<String>("00000.mpls").unwrap();
			if ! file.ends_with(".mpls") {
				eprintln!("The chapter file must have the extension \".mpls\"!");
				return;
			}

			if ! Path::new(file).is_file()
			{
				eprintln!("\"{}\" does not exist.", file);
				return;
			}

			let m2ts = serialize(file);
			write_ffmetadata(m2ts);
		}
		_ => unreachable!(),
	}
}


fn write_ffmetadata(files: Vec<M2ts>) {
	fn str_to_time(start: String) -> String {
		let start_str: Vec<&str> = start.split(":").collect();
		let hours = start_str.get(0).unwrap().parse::<u64>().unwrap() * 60 * 60;
		let minutes = start_str.get(1).unwrap().parse::<u64>().unwrap() * 60;
		let start_str_2: Vec<&str> = start_str.get(2).unwrap().split(".").collect();
		let seconds = start_str_2.get(0).unwrap().parse::<u64>().unwrap();
		let mut ms_str = start_str_2.get(1).unwrap().to_owned().to_owned();
		loop {
			if ms_str.len() > 3 {
				ms_str.pop();
			} else {
				break
			}
		}
		format!("{}{}", hours + minutes + seconds, ms_str)
	}
	for file in files {
		let mut output: String = ";FFMETADATA1\n".to_string();
		for chapter in file.chapters {
			output = output + "\n[CHAPTER]\nTIMEBASE=1/1000";
			output = output + "\nSTART=" + format!("{}", str_to_time(chapter.start.clone())).as_str();
			output = output + "\nEND=" + format!("{}", str_to_time(chapter.start.clone())).as_str();
			output = output + "\ntitle=" + &chapter.title;
		}
		let mut file = File::create(file.path+".ff").unwrap();
		writeln!(&mut file, "{}", output).unwrap();
	}
}
