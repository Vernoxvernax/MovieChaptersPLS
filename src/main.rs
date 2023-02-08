use std::path::Path;
use clap::{Arg, Command, ArgAction};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::time::Duration;
use std::fs::File;
use std::io::Write;
use ffmpeg_next;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
struct XMLChapter {
    title: Option<String>,
    start: Option<String>,
    end: Option<String>,
    lang: Option<String>
}


#[derive(Debug, Clone)]
struct M2ts {
    path: String,
    chapters: Vec<XMLChapter>
}


fn main () {
    let matches = Command::new("chapterpls")
        .about("easily read xml chapters and automatically output ffmetadata files")
        .version(VERSION)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Vernox Vernax")
        .subcommand(
            Command::new("run")
            .short_flag('c')
            .long_flag("check")
            .about("Convert xml file to ffmetadata")
            .arg(
                Arg::new("chapters.xml")
                .long("xml")
                .short('x')
                .help("xml file from mkvmerge")
                .required(true)
                .action(ArgAction::Set)
                .num_args(1)
            )
            .arg(
                Arg::new("video files")
                .long("files")
                .short('f')
                .help("A list of files that the chapters are spread across. Make sure to list them in the correct order.")
                .required(true)
                .action(ArgAction::Set)
                .num_args(1..)
            )
        )
    .get_matches();
    match matches.subcommand() {
        Some(("run", matches)) => {
            let file = matches.get_one::<String>("chapters.xml").unwrap();
            if ! file.ends_with(".xml") {
                println!("The chapter file must be of the xml format.");
                return;
            }
            let chapters_att = get_chapters(file);
            let chapters: Vec<XMLChapter>;
            match chapters_att {
                Ok(payload) => {
                    chapters = payload;
                },
                Err(_) => {
                    return;
                }
            };
            let chapters = split_chapters(
                chapters,
                matches.get_many::<String>("video files").unwrap().map(|a| a.to_string()).collect::<Vec<_>>()
            );
            if chapters.is_empty() {
                println!("No chapters found to write.");
                return;
            }
            write_ffmetadata(chapters);
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
            if chapter.start.is_some() {  
                output = output + "\nSTART=" + format!("{}", str_to_time(chapter.start.clone().unwrap())).as_str();
            } else {
                output = output + "\nSTART=" + format!("{}", str_to_time(chapter.end.clone().unwrap())).as_str();
            };
            if chapter.end.is_some() {  
                output = output + "\nEND=" + format!("{}", str_to_time(chapter.end.unwrap())).as_str();
            } else {
                output = output + "\nEND=" + format!("{}", str_to_time(chapter.start.unwrap())).as_str();
            };
            if chapter.title.is_some() {
                output = output + "\ntitle=" + &chapter.title.unwrap();
            }
        }
        let mut file = File::create(file.path+".ff").unwrap();
        writeln!(&mut file, "{}", output).unwrap();
    }
}


fn split_chapters(payload: Vec<XMLChapter>, files: Vec<String>) -> Vec<M2ts> {
    ffmpeg_next::init().unwrap();
    let mut payload_mut = payload;
    let mut m2tss = vec![];
    for file in files {
        let mut seconds: u64 = 0;
        match ffmpeg_next::format::input(&file) {
            Ok(context) => {
                let duration =  context.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE);
                let second_str = duration.to_string();
                if second_str.contains(".") {
                    let split_str: Vec<String> = second_str.split(".").map(|s| s.to_string()).collect();
                    seconds = split_str.get(0).unwrap().parse::<u64>().unwrap();
                } else {
                    seconds = format!("{}", duration).parse::<u64>().unwrap();
                }
            },
            Err(error) => println!("error: {}", error),
        };

        let search: usize = closest_chapter(payload_mut.clone(), seconds as f64);
        let last_chapter = payload_mut.get(search).unwrap().clone();
        let mut chapters: Vec<XMLChapter> = vec![];
        for _index in 0..search+1 {
            chapters.append(&mut vec![payload_mut.get(0).unwrap().clone()]);
            payload_mut.remove(0);
        };
        payload_mut = {
            let mut ram = vec![];
            for index in 0..payload_mut.len() {
                let chapter = payload_mut.get(index).unwrap();
                let new_ch = substract_time(chapter.clone(), last_chapter.clone());
                ram.append(&mut vec![new_ch]);
            };
            ram
        };
        m2tss.append(&mut vec![M2ts {
            path: file,
            chapters
        }]);
    }
    m2tss
}


