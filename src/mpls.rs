use std::fs;
use std::io::{Read};
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

pub fn serialize(path: &String, merge: Vec<&String>) -> Vec<M2ts>
{
  let pathed_file = Path::new(&path);
  let mut file = fs::File::open(pathed_file).unwrap();

  let mut buf = vec![0u8; 4*5];
  file.read_exact(&mut buf).unwrap();
  
  // Reading Header from 0
  let type_ind1 = String::from_utf8_lossy(&buf[0..4]).to_string();
  let type_ind2 = String::from_utf8_lossy(&buf[4..8]).to_string();
  if type_ind1 != *"MPLS" || ! "0100 0200 0300".contains(type_ind2.as_str())
  {
    eprintln!("This is not a valid mpls playlist file.")
  }

  let mut bc = &buf[8..];
  let playliststartaddress = bc.read_u32::<BigEndian>().unwrap();
  let playlistmarkstartaddress = bc.read_u32::<BigEndian>().unwrap();
  bc.read_u32::<BigEndian>().unwrap();

  // Creating new buffer so that we can skip AppInfoPlayList
  // Buffer starts at PlayList
  let mut buf2 = vec![];
  file.rewind().unwrap();
  file.read_to_end(&mut buf2).unwrap();
  let mut bc2 = &buf2[(playliststartaddress as usize)..];
  
  bc2.read_u32::<BigEndian>().unwrap(); // Length
  bc2.read_u16::<BigEndian>().unwrap(); // reserved
  
  let numberofplayitems = bc2.read_u16::<BigEndian>().unwrap();
  bc2.read_u16::<BigEndian>().unwrap(); // NumberOfSubPaths

  let mut media: Vec<(u16, String, u32, u32)> = vec![];

  // Reading all PlayItems
  for _n in 0..numberofplayitems
  {
    let mut length = bc2.read_u16::<BigEndian>().unwrap();

    let clipinformationfilename = bc2.read_int::<BigEndian>(5).unwrap();
    let file_id = String::from_utf8_lossy(&clipinformationfilename.to_be_bytes()).to_string();
    let file_id = file_id.trim_start_matches("\0\0\0").parse::<u16>().unwrap();

    let clipcodecidentifier = bc2.read_int::<BigEndian>(4).unwrap();
    let codec = String::from_utf8_lossy(&clipcodecidentifier.to_be_bytes()).to_string();
    let codec_trim = codec.trim_start_matches('\0').to_string();

    bc2.read_u16::<BigEndian>().unwrap(); // reserved, IsMultiAngle, ConnectionCondition
    bc2.read_u8().unwrap(); // RefToSTCID
    let intime = bc2.read_u32::<BigEndian>().unwrap();
    let outtime = bc2.read_u32::<BigEndian>().unwrap();

    media.append(&mut vec![(file_id, codec_trim, intime, outtime)]);

    length -= 5 + 4 + 11;
    loop
    {
      if length > 8
      {
        length -= 8;
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
  file.rewind().unwrap();
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
      println!("[WARN] No chapters have been found \"{:05}.{}\".", m.0, m.1.to_lowercase());
      continue;
    }

    let mut offset = play_item_marks.get(0).unwrap().2;
    if m.2 < offset
    {
      offset = m.2
    }

    let mut chapters: Vec<XMLChapter> = vec![];
    let mut last_mark: String = String::new(); // So that we can set correct chapter start and end values
    let mut last_duration: Duration = Duration::new(0, 0);

    for n in 0..play_item_marks.len()
    {
      let mark = play_item_marks.get(n).unwrap();
      let time_mark = (mark.2 - offset) as f32 / 45000.0;
      let time_mark_str = if time_mark != 0.0
      {
        convert_to_seconds(&time_mark)
      }
      else
      {
        (0, "0".to_string())
      };

      let obj = Duration::new(time_mark_str.0, time_mark_str.1.parse::<u32>().unwrap());
      let time = 
        format!("{:02}:{:02}:{:02}.{:02}",
        ((obj.as_secs() / 60) / 60),
        ((obj.as_secs() / 60) % 60),
        obj.as_secs() % 60,
      time_mark_str.1);

      if n != 0
      {
        chapters.append(&mut vec![XMLChapter {
          title: format!("Chapter {n}"),
          start: last_mark.trim().to_string().clone(),
          end: time.clone(),
          start_duration: last_duration,
          end_duration: obj
        }]);
      }
      
      if n == play_item_marks.len() - 1
      {
        let end_mark = (m.3 - offset) as f32 / 45000.0;
        let end_time_mark_str = convert_to_seconds(&end_mark);
        let end_duration = Duration::new(end_time_mark_str.0, end_time_mark_str.1.parse::<u32>().unwrap());
        let end_time = 
          format!("{:02}:{:02}:{:02}.{:02}",
          ((end_duration.as_secs() / 60) / 60),
          ((end_duration.as_secs() / 60) %60),
          end_duration.as_secs() % 60,
        end_time_mark_str.1);

        chapters.append(&mut vec![XMLChapter {
          title: format!("Chapter {}", n+1),
          start: time.clone(),
          end: end_time.clone(),
          start_duration: obj,
          end_duration
        }]);
      }
      last_mark = time;
      last_duration = obj;
    }

    if ((merge.is_empty() && chapters.len() != 1) || (!merge.is_empty() && !chapters.is_empty())) && chapters.get(0).unwrap().start != "00:00:00.00"
    {
      m2ts.append(&mut vec![M2ts{
        id: m.0,
        path: format!("{:05}.{}", m.0, m.1.to_lowercase()),
        chapters
      }]);
    }
    else
    {
      println!("[WARN] Only one chapter has been found for \"{:05}.{}\" (skipping).", m.0, m.1.to_lowercase());
    }
  }

  if merge.is_empty()
  {
    m2ts
  }
  else
  {
    let mut merged: Vec<M2ts> = vec![];
    let mut chapters: Vec<XMLChapter> = vec![];
    let mut record: bool = false;
    let mut last_end: Duration = Duration::new(0, 0);
    for file in m2ts
    {

      if file.id == merge.first().unwrap().parse::<u16>().unwrap()
      {
        record = true;
        chapters.append(&mut file.chapters.clone());
        last_end = chapters.last().unwrap().end_duration;
        continue;
      }

      if record
      {
        let mut end = Duration::new(0, 0);
        for chapter in file.chapters
        {
          let n = chapters.len();
          let start = chapter.start_duration + last_end;
          end = chapter.end_duration + last_end;

          let nanos = get_nanos(start);
          let start_time = 
            format!("{:02}:{:02}:{:02}.{}",
            ((start.as_secs() / 60) / 60),
            ((start.as_secs() / 60) % 60),
            start.as_secs() % 60,
          nanos);

          let nanos = get_nanos(end);
          let end_time = 
            format!("{:02}:{:02}:{:02}.{:02}",
            ((end.as_secs() / 60) / 60),
            ((end.as_secs() / 60) % 60),
            end.as_secs() % 60,
          nanos);
          
          chapters.append(&mut vec![XMLChapter {
            title: format!("Chapter {}", n+1),
            start: start_time,
            end: end_time,
            start_duration: start,
            end_duration: end
          }]);
        }
        last_end = end;
      }

      if file.id == merge.last().unwrap().parse::<u16>().unwrap()
      {
        merged.append(&mut vec![M2ts{
          id: file.id,
          path: file.path,
          chapters
        }]);
        break;
      }
    }
    merged
  }
}

fn get_nanos(time: Duration) -> String
{
  let strd = time.as_secs_f64().to_string();
  let iter: Vec<&str> = strd.split('.').collect();
  iter.last().unwrap().to_string()
}

fn convert_to_seconds(time: &f32) -> (u64, String)
{
  let time_str = time.to_string();
  let time_split: Vec<&str> = time_str.split('.').collect();
  let nano = if let Some(nanos) = time_split.get(1)
  {
    let mut nanos = nanos.to_string();
    if nanos.len() > 9 {
      for _x in 0..nanos.len()-9 {
        nanos.pop();
      }
    }
    if nanos.len() < 9 {
      for _x in 0..9-nanos.len() {
        nanos.push('0')
      }
    }
    nanos
  }
  else
  {
    0.to_string()
  };
  
  (time_split.first().unwrap().parse::<u64>().unwrap(), nano)
}
