use clap::{Arg, Command, ArgAction, arg};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use rand::Rng;

mod mpls;
use mpls::serialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct XMLChapter {
  title: String,
  start: String,
  end: String,
  start_duration: Duration,
  end_duration: Duration
}

#[derive(Debug, Clone)]
pub struct M2ts {
  id: u16,
  path: String,
  chapters: Vec<XMLChapter>
}

fn main () {
  let matches = Command::new("moviechapterspls")
    .about("read chapters from mpls and export them")
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
    .arg(
      Arg::new("XML")
      .long("xml")
      .short('X')
      .help("Output as XML chapters")
      .required(false)
      .action(ArgAction::SetTrue)
    )
    .arg(
      Arg::new("FFmetadata")
      .long("ffmetadata")
      .short('F')
      .help("Output as FFmetadata chapters")
      .required(false)
      .action(ArgAction::SetTrue)
    )
    .arg(
      arg!(-m --merge <id>)
      .long("merge")
      .short('m')
      .help("Merge chapters for <id> until <id> into one chapter file (last one must have chapters). This option will also mind chapters that make up the entirety of the file.")
      .required(false)
      .action(ArgAction::Set)
      .conflicts_with("only")
      .num_args(2)
    )
    .arg(
      arg!(-o --only <"00:00:00.0">)
      .long("only")
      .short('o')
      .help("Output only chapters from <time> to <time> and normalize their start position to 0. For continuous chapterlists that don't have information about files (like DVDs).")
      .action(ArgAction::Set)
      .conflicts_with("merge")
      .num_args(2)
    )
  .get_matches();
  match matches.args_present() {
    true => {
      let file = matches.get_one::<String>("00000.mpls").unwrap();
      let mut xml: bool = *matches.get_one::<bool>("XML").unwrap();
      let ffmetadata: bool = *matches.get_one::<bool>("FFmetadata").unwrap();

      if ! ffmetadata && ! xml {
        xml = true;
      }

      let merge: Vec<&String> = if let Some(ids) = matches.try_get_many::<String>("merge").unwrap() {
        ids.collect()
      } else {
        vec![]
      };

      let only: Vec<&String> = if let Some(time) = matches.try_get_many::<String>("only").unwrap() {
        time.collect()
      } else {
        vec![]
      };

      if ! file.ends_with(".mpls") {
        eprintln!("The chapter file must have the extension \".mpls\"!");
        return;
      }

      if ! Path::new(file).is_file() {
        eprintln!("\"{file}\" does not exist.");
        return;
      }

      let m2ts = serialize(file, merge, only);
      write_chapters(m2ts, xml, ffmetadata);
    }
    _ => unreachable!(),
  }
}

fn str_to_time(start: String) -> String {
  let start_str: Vec<&str> = start.split(':').collect();
  let hours = start_str.get(0).unwrap().parse::<u64>().unwrap() * 60 * 60;
  let minutes = start_str.get(1).unwrap().parse::<u64>().unwrap() * 60;
  let start_str_2: Vec<&str> = start_str.get(2).unwrap().split('.').collect();
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

fn write_chapters(files: Vec<M2ts>, xml: bool, ffmetadata: bool) {
  if xml {
    for file in files.clone() {
      let mut rng = rand::thread_rng();
      let mut output: String = "<?xml version=\"1.0\"?>\n<!-- <!DOCTYPE Chapters SYSTEM \"matroskachapters.dtd\"> -->
<Chapters>
  <EditionEntry>\n".to_string();
      output = output + "    <EditionUID>" + rng.gen::<u64>().to_string().as_str() + "</EditionUID>\n";

      for chapter in file.chapters {
        output += "    <ChapterAtom>\n";
        output = output + "      <ChapterUID>" + rng.gen::<u64>().to_string().as_str() + "</ChapterUID>\n";
        output = output + "      <ChapterTimeStart>" + &chapter.start + "</ChapterTimeStart>\n";
        output = output + "      <ChapterTimeEnd>" + &chapter.end + "</ChapterTimeEnd>\n";
        output += "      <ChapterDisplay>\n";
        output = output + "        <ChapterString>" + &chapter.title + "</ChapterString>\n";
        output += "      </ChapterDisplay>\n";
        output += "    </ChapterAtom>\n";
      }

      output += "  </EditionEntry>\n</Chapters>\n";

      let mut file = File::create(file.path+".xml").unwrap();
      writeln!(&mut file, "{output}").unwrap();
    }
  }
  if ffmetadata {
    for file in files {
      let mut output: String = ";FFMETADATA1\n".to_string();
  
      for chapter in file.chapters {
        output += "\n[CHAPTER]\nTIMEBASE=1/1000";
        output = output + "\nSTART=" + str_to_time(chapter.start.clone()).to_string().as_str();
        output = output + "\nEND=" + str_to_time(chapter.end.clone().to_string()).as_str();
        output = output + "\ntitle=" + &chapter.title;
      }
  
      let mut file = File::create(file.path+".ff").unwrap();
      writeln!(&mut file, "{output}").unwrap();
    }
  }
}