fn closest_chapter(payload: Vec<XMLChapter>, seconds: f64) -> usize {
    let mut last: usize = 0;
    for (index, chapter) in payload.iter().enumerate() {
        last = index;
        let ch = floating(&chapter.start.clone().unwrap()).floor();
        if seconds == ch {
            return index;
        }
    }
    last
}


fn floating(time: &String) -> f64 {
    let start: Vec<&str> = time.split(":").collect();
    let hour = start.get(0).unwrap().parse::<f64>().unwrap();
    let minute = start.get(1).unwrap().parse::<f64>().unwrap();
    let second = start.get(2).unwrap().parse::<f64>().unwrap();
    hour*60.0*60.0+minute*60.0+second
}


fn convert_to_seconds(time: &String, time2: &String) -> (u64, String) {
    let difference = (floating(time)-floating(time2)).to_string();
    let difference_str: Vec<&str> = difference.split(".").collect();
    let mut nanos = format!("{}", difference_str.get(1).unwrap());
    if nanos.len() > 9 {
        for _x in 0..nanos.len()-9 {
            nanos.pop();
        }
    }
    if nanos.len() < 9 {
        for _x in 0..9-nanos.len() {
            nanos.push_str("0")
        }
    }
    (difference_str.get(0).unwrap().parse::<u64>().unwrap(), nanos)
}


fn substract_time(ch1: XMLChapter, ch2: XMLChapter) -> XMLChapter {
    let start_time_str = convert_to_seconds(ch1.start.as_ref().unwrap(), ch2.start.as_ref().unwrap());
    let obj = Duration::new(start_time_str.0, start_time_str.1.parse::<u32>().unwrap());
    let start_time = Some(
        format!("{:02}:{:02}:{:02}.{:02}",
        (((obj.as_secs() / 60) / 60)),
        ((obj.as_secs() / 60) %60),
        obj.as_secs() % 60,
        start_time_str.1));
    let new_end = if ch1.end.is_some() {
        let end_time_str = convert_to_seconds(ch1.end.as_ref().unwrap(), ch2.end.as_ref().unwrap());
        let obj = Duration::new(end_time_str.0, end_time_str.1.parse::<u32>().unwrap());
        Some(format!("{:02}:{:02}:{:02}", (((obj.as_secs() / 60) / 60)), ((obj.as_secs() / 60) %60), obj.as_secs() % 60))
    } else {
        None
    };
    XMLChapter {
        title: ch1.title,
        start: start_time,
        end: new_end,
        lang: ch1.lang
    }
}


fn get_chapters(path: &String) -> Result<Vec<XMLChapter>, ()> {
    let file = Path::new(&path);
    let mut reader_att = Reader::from_file(file).unwrap();
    let reader = reader_att.trim_text(true);
    let mut buf = Vec::new();
    let mut chapters: Vec<XMLChapter> = vec![];
    let mut title: Option<String> = None;
    let mut start: Option<String> = None;
    let mut end: Option<String> = None;
    let mut lang: Option<String> = None;
    let mut curr: &str = "";
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"ChapterString" => {
                        curr = "1";
                    }
                    b"ChapterTimeStart" => {
                        curr = "2";
                    },
                    b"ChapterTimeEnd" => {
                        curr = "3";
                    }
                    b"ChapterLanguage" => {
                        curr = "4";
                    },
                    _ => ()
                }
            },
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"ChapterAtom" => {
                        chapters.append(&mut vec![XMLChapter {
                            title: title.clone(),
                            start: start.clone(),
                            end: end.clone(),
                            lang: lang.clone()
                        }]);
                        (title, start, end, lang) = (None, None, None, None);
                    }
                    _ => (),
                }
            }
            Ok(Event::Text(e)) => {
                match curr {
                    "1" => {
                        title = Some(e.unescape().unwrap().into_owned().to_string());
                        curr = "";
                    },
                    "2" => {
                        start = Some(e.unescape().unwrap().into_owned().to_string());
                        curr = "";
                    },
                    "3" => {
                        end = Some(e.unescape().unwrap().into_owned().to_string());
                        curr = "";
                    },
                    "4" => {
                        lang = Some(e.unescape().unwrap().into_owned().to_string());
                        curr = "";
                    },
                    _ => (),
                }
            },
            _ => (),
        }
        buf.clear();
    }
    Ok(chapters)
}
