use std::fs;
use std::io::{SeekFrom, Read};
use std::io::Seek;
use std::path::Path;
use std::time::Duration;
use byteorder::{ReadBytesExt, BigEndian};

use crate::M2ts;
use crate::XMLChapter;

/*
  If you're wondering wtf is going on in the code below,
  check this website out, it helped me a lot:
  https://en.wikibooks.org/wiki/User:Bdinfo/mpls
*/

pub fn serialize(path: &String) -> Vec<M2ts>
{
  let pathed_file = Path::new(&path);
  let mut file = fs::File::open(pathed_file).unwrap();

  let mut buf = vec![0u8; 4*5];
  file.read_exact(&mut buf).unwrap();
  
  // Reading Header from 0
  let type_ind1 = String::from_utf8_lossy(&buf[0..4]).to_string();
  let type_ind2 = String::from_utf8_lossy(&buf[4..8]).to_string();
  if type_ind1 != String::from("MPLS") || ! "0100 0200 0300".contains(type_ind2.as_str()) {
    eprintln!("This is not a valid mpls playlist file.")
  }

  let mut bc = &buf[8..];
  let playliststartaddress = bc.read_u32::<BigEndian>().unwrap();
  let playlistmarkstartaddress = bc.read_u32::<BigEndian>().unwrap();
  bc.read_u32::<BigEndian>().unwrap();

  // Creating new buffer so that we can skip AppInfoPlayList
  // Buffer starts at PlayList
  let mut buf2 = vec![];
  file.seek(SeekFrom::Start(0)).unwrap();
  file.read_to_end(&mut buf2).unwrap();
  let mut bc2 = &buf2[(playliststartaddress as usize)..];
  
  bc2.read_u32::<BigEndian>().unwrap(); // Length
  bc2.read_u16::<BigEndian>().unwrap(); // reserved
  
  let numberofplayitems = bc2.read_u16::<BigEndian>().unwrap();
  bc2.read_u16::<BigEndian>().unwrap(); // NumberOfSubPaths

  let mut media: Vec<(u16, String, u32, u32)> = vec![];

  // Reading all PlayItems
  for _n in 0..numberofplayitems {
    let mut length = bc2.read_u16::<BigEndian>().unwrap();

    let clipinformationfilename = bc2.read_int::<BigEndian>(5).unwrap();
    let file_id = String::from_utf8_lossy(&clipinformationfilename.to_be_bytes()).to_string();
    let file_id = file_id.trim_start_matches("\0\0\0").parse::<u16>().unwrap();

    let clipcodecidentifier = bc2.read_int::<BigEndian>(4).unwrap();
    let codec = String::from_utf8_lossy(&clipcodecidentifier.to_be_bytes()).to_string();
    let codec_trim = codec.trim_start_matches("\0").to_string();

    bc2.read_u16::<BigEndian>().unwrap(); // reserved, IsMultiAngle, ConnectionCondition
    bc2.read_u8().unwrap(); // RefToSTCID
    let intime = bc2.read_u32::<BigEndian>().unwrap();
    let outtime = bc2.read_u32::<BigEndian>().unwrap();

    media.append(&mut vec![(file_id, codec_trim, intime, outtime)]);

    length = length - (5 + 4 + 11);
    loop {
      if length > 8
      {
        length = length - 8;
        bc2.read_int::<BigEndian>(8).unwrap();
      }
      else
      {
        bc2.read_int::<BigEndian>(length as usize).unwrap();
        break;
      }
    }
  }

  // Creating new buffer so that we can fast-forwarding to PlayListMark (chapters)
  let mut marks: Vec<(u8, u16, u32)> = vec![];
  let mut buf3 = vec![];
  file.seek(SeekFrom::Start(0)).unwrap();
  file.read_to_end(&mut buf3).unwrap();
  let mut bc3 = &buf3[(playlistmarkstartaddress as usize)..];
  bc3.read_u32::<BigEndian>().unwrap(); // Length
  let amount = bc3.read_u16::<BigEndian>().unwrap();
  for _n in 0..amount
  {
    bc3.read_u8().unwrap(); // reserved
    let marktype = bc3.read_u8().unwrap();
    let reftoplayitemid = bc3.read_u16::<BigEndian>().unwrap();
    let marktimestamp = bc3.read_u32::<BigEndian>().unwrap();
    bc3.read_u16::<BigEndian>().unwrap(); // EntryESPID
    bc3.read_u32::<BigEndian>().unwrap(); // Duration

    marks.append(&mut vec![(marktype, reftoplayitemid, marktimestamp)]);
  }


  // Processing the data we've found using structs and vectors.
  let mut m2ts: Vec<M2ts> = vec![];
  for (nr, m) in media.iter().enumerate()
  {
    let play_item_marks = marks.clone().into_iter()
    .filter(|x| x.0 == 1 && x.1 as usize == nr)
    .collect::<Vec<_>>();
    if play_item_marks.is_empty()
    {
      println!("No chapters have been found \"{:05}.{}\".", m.0, m.1.to_lowercase());
      continue;
    }

    let mut offset = play_item_marks.get(0).unwrap().2;
    if m.2 < offset
    {
      offset = m.2
    }

    let mut chapters: Vec<XMLChapter> = vec![];
    for (n, mark) in play_item_marks.iter().enumerate()
    {
      let start = (mark.2 - offset) as f32 / 45000.0;
      let start_time_str = if start != 0.0
      {
        convert_to_seconds(&start)
      }
      else
      {
        (0, "0".to_string())
      };
      let obj = Duration::new(start_time_str.0, start_time_str.1.parse::<u32>().unwrap());
      let start_time = 
        format!("{:02}:{:02}:{:02}.{:02}",
        (((obj.as_secs() / 60) / 60)),
        ((obj.as_secs() / 60) %60),
        obj.as_secs() % 60,
      start_time_str.1);

      chapters.append(&mut vec![XMLChapter{
        title: format!("Chapter {}", n+1),
        start: start_time
      }])
    }
    if chapters.len() != 1 && chapters.get(0).unwrap().start != "00:00:00.00"
    {
      m2ts.append(&mut vec![M2ts{
        path: format!("{:05}.{}", m.0, m.1.to_lowercase()),
        chapters
      }])
    }
    else
    {
      println!("No chapters have been found \"{:05}.{}\".", m.0, m.1.to_lowercase());
    }
  }
  m2ts
}

fn convert_to_seconds(time: &f32) -> (u64, String) {
  let time_str = time.to_string();
  let time_split: Vec<&str> = time_str.split(".").collect();
  let mut nanos = format!("{}", time_split.get(1).unwrap());
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
  (time_split.get(0).unwrap().parse::<u64>().unwrap(), nanos)
}
