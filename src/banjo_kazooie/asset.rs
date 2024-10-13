use std::collections::btree_map::{IterMut, Keys};
use std::collections::HashMap;
use std::{default, fmt};
use std::fs::{self, File, DirBuilder};
use std::io::{Write, BufWriter};
use std::path::{Display, Path};
use yaml_rust::{Yaml, YamlLoader};
use png;

pub fn from_seg_indx_and_bytes(segment :usize, i :usize, in_bytes: &[u8]) -> Box<dyn Asset>{
    return match segment{
        0 => Box::new(Animation::from_bytes(in_bytes)),
        1 | 3 => match in_bytes { //models and sprites
            [0x00, 0x00, 0x00, 0x0B, ..] => Box::new(Model::from_bytes(in_bytes)),
            _ => Box::new(Sprite::from_bytes(in_bytes)),
        }, //sprites
        2 => Box::new(LevelSetup::from_bytes(in_bytes, i)),
        4 => match in_bytes { //Dialog, GruntyQuestions, QuizQuestions, DemoButtonFiles
                [0x01, 0x01, 0x02, 0x05, 0x00, ..] => Box::new(QuizQuestion::from_bytes(in_bytes)),
                [0x01, 0x03, 0x00, 0x05, 0x00, ..] => Box::new(GruntyQuestion::from_bytes(in_bytes)),
                [0x01, 0x03, 0x00,..] => Box::new(Dialog::from_bytes(in_bytes)),
                _ => Box::new(DemoButtonFile::from_bytes(in_bytes)),
            },
        5 => Box::new(Model::from_bytes(in_bytes)),
        6 => Box::new(MidiSeqFile::from_bytes(in_bytes)),
        _ => Box::new(Binary::from_bytes(in_bytes)),
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ImgFmt{
    CI4,
    CI8,
    I4,
    I8,
    RGBA16,
    RGBA32,
    IA4,
    IA8,
    Unknown(u16),
}

pub enum AssetType{
    Animation,
    Binary,
    DemoInput,
    Dialog,
    GruntyQuestion,
    LevelSetup,
    Midi,
    Model,
    QuizQuestion,
    Sprite(ImgFmt),
}

pub struct Binary{
    bytes: Vec<u8>,
}

impl Binary{
    pub fn from_bytes(in_bytes: &[u8])->Binary{
        Binary{bytes: in_bytes.to_vec()}
    }

    pub fn read(path: &Path) -> Binary{
        Binary{bytes: fs::read(path).unwrap()}
    }
}

impl Asset for Binary{
    fn to_bytes(&self)->Vec<u8>{
        return self.bytes.clone();
    }

    fn get_type(&self)->AssetType{
        return AssetType::Binary;
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();
    }
}

#[derive(Clone)]
struct BKString{
    cmd: u8,
    string: Vec<u8>,
}

impl BKString{
    pub fn from_yaml(yaml: &Yaml) -> BKString{
        let cmd = yaml["cmd"].as_i64().unwrap() as u8;
        let string = string_to_vecu8(&yaml["string"].as_str().unwrap());            
        
        BKString{cmd : cmd, string: string}
    }
}

pub struct Dialog{
    bottom: Vec<BKString>,
    top: Vec<BKString>,
}

impl Dialog{
    pub fn from_bytes(in_bytes: &[u8])->Dialog{
        let mut offset : usize = 3;
            
        let mut bottom = Vec::new();
        let bottom_size : u8 = in_bytes[offset];
        offset += 1;
        let mut i = 0;
        for i in 0..bottom_size{
            let cmd : u8 = in_bytes[offset];
            let str_size : u8 = in_bytes[offset + 1];
            let i_string = BKString{cmd : cmd, string : in_bytes[offset + 2 .. offset + 2 + str_size as usize].to_vec()};
            bottom.push(i_string);
            offset += 2 + str_size as usize;
        }

        let mut top = Vec::new();
        let top_size : u8 = in_bytes[offset];
        offset += 1;
        let mut i = 0;
        for i in 0..top_size{
            let cmd : u8 = in_bytes[offset];
            let str_size : u8 = in_bytes[offset + 1];
            let i_string = BKString{cmd : cmd, string : in_bytes[offset + 2 .. offset + 2 + str_size as usize].to_vec()};
            top.push(i_string);
            offset += 2 + str_size as usize;
        }

        return Dialog{ bottom: bottom, top: top,};
    }

    pub fn read(path: &Path) -> Dialog{
        let doc = &YamlLoader::load_from_str(&fs::read_to_string(path).expect("could not open yaml")).unwrap()[0];
        let doc_type = doc["type"].as_str().unwrap();
        assert_eq!(doc_type, "Dialog");
        let bottom_obj = doc["bottom"].as_vec().unwrap();
        let bottom : Vec<BKString> = bottom_obj.iter()
            .map(|y|{BKString::from_yaml(y)})
            .collect();

        let top_obj = doc["top"].as_vec().unwrap();
        let top : Vec<BKString> = top_obj.iter()
            .map(|y|{BKString::from_yaml(y)})
            .collect();

        Dialog{bottom: bottom, top: top}
    }
}

impl Asset for Dialog{
    fn to_bytes(&self)->Vec<u8>{
        let mut out :Vec<u8> = vec![0x01, 0x03, 0x00];
        out.push(self.bottom.len() as u8);
        for text in self.bottom.iter(){
            out.push(text.cmd);
            out.push(text.string.len() as u8);
            out.append(&mut text.string.clone());
        }
        out.push(self.top.len() as u8);
        for text in self.top.iter(){
            out.push(text.cmd);
            out.push(text.string.len() as u8);
            out.append(&mut text.string.clone());
        }
        return out;
    }

    fn get_type(&self)->AssetType{
        return AssetType::Dialog;
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        
        writeln!(bin_file, "type: Dialog").unwrap();
        writeln!(bin_file, "bottom:").unwrap();
        for text in self.bottom.iter(){
            writeln!(bin_file,"  - {{ cmd: 0x{:02X}, string: \"{}\"}}", text.cmd, vecu8_to_string(&text.string)).unwrap()
        }
        writeln!(bin_file, "top:").unwrap();
        for text in self.top.iter(){
            writeln!(bin_file,"  - {{ cmd: 0x{:02X}, string: \"{}\"}}", text.cmd, vecu8_to_string(&text.string)).unwrap()
        }
    }
}

pub struct QuizQuestion{
    question: Vec<BKString>,
    options: [BKString; 3],
}

impl QuizQuestion{
    pub fn from_bytes(in_bytes: &[u8])->QuizQuestion{
        let mut texts = Vec::new();
        let mut str_cnt = in_bytes[5];
        let mut offset : usize = 6;
        for _i in 0..str_cnt{
            let cmd : u8 = in_bytes[offset];
            let str_size : u8 = in_bytes[offset + 1];
            let i_string = BKString{cmd : cmd, string : in_bytes[offset + 2 .. offset + 2 + str_size as usize].to_vec()};
            texts.push(i_string);
            offset += 2 + str_size as usize;
        }
        let (q_text, o_text) = texts.split_at(texts.len() - 3); 

        let options : [BKString; 3] = [o_text[0].clone(), o_text[1].clone(), o_text[2].clone()];
        return QuizQuestion{ question: q_text.to_vec(), options: options};
    }

    pub fn read(path: &Path) -> QuizQuestion{
        let doc = &YamlLoader::load_from_str(&fs::read_to_string(path).expect("could not open yaml")).unwrap()[0];
        let doc_type = doc["type"].as_str().unwrap();
        assert_eq!(doc_type, "QuizQuestion");
        let q_obj = doc["question"].as_vec().unwrap();
        let q : Vec<BKString> = q_obj.iter()
            .map(|y|{BKString::from_yaml(y)})
            .collect();

        let a_obj = doc["options"].as_vec().unwrap();
        let a : Vec<BKString> = a_obj.iter()
            .map(|y|{BKString::from_yaml(y)})
            .collect();

        let options : [BKString; 3] = [a[0].clone(), a[1].clone(), a[2].clone()];

        QuizQuestion{question: q, options: options}
    }
}

impl Asset for QuizQuestion{
    fn to_bytes(&self)->Vec<u8>{
        let mut out :Vec<u8> = vec![0x01, 0x01, 0x02, 0x05, 0x00];
        out.push((self.question.len() + self.options.len()) as u8);
        for text in self.question.iter(){
            out.push(text.cmd);
            out.push(text.string.len() as u8);
            out.append(&mut text.string.clone());
        }
        for text in self.options.iter(){
            out.push(text.cmd);
            out.push(text.string.len() as u8);
            out.append(&mut text.string.clone());
        }
        return out;
    }
    
    fn get_type(&self)->AssetType{
        return AssetType::QuizQuestion
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        
        writeln!(bin_file, "type: QuizQuestion").unwrap();
        writeln!(bin_file, "question:").unwrap();
        for text in self.question.iter(){
            writeln!(bin_file,"  - {{ cmd: 0x{:02X}, string: \"{}\"}}", text.cmd, vecu8_to_string(&text.string)).unwrap()
        }
        writeln!(bin_file, "options:").unwrap();
        for text in self.options.iter(){
            writeln!(bin_file,"  - {{ cmd: 0x{:02X}, string: \"{}\"}}", text.cmd, vecu8_to_string(&text.string)).unwrap()
        }
    }
}

pub struct GruntyQuestion{
    question: Vec<BKString>,
    options: [BKString; 3],
}

impl GruntyQuestion{
    pub fn from_bytes(in_bytes: &[u8])->GruntyQuestion{
        let mut texts = Vec::new();
        let mut str_cnt = in_bytes[5];
        let mut offset : usize = 6;
        for _i in 0..str_cnt{
            let cmd : u8 = in_bytes[offset];
            let str_size : u8 = in_bytes[offset + 1];
            let i_string = BKString{cmd : cmd, string : in_bytes[offset + 2 .. offset + 2 + str_size as usize].to_vec()};
            texts.push(i_string);
            offset += 2 + str_size as usize;
        }
        let (q_text, o_text) = texts.split_at(texts.len() - 3); 

        let options : [BKString; 3] = [o_text[0].clone(), o_text[1].clone(), o_text[2].clone()];
        return GruntyQuestion{ question: q_text.to_vec(), options: options};
    }

    pub fn read(path: &Path) -> GruntyQuestion{
        let doc = &YamlLoader::load_from_str(&fs::read_to_string(path).expect("could not open yaml")).unwrap()[0];
        let doc_type = doc["type"].as_str().unwrap();
        assert_eq!(doc_type, "GruntyQuestion");
        let q_obj = doc["question"].as_vec().unwrap();
        let q : Vec<BKString> = q_obj.iter()
            .map(|y|{BKString::from_yaml(y)})
            .collect();

        let a_obj = doc["options"].as_vec().unwrap();
        let a : Vec<BKString> = a_obj.iter()
            .map(|y|{BKString::from_yaml(y)})
            .collect();

        let options : [BKString; 3] = [a[0].clone(), a[1].clone(), a[2].clone()];

        GruntyQuestion{question: q, options: options}
    }
}

impl Asset for GruntyQuestion{
    fn to_bytes(&self)->Vec<u8>{
        let mut out :Vec<u8> = vec![0x01, 0x03, 0x00, 0x05, 0x00];
        out.push((self.question.len() + self.options.len()) as u8);
        for text in self.question.iter(){
            out.push(text.cmd);
            out.push(text.string.len() as u8);
            out.append(&mut text.string.clone());
        }
        for text in self.options.iter(){
            out.push(text.cmd);
            out.push(text.string.len() as u8);
            out.append(&mut text.string.clone());
        }
        return out;
    }
    
    fn get_type(&self)->AssetType{
        return AssetType::GruntyQuestion
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        
        writeln!(bin_file, "type: GruntyQuestion").unwrap();
        writeln!(bin_file, "question:").unwrap();
        for text in self.question.iter(){
            writeln!(bin_file,"  - {{ cmd: 0x{:02X}, string: \"{}\"}}", text.cmd, vecu8_to_string(&text.string)).unwrap()
        }
        writeln!(bin_file, "options:").unwrap();
        for text in self.options.iter(){
            writeln!(bin_file,"  - {{ cmd: 0x{:02X}, string: \"{}\"}}", text.cmd, vecu8_to_string(&text.string)).unwrap()
        }
    }
}

pub trait Asset {
    fn to_bytes(&self)->Vec<u8>;
    fn get_type(&self)->AssetType;
    fn write(&self, path: &Path);
}

fn string_to_vecu8(string: &str) -> Vec<u8>{
    let mut string = string.as_bytes().to_vec();
    let mut squig_indx : Vec<usize> = string.windows(2)
        .enumerate()
        .filter(|(_, win)|{match win {[0xC3, 0xBD]=> true, _=>false,} })
        .map(|(i, _)|{i})
        .collect();
    squig_indx.reverse();
    for i in squig_indx{
        string[i] = 0xFD;
        string.remove(i+1);
    }
    string.push(0);
    return string
}

fn vecu8_to_string(bytes: &Vec<u8>) -> String{
    let mut out : String = String::new();
    for b in &bytes[..bytes.len() - 1]{
        let ch = *b as char;
        if !ch.is_ascii() || *b < 0x20 {
            out += format!("\\x{:02X}", ch as u8).as_str();
        }
        else{
            out.push(ch);
        }
    }
    return out
}

struct ContInput{
    x: i8,
    y: i8,
    buttons: u16,
    frames: u8,
}

impl ContInput{
    fn to_bytes(&self)->Vec<u8>{
        let b = self.buttons.to_be_bytes();
        return vec![self.x as u8, self.y as u8, b[0], b[1], self.frames, 0x00];
    }

    fn from_yaml(yaml: &Yaml)->ContInput{
        let x = yaml["x"].as_i64().unwrap() as i8;
        let y = yaml["y"].as_i64().unwrap() as i8;
        let buttons = yaml["buttons"].as_i64().unwrap() as u16;
        let frames = yaml["frames"].as_i64().unwrap() as u8;
        return ContInput{x: x, y: y, buttons: buttons, frames: frames}
    }
}

pub struct DemoButtonFile{
    inputs: Vec<ContInput>,
    frame1_flag: u8,
}

impl DemoButtonFile{
    pub fn from_bytes(in_bytes: &[u8])->DemoButtonFile{
        if in_bytes.len() < 4 { return DemoButtonFile{inputs: Vec::new(), frame1_flag: 0}}
        let expect_len : usize =  u32::from_be_bytes(in_bytes[..4].try_into().unwrap()) as usize;
        let f1f = in_bytes[9];
        let inputs : Vec<ContInput> = in_bytes[4..].chunks_exact(6)
            .map(|a|{
                ContInput{
                    x : a[0] as i8, 
                    y : a[1] as i8,
                    buttons : u16::from_be_bytes([a[2], a[3]]),
                    frames : a[4],
                }
            })
            .collect();
        assert_eq!(expect_len, inputs.len()*6);
        DemoButtonFile{inputs: inputs, frame1_flag: f1f}
    }

    pub fn read(path: &Path) -> DemoButtonFile{
        let doc = &YamlLoader::load_from_str(&fs::read_to_string(path).expect("could not open yaml")).unwrap()[0];
        let doc_type = doc["type"].as_str().unwrap();
        let f1f = doc["flag"].as_i64().unwrap() as u8;
        assert_eq!(doc_type, "DemoInput");
        
        let inputs_yaml = doc["inputs"].as_vec().unwrap();
        let mut inputs : Vec<ContInput> = inputs_yaml.iter().map(|y|{
            ContInput::from_yaml(y)
        })
        .collect();
        return DemoButtonFile{inputs:inputs, frame1_flag: f1f}
    }
}

impl Asset for DemoButtonFile{
    fn to_bytes(&self)->Vec<u8>{
        if self.inputs.is_empty() { return Vec::new(); }

        let mut output : Vec<u8> = (6*self.inputs.len() as u32).to_be_bytes().to_vec();
        let mut input_bytes : Vec<u8> = self.inputs.iter().map(|i|{
            i.to_bytes()
        })
        .flatten()
        .collect();
        input_bytes[5] = self.frame1_flag;
        output.append(&mut input_bytes);
        return output;
    }

    fn get_type(&self)->AssetType{
        return AssetType::DemoInput;
    }

    fn write(&self, path: &Path){
        let mut demo_file = File::create(path).unwrap();
        writeln!(demo_file, "type: DemoInput").unwrap();
        writeln!(demo_file, "flag: 0x{:02X}", self.frame1_flag).unwrap();
        if(self.inputs.len() == 0){
            writeln!(demo_file, "inputs: []").unwrap();
            return;
        }
        writeln!(demo_file, "inputs:").unwrap();
        for input in self.inputs.iter(){
            writeln!(demo_file, "  - {{x: {:3}, y: {:3}, buttons: 0x{:04X}, frames: {}}}", input.x, input.y, input.buttons, input.frames).unwrap();
        }
    }
}

/// MidiSeqFile TODO !!!!!!!!!
///     - struct members
///     - from_bytes
///     - read
///     - to_bytes
///     - write

pub struct MidiSeqFile{
    bytes: Vec<u8>,
}

impl MidiSeqFile{
    pub fn from_bytes(in_bytes: &[u8])->MidiSeqFile{
        MidiSeqFile{bytes: in_bytes.to_vec()}
    }

    pub fn read(path: &Path) -> MidiSeqFile{
        MidiSeqFile{bytes: fs::read(path).unwrap()}
    }
}

impl Asset for MidiSeqFile{
    fn to_bytes(&self)->Vec<u8>{
        return self.bytes.clone();
    }

    fn get_type(&self)->AssetType{
        return AssetType::Midi;
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();
    }
}

/// LevelSetup TODO !!!!!!!!!
///     - struct members
///     - from_bytes
///     - read
///     - to_bytes
///     - write

pub struct LevelSetup{
    bytes: Vec<u8>,
}

struct LevelSetupReader<'a> {
    in_bytes: &'a[u8],
    offset: usize
}

impl LevelSetupReader<'_> {
    pub fn new(in_bytes: &[u8]) ->LevelSetupReader {
        LevelSetupReader {
            in_bytes,
            offset: 0
        }
    }

    // This currently only advances the offset until we figure out what a n64 "word" is in Rust terms
    pub fn read_word(&mut self) {
        // the "word" size for n64 is 4
        self.offset += 4;
    }

    // the BK code uses s32 instead of i32
    pub fn read_i32(&mut self) -> i32 {
        let out = i32::from_be_bytes([
            self.in_bytes[self.offset], 
            self.in_bytes[self.offset+1], 
            self.in_bytes[self.offset+2], 
            self.in_bytes[self.offset+3]
        ]);

        self.offset += 4;

        out
    }

    // the BK code uses s16 instead of i16
    pub fn read_i16(&mut self) -> i16 {
        let out = i16::from_be_bytes([self.in_bytes[self.offset], self.in_bytes[self.offset+1]]);
        self.offset += 2;

        out
    }

    // the BK code uses s8 instead of i8
    pub fn read_i8(&mut self) -> i8 {
        let out = i8::from_be_bytes([self.in_bytes[self.offset]]);
        self.offset += 1;

        out
    }
    
    pub fn read_u32(&mut self) -> u32 {
        let out = u32::from_be_bytes([
            self.in_bytes[self.offset], 
            self.in_bytes[self.offset+1], 
            self.in_bytes[self.offset+2], 
            self.in_bytes[self.offset+3]
        ]);

        self.offset += 4;

        out
    }

    pub fn read_u16(&mut self) -> u16 {
        let out = u16::from_be_bytes([self.in_bytes[self.offset], self.in_bytes[self.offset+1]]);
        self.offset += 2;

        out
    }

    pub fn read_u8(&mut self) -> u8 {
        let out = self.in_bytes[self.offset];
        self.offset += 1;

        out
    }
    
    pub fn read_f32(&mut self) -> f32 {
        let out = f32::from_be_bytes([
            self.in_bytes[self.offset], 
            self.in_bytes[self.offset+1], 
            self.in_bytes[self.offset+2], 
            self.in_bytes[self.offset+3]
        ]);

        self.offset += 4;

        out
    }

    pub fn read_n<T>(&mut self, n: usize, reader_fn: impl Fn(&mut LevelSetupReader) -> T) -> Vec<T> {
        let mut out = vec![];
        println!("Read n {n}");
        for _ in 0..n {
            out.push(reader_fn(self));
        }

        out
    }

    pub fn peek_u8(&self) -> u8 {
        self.in_bytes[self.offset]
    }

    pub fn read_u8_n(&mut self, n: usize) -> Vec<u8>{
        return self.read_n(n, |r| r.read_u8());
        let out = self.in_bytes[self.offset..(self.offset + n)].into();
        self.offset += n;

        out
    }

    pub fn read_if_expected<T>(&mut self, expected_value: u8, reader_fn: impl Fn(&mut LevelSetupReader) -> T) -> Option<T> {
        if self.in_bytes[self.offset] == expected_value {
            self.offset += 1;
            Some(reader_fn(self))
        } else {
            None
        }
    }

    pub fn u8s_to_string(in_bytes: &[u8]) -> String {
        in_bytes.iter().map(|x| format!("{:02X}", x)).collect::<Vec<String>>().join(" ")
    }
}

impl fmt::Display for LevelSetupReader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.in_bytes[self.offset..].iter().map(|x| format!("{:02X}", x)).collect::<Vec<String>>().join(" "))
    }
}

impl LevelSetup{
    pub fn from_bytes(in_bytes: &[u8], i: usize)->LevelSetup{
        let map_id_offset = 1820;
        let map_idx = i - map_id_offset;
        
        let maps = LevelSetup::build_map_hash_set();
        let map_name = maps.get(&map_idx).expect(format!("Expected {map_idx} to exist in maps").as_str());
        if map_name.eq(&String::from("SM_SPIRAL_MOUNTAIN")) {
            LevelSetup::parse_file(in_bytes);
        }
        
        LevelSetup{bytes: in_bytes.to_vec()}
    }

    fn parse_file(in_bytes: &[u8]) {
        let mut reader = LevelSetupReader::new(in_bytes);
        loop {
            let cmd = reader.read_u8();
            match cmd {
                0 => {
                    return;
                }
                1 => {
                    // cubeList_fromFile
                    let has_cubes_from = reader.read_u8();
                    let mut cubes_from: [i32; 3] = [0,0,0];
                    if has_cubes_from == 1 {
                        cubes_from = [
                            reader.read_i32(),
                            reader.read_i32(),
                            reader.read_i32(),
                        ];
                    }
                    let cubes_to = [
                        reader.read_i32(),
                        reader.read_i32(),
                        reader.read_i32(),
                    ];

                    for x in cubes_from[0]..=cubes_to[0] {
                        for y in cubes_from[1]..=cubes_to[1] {
                            for z in cubes_from[2]..=cubes_to[2] {
                                // println!("x: {x} | y: {y} | z: {z}");
                                let cubes = LevelSetup::get_cubes_from_reader(&mut reader);
                                //  println!("Found {} cubes in parse_file", cubes.len());
                            }
                        }
                    }
                    
                    // in the c code after the for loops there is:
                    // file_isNextByteExpected(file_ptr, 0);
                    // which, in essence, advances the file_ptr by 1 if the current value is 0
                    reader.read_if_expected(0, |_| 0);
                }, 
                3 => {
                    // ncCameraNodeList_fromFile
                    loop {
                        let cmd = reader.read_u8();
                        if cmd == 0 {
                            break
                        }

                        if cmd != 1 {
                            panic!("Unexpected cmd {cmd}");
                        }
                        
                        let _camera_node_index = reader.read_i16();
                         // seems to be between 0 and 4
                        let camera_node_type = reader.read_if_expected(2, |r| r.read_u8()).unwrap_or(0);
                        
                        match camera_node_type {
                            0 => {
                                break;
                            },
                            2 => {
                                loop {
                                    let cmd = reader.read_u8();
                                    if cmd == 0 {
                                        break;
                                    }

                                    if cmd == 1 {
                                        // f32s file_getNFloats_ifExpected(file_ptr, 1, arg1->position, 3)
                                        println!("cmd = 1: {}", LevelSetupReader::u8s_to_string(&reader.read_u8_n(12)));
                                        // reader.read_u8_n(3 * 4);
                                    }
                                    
                                    else if cmd == 2 {
                                        // f32s file_getNFloats_ifExpected(file_ptr, 2, arg1->rotation, 3)
                                        println!("cmd = 2: {}", LevelSetupReader::u8s_to_string(&reader.read_u8_n(12)));
                                        // reader.read_u8_n(3 * 4);
                                    } else {
                                        todo!("cmd = {cmd}: {}", LevelSetupReader::u8s_to_string(&reader.read_u8_n(50)));
                                    }
                                }
                            },
                            3 => {
                                loop {
                                    let cmd = reader.read_u8();
                                    if cmd == 0 {
                                        break;
                                    }

                                    if cmd == 1 {
                                        // file_getNFloats_ifExpected(file_ptr, 1, arg1->position, 3)
                                        reader.read_n(3, |r| r.read_f32());
                                    } else if cmd == 2 {
                                        // file_getFloat(file_ptr, &arg1->unkC);
                                        reader.read_f32();
                                        // file_getFloat(file_ptr, &arg1->unk10);
                                        reader.read_f32();
                                    } else if cmd == 3 {
                                        // file_getFloat(file_ptr, &arg1->unk14);
                                        reader.read_f32();
                                        // file_getFloat(file_ptr, &arg1->unk18);
                                        reader.read_f32();
                                    } else if cmd == 4 {
                                        // file_getFloat(file_ptr, &arg1->unk1C);
                                        reader.read_f32();
                                        // file_getFloat(file_ptr, &arg1->unk20);
                                        reader.read_f32();
                                    } else if cmd == 5 {
                                        // file_getWord_ifExpected(file_ptr, 5, &arg1->unk30);
                                        reader.read_word();
                                    }
                                }
                            },
                            _ => {
                                todo!("Implement camera_node_type {camera_node_type}");
                            },
                        }
                    }
                }, 
                4 => {
                    // func_80333B78
                    loop {
                        let cmd = reader.read_u8();
                        if cmd == 0 {
                            break
                        }

                        todo!("Implement cmd {cmd}");
                    }
                }, 
                _ => {
                    // no-op
                    todo!("Implement cmd {cmd}?");
                },
            }
        }
    }
    
    fn get_cubes_from_reader(reader: &mut LevelSetupReader) -> Vec<String> {
        let mut out_cubes = vec![];

        loop {
            let cmd = reader.read_u8();
            // println!("Cmd = {cmd}, offset = {offset}");

            match cmd {
                0 => {
                    /*
                    if (file_getNWords_ifExpected(file_ptr, 0, sp2C, 3)) {
                        file_getNWords(file_ptr, sp2C, 3);
                    */
                    reader.read_n(6, |r| r.read_word());
                },
                1 => {
                    return out_cubes;
                },
                2 => {
                    /*
                     !file_getNWords_ifExpected(file_ptr, 2, &sp2C, 3)
                     */
                    todo!("Cmd = 2");
                    reader.read_n(3, |r| r.read_word());
                },
                3 => { // ->cube_fromFile
                    let cube_type = reader.read_u8();
                    let count: usize = reader.read_u8().into();

                    let next_expected = match cube_type {
                        0xA => 0xB,
                        0x6 => 0x7,
                        _ => panic!("Unsupported cude type ? cube_type = {cube_type}")
                    };
        
                    println!("Reading {count} cubes (3)");
                    
                    let cube_byte_size = 20; // sizeof(NodeProp)
                    let cubes = reader.read_if_expected(next_expected, |r| r.read_u8_n(count * cube_byte_size));

                    if let Some(cubes) = cubes {
                        cubes.chunks(cube_byte_size).for_each(|cube| {
                            out_cubes.push(LevelSetupReader::u8s_to_string(cube));
                        });
                    }
                },
                8 => {
                    // this "frees" the cube in the c code
                    let count: usize = reader.read_u8().into();

                    println!("Reading {count} cubes (8)");

                    let cube_byte_size = 12; // sizeof(OtherNode)
                    let cubes = reader.read_if_expected(9, |r| r.read_u8_n(count* cube_byte_size));

                    if let Some(cubes) = cubes {
                        cubes.chunks(cube_byte_size).for_each(|cube| {
                            out_cubes.push(LevelSetupReader::u8s_to_string(cube));
                        });
                    }
                },
                _ => {
                    todo!("Unknown cmd {cmd}");
                },
            }
        }
    }


    fn build_map_hash_set() -> HashMap<usize, String> {
        let mut hs: HashMap<usize, String> = HashMap::new();
        
        hs.insert(0x1, String::from("SM_SPIRAL_MOUNTAIN"));
        hs.insert(0x2, String::from("MM_MUMBOS_MOUNTAIN"));
        hs.insert(0x5, String::from("TTC_BLUBBERS_SHIP"));
        hs.insert(0x6, String::from("TTC_NIPPERS_SHELL"));
        hs.insert(0x7, String::from("TTC_TREASURE_TROVE_COVE"));
        hs.insert(0xA, String::from("TTC_SANDCASTLE"));
        hs.insert(0xB, String::from("CC_CLANKERS_CAVERN"));
        hs.insert(0xC, String::from("MM_TICKERS_TOWER"));
        hs.insert(0xD, String::from("BGS_BUBBLEGLOOP_SWAMP"));
        hs.insert(0xE, String::from("MM_MUMBOS_SKULL"));
        hs.insert(0x10,String::from("BGS_MR_VILE"));
        hs.insert(0x11,String::from("BGS_TIPTUP"));
        hs.insert(0x12,String::from("GV_GOBIS_VALLEY"));
        hs.insert(0x13,String::from("GV_MEMORY_GAME"));
        hs.insert(0x14,String::from("GV_SANDYBUTTS_MAZE"));
        hs.insert(0x15,String::from("GV_WATER_PYRAMID"));
        hs.insert(0x16,String::from("GV_RUBEES_CHAMBER"));
        hs.insert(0x1A,String::from("GV_INSIDE_JINXY"));
        hs.insert(0x1B,String::from("MMM_MAD_MONSTER_MANSION"));
        hs.insert(0x1C,String::from("MMM_CHURCH"));
        hs.insert(0x1D,String::from("MMM_CELLAR"));
        hs.insert(0x1E,String::from("CS_START_NINTENDO"));
        hs.insert(0x1F,String::from("CS_START_RAREWARE"));
        hs.insert(0x20,String::from("CS_END_NOT_100"));
        hs.insert(0x21,String::from("CC_WITCH_SWITCH_ROOM"));
        hs.insert(0x22,String::from("CC_INSIDE_CLANKER"));
        hs.insert(0x23,String::from("CC_GOLDFEATHER_ROOM"));
        hs.insert(0x24,String::from("MMM_TUMBLARS_SHED"));
        hs.insert(0x25,String::from("MMM_WELL"));
        hs.insert(0x26,String::from("MMM_NAPPERS_ROOM"));
        hs.insert(0x27,String::from("FP_FREEZEEZY_PEAK"));
        hs.insert(0x28,String::from("MMM_EGG_ROOM"));
        hs.insert(0x29,String::from("MMM_NOTE_ROOM"));
        hs.insert(0x2A,String::from("MMM_FEATHER_ROOM"));
        hs.insert(0x2B,String::from("MMM_SECRET_CHURCH_ROOM"));
        hs.insert(0x2C,String::from("MMM_BATHROOM"));
        hs.insert(0x2D,String::from("MMM_BEDROOM"));
        hs.insert(0x2E,String::from("MMM_HONEYCOMB_ROOM"));
        hs.insert(0x2F,String::from("MMM_WATERDRAIN_BARREL"));
        hs.insert(0x30,String::from("MMM_MUMBOS_SKULL"));
        hs.insert(0x31,String::from("RBB_RUSTY_BUCKET_BAY"));
        hs.insert(0x34,String::from("RBB_ENGINE_ROOM"));
        hs.insert(0x35,String::from("RBB_WAREHOUSE"));
        hs.insert(0x36,String::from("RBB_BOATHOUSE"));
        hs.insert(0x37,String::from("RBB_CONTAINER_1"));
        hs.insert(0x38,String::from("RBB_CONTAINER_3"));
        hs.insert(0x39,String::from("RBB_CREW_CABIN"));
        hs.insert(0x3A,String::from("RBB_BOSS_BOOM_BOX"));
        hs.insert(0x3B,String::from("RBB_STORAGE_ROOM"));
        hs.insert(0x3C,String::from("RBB_KITCHEN"));
        hs.insert(0x3D,String::from("RBB_NAVIGATION_ROOM"));
        hs.insert(0x3E,String::from("RBB_CONTAINER_2"));
        hs.insert(0x3F,String::from("RBB_CAPTAINS_CABIN"));
        hs.insert(0x40,String::from("CCW_HUB"));
        hs.insert(0x41,String::from("FP_BOGGYS_IGLOO"));
        hs.insert(0x43,String::from("CCW_SPRING"));
        hs.insert(0x44,String::from("CCW_SUMMER"));
        hs.insert(0x45,String::from("CCW_AUTUMN"));
        hs.insert(0x46,String::from("CCW_WINTER"));
        hs.insert(0x47,String::from("BGS_MUMBOS_SKULL"));
        hs.insert(0x48,String::from("FP_MUMBOS_SKULL"));
        hs.insert(0x4A,String::from("CCW_SPRING_MUMBOS_SKULL"));
        hs.insert(0x4B,String::from("CCW_SUMMER_MUMBOS_SKULL"));
        hs.insert(0x4C,String::from("CCW_AUTUMN_MUMBOS_SKULL"));
        hs.insert(0x4D,String::from("CCW_WINTER_MUMBOS_SKULL"));
        hs.insert(0x53,String::from("FP_CHRISTMAS_TREE"));
        hs.insert(0x5A,String::from("CCW_SUMMER_ZUBBA_HIVE"));
        hs.insert(0x5B,String::from("CCW_SPRING_ZUBBA_HIVE"));
        hs.insert(0x5C,String::from("CCW_AUTUMN_ZUBBA_HIVE"));
        hs.insert(0x5E,String::from("CCW_SPRING_NABNUTS_HOUSE"));
        hs.insert(0x5F,String::from("CCW_SUMMER_NABNUTS_HOUSE"));
        hs.insert(0x60,String::from("CCW_AUTUMN_NABNUTS_HOUSE"));
        hs.insert(0x61,String::from("CCW_WINTER_NABNUTS_HOUSE"));
        hs.insert(0x62,String::from("CCW_WINTER_HONEYCOMB_ROOM"));
        hs.insert(0x63,String::from("CCW_AUTUMN_NABNUTS_WATER_SUPPLY"));
        hs.insert(0x64,String::from("CCW_WINTER_NABNUTS_WATER_SUPPLY"));
        hs.insert(0x65,String::from("CCW_SPRING_WHIPCRACK_ROOM"));
        hs.insert(0x66,String::from("CCW_SUMMER_WHIPCRACK_ROOM"));
        hs.insert(0x67,String::from("CCW_AUTUMN_WHIPCRACK_ROOM"));
        hs.insert(0x68,String::from("CCW_WINTER_WHIPCRACK_ROOM"));
        hs.insert(0x69,String::from("GL_MM_LOBBY"));
        hs.insert(0x6A,String::from("GL_TTC_AND_CC_PUZZLE"));
        hs.insert(0x6B,String::from("GL_180_NOTE_DOOR"));
        hs.insert(0x6C,String::from("GL_RED_CAULDRON_ROOM"));
        hs.insert(0x6D,String::from("GL_TTC_LOBBY"));
        hs.insert(0x6E,String::from("GL_GV_LOBBY"));
        hs.insert(0x6F,String::from("GL_FP_LOBBY"));
        hs.insert(0x70,String::from("GL_CC_LOBBY"));
        hs.insert(0x71,String::from("GL_STATUE_ROOM"));
        hs.insert(0x72,String::from("GL_BGS_LOBBY"));
        hs.insert(0x73,String::from("UNKNOWN"));
        hs.insert(0x74,String::from("GL_GV_PUZZLE"));
        hs.insert(0x75,String::from("GL_MMM_LOBBY"));
        hs.insert(0x76,String::from("GL_640_NOTE_DOOR"));
        hs.insert(0x77,String::from("GL_RBB_LOBBY"));
        hs.insert(0x78,String::from("GL_RBB_AND_MMM_PUZZLE"));
        hs.insert(0x79,String::from("GL_CCW_LOBBY"));
        hs.insert(0x7A,String::from("GL_CRYPT"));
        hs.insert(0x7B,String::from("CS_INTRO_GL_DINGPOT_1"));
        hs.insert(0x7C,String::from("CS_INTRO_BANJOS_HOUSE_1"));
        hs.insert(0x7D,String::from("CS_SPIRAL_MOUNTAIN_1"));
        hs.insert(0x7E,String::from("CS_SPIRAL_MOUNTAIN_2"));
        hs.insert(0x7F,String::from("FP_WOZZAS_CAVE"));
        hs.insert(0x80,String::from("GL_FF_ENTRANCE"));
        hs.insert(0x81,String::from("CS_INTRO_GL_DINGPOT_2"));
        hs.insert(0x82,String::from("CS_ENTERING_GL_MACHINE_ROOM"));
        hs.insert(0x83,String::from("CS_GAME_OVER_MACHINE_ROOM"));
        hs.insert(0x84,String::from("CS_UNUSED_MACHINE_ROOM"));
        hs.insert(0x85,String::from("CS_SPIRAL_MOUNTAIN_3"));
        hs.insert(0x86,String::from("CS_SPIRAL_MOUNTAIN_4"));
        hs.insert(0x87,String::from("CS_SPIRAL_MOUNTAIN_5"));
        hs.insert(0x88,String::from("CS_SPIRAL_MOUNTAIN_6"));
        hs.insert(0x89,String::from("CS_INTRO_BANJOS_HOUSE_2"));
        hs.insert(0x8A,String::from("CS_INTRO_BANJOS_HOUSE_3"));
        hs.insert(0x8B,String::from("RBB_ANCHOR_ROOM"));
        hs.insert(0x8C,String::from("SM_BANJOS_HOUSE"));
        hs.insert(0x8D,String::from("MMM_INSIDE_LOGGO"));
        hs.insert(0x8E,String::from("GL_FURNACE_FUN"));
        hs.insert(0x8F,String::from("TTC_SHARKFOOD_ISLAND"));
        hs.insert(0x90,String::from("GL_BATTLEMENTS"));
        hs.insert(0x91,String::from("FILE_SELECT"));
        hs.insert(0x92,String::from("GV_SNS_CHAMBER"));
        hs.insert(0x93,String::from("GL_DINGPOT"));
        hs.insert(0x94,String::from("CS_INTRO_SPIRAL_7"));
        hs.insert(0x95,String::from("CS_END_ALL_100"));
        hs.insert(0x96,String::from("CS_END_BEACH_1"));
        hs.insert(0x97,String::from("CS_END_BEACH_2"));
        hs.insert(0x98,String::from("CS_END_SPIRAL_MOUNTAIN_1"));
        hs.insert(0x99,String::from("CS_END_SPIRAL_MOUNTAIN_"));

        hs
    }

    fn bytes_to_string_vec(in_bytes: Vec<u8>) -> String {
        in_bytes.into_iter().map(|x| format!("{:02X}", x)).collect::<Vec<String>>().join(" ")
    }

    fn bytes_to_string_slice(in_bytes: &[u8]) -> String {
        in_bytes.iter().map(|x| format!("{:02X}", x)).collect::<Vec<String>>().join(" ")
    }

    pub fn read(path: &Path) -> LevelSetup{
        LevelSetup{bytes: fs::read(path).unwrap()}
    }
}

impl Asset for LevelSetup{
    fn to_bytes(&self)->Vec<u8>{
        return self.bytes.clone();
    }

    fn get_type(&self)->AssetType{
        return AssetType::LevelSetup;
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();
    }
}

/// Animation TODO !!!!!!!!!
///     - struct members
///     - from_bytes
///     - read
///     - to_bytes
///     - write

pub struct Animation{
    bytes: Vec<u8>,
}

impl Animation{
    pub fn from_bytes(in_bytes: &[u8])->Animation{
        Animation{bytes: in_bytes.to_vec()}
    }

    pub fn read(path: &Path) -> Animation{
        Animation{bytes: fs::read(path).unwrap()}
    }
}

impl Asset for Animation{
    fn to_bytes(&self)->Vec<u8>{
        return self.bytes.clone();
    }

    fn get_type(&self)->AssetType{
        return AssetType::Animation;
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();
    }
}

/// Model TODO !!!!!!!!!
///     - struct members
///     - from_bytes
///     - read
///     - to_bytes
///     - write

pub struct Model{
    bytes: Vec<u8>,
}

impl Model{
    pub fn from_bytes(in_bytes: &[u8])->Model{
        Model{bytes: in_bytes.to_vec()}
    }

    pub fn read(path: &Path) -> Model{
        Model{bytes: fs::read(path).unwrap()}
    }
}

impl Asset for Model{
    fn to_bytes(&self)->Vec<u8>{
        return self.bytes.clone();
    }

    fn get_type(&self)->AssetType{
        return AssetType::Model;
    }

    fn write(&self, path: &Path){
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();
    }
}

pub struct Texture {
    texture_type: ImgFmt,
    w : usize,
    h : usize,

    palette : Option<Vec<u8>>,
    pixel_data : Vec<u8>,
}

impl Texture {
    pub fn new(texture_type: ImgFmt, w : usize, h : usize, bin : &[u8])->Texture{
        let palette : Option<Vec<u8>> = match texture_type{
            ImgFmt::CI4 => Some(bin[0.. 0x20].to_vec()),
            ImgFmt::CI8 => Some(bin[0.. 0x200].to_vec()),
            _=> None,
        };
        
        let pixel_data = match texture_type {
            ImgFmt::CI4 => &bin[0x20..],
            ImgFmt::CI8 => &bin[0x200..],
            _ => bin,
        };

        return Texture{
            texture_type : texture_type, 
            w : w,
            h : h,
            palette : palette,
            pixel_data : pixel_data.to_vec(),
        }
    }

    pub fn to_rgba32(&self) -> Vec<u8>{
        match self.texture_type{
            ImgFmt::CI4 => 
            {   
                match &self.palette{
                    None => panic!("Expected CI4 palette, but none found"),
                    Some(pal) => Texture::ci4_to_rgba32(&self.pixel_data, &pal)
                }
            },
            ImgFmt::CI8 => 
            {   
                match &self.palette{
                    None => panic!("Expected CI8 palette, but none found"),
                    Some(pal) => Texture::ci8_to_rgba32(&self.pixel_data, &pal)
                }
            }
            ImgFmt::RGBA16 => Texture::rgba16_to_rgba32(&self.pixel_data),
            ImgFmt::RGBA32 => self.pixel_data.clone(),
            ImgFmt::I4 => Texture::i4_to_rgba32(&self.pixel_data),
            ImgFmt::I8 => Texture::i8_to_rgba32(&self.pixel_data),
            ImgFmt::IA4 => Texture::ia4_to_rgba32(&self.pixel_data),
            ImgFmt::IA8 => Texture::ia8_to_rgba32(&self.pixel_data),
            _ => {panic!("Image type not implemented yet");},

        }
    }

    pub fn rgba16_to_rgba32(rgba16 : &[u8])->Vec<u8>{
        return rgba16.chunks_exact(2)
            .map(|a|{
                let val = u16::from_be_bytes([a[0], a[1]]);
                let r16 = ((val >> 11) & 0x1f) as u8;
                let g16 = ((val >> 6) & 0x1f) as u8;
                let b16 = ((val >> 1) & 0x1f) as u8;
                let a16 = (val & 0x1) as u8;

                let r32 = (r16 << 3) | (r16 >> 3);
                let g32 = (g16 << 3) | (g16 >> 3);
                let b32 = (b16 << 3) | (b16 >> 3);
                let a32 = (((a16 << 7) as i8) >> 7) as u8;

                return [r32, g32, b32, a32]
            })
            .flatten()
            .collect()
    }

    pub fn ci4_to_rgba32(ci4 : &[u8], palatte: &[u8])->Vec<u8>{
        let pal : Vec<[u8; 4]> = palatte.chunks_exact(2)
            .map(|a|{
                let val = u16::from_be_bytes([a[0], a[1]]);
                let r16 = ((val >> 11) & 0x1f) as u8;
                let g16 = ((val >> 6) & 0x1f) as u8;
                let b16 = ((val >> 1) & 0x1f) as u8;
                let a16 = (val & 0x1) as u8;

                let r32 = (r16 << 3) | (r16 >> 3);
                let g32 = (g16 << 3) | (g16 >> 3);
                let b32 = (b16 << 3) | (b16 >> 3);
                let a32 = (((a16 << 7) as i8) >> 7) as u8;

                [r32, g32, b32, a32]
            })
            .collect();

        return ci4
            .into_iter()
            .map(|a|{[a >> 4, a & 0xF]}) //cvt to ci8
            .flatten()
            .map(|indx|{pal[indx as usize]})
            .flatten()
            .collect()
    }
    pub fn ci8_to_rgba32(ci8 : &[u8], palatte: &[u8])->Vec<u8>{
        let pal : Vec<[u8; 4]> = palatte.chunks_exact(2)
            .map(|a|{
                let val = u16::from_be_bytes([a[0], a[1]]);
                let r16 = ((val >> 11) & 0x1f) as u8;
                let g16 = ((val >> 6) & 0x1f) as u8;
                let b16 = ((val >> 1) & 0x1f) as u8;
                let a16 = (val & 0x1) as u8;

                let r32 = (r16 << 3) | (r16 >> 3);
                let g32 = (g16 << 3) | (g16 >> 3);
                let b32 = (b16 << 3) | (b16 >> 3);
                let a32 = (((a16 << 7) as i8) >> 7) as u8;

                [r32, g32, b32, a32]
            })
            .collect();

        return ci8
            .iter()
            .map(|indx|{pal[*indx as usize]})
            .flatten()
            .collect()
    }

    pub fn i4_to_rgba32(i_4 : &[u8])->Vec<u8>{
        return i_4.into_iter()
            .map(|a|{
                let val1 = (a & 0xF0) | (a >> 4);
                let val2 = (a << 4) | (a & 0xF);
                [val1, val1, val1, 0xFF, val2, val2, val2, 0xFF]
            })
            .flatten()
            .collect()
    }

    pub fn i8_to_rgba32(i_8 : &[u8])->Vec<u8>{
        return i_8.iter()
            .map(|a|{
                let val = *a;
                [val, val, val, 0xFF]
            })
            .flatten()
            .collect()
    }

    pub fn ia4_to_rgba32(ia4 : &[u8])->Vec<u8>{
        return ia4
            .into_iter()
            .map(|a|{
                let i1 = (a & 0xE0) | (a >> 3) | (a >> 6);
                let a1 = (((a << 3) as i8) >> 7) as u8;
                let i2 = (a >> 1) & 0x7;
                let i2 = (i2 << 5) | (i2 << 2) | (i2 >> 1);
                let a2 = (((a << 7) as i8) >> 7) as u8;
                [i1, i1, i1, a1, i2, i2, i2, a2]
            })
            .flatten()
            .collect()
    }

    pub fn ia8_to_rgba32(ia8 : &[u8])->Vec<u8>{
        return ia8
            .iter()
            .map(|a|{
                let val = (*a & 0xF0) | (*a >> 4);
                let alpha = (*a << 4) | (*a & 0xF);
                [val, val, val, alpha]
            })
            .flatten()
            .collect()
    }
}

struct SpriteChunk {
    x : isize,
    y : isize,
    w : usize,
    h : usize,
    pub pixel_data : Vec<u8>,
}

impl SpriteChunk {
    pub fn new(bin : &[u8], file_offset : &mut usize, format : &ImgFmt)->SpriteChunk{
        let chunk_bin = &bin[*file_offset..];
        let x = i16::from_be_bytes([chunk_bin[0], chunk_bin[1]]) as isize;
        let y = i16::from_be_bytes([chunk_bin[2], chunk_bin[3]]) as isize;
        let w = u16::from_be_bytes([chunk_bin[4], chunk_bin[5]]) as usize;
        let h = u16::from_be_bytes([chunk_bin[6], chunk_bin[7]]) as usize;
        // println!("\t\t{:02X?}", &chunk_bin[..8]);
        *file_offset += 8;
        *file_offset = (*file_offset + (8 - 1)) & !(8 - 1); //align
        let pxl_size : usize = match format{
            ImgFmt::I4 | ImgFmt::IA4 | ImgFmt::CI4 => 4,
            ImgFmt::I8 | ImgFmt::IA8 | ImgFmt::CI8 => 8,
            ImgFmt::RGBA16 => 16,
            ImgFmt::RGBA32 => 32,
            _=> 0,
        };
        let data_size : usize = w*h*pxl_size/8;

        let data : Vec<u8> = bin[*file_offset .. *file_offset + data_size].to_vec();
        *file_offset += data_size;

        SpriteChunk{
            x : x, 
            y : y, 
            w : w, 
            h : h,
            pixel_data : data, 
        }
    }
}

pub struct SpriteFrame {
    w : usize,
    h : usize,
    pub header: Vec<u8>,
    pub chk_hdrs: Vec<Vec<u8>>,
    palette : Option<Vec<u8>>,
    pixel_data : Vec<u8>,
}

impl SpriteFrame {
    pub fn new(bin : &[u8], file_offset : usize, format : &ImgFmt)->SpriteFrame{
        let header = bin[file_offset..file_offset+0x14].to_vec();
        // println!("\t{:02X?}", &header);
        let frame_bin = &bin[file_offset..];
        let x = i16::from_be_bytes([frame_bin[0], frame_bin[1]]) as isize;
        let y = i16::from_be_bytes([frame_bin[2], frame_bin[3]]) as isize;
        let w = u16::from_be_bytes([frame_bin[4], frame_bin[5]]) as usize;
        let h = u16::from_be_bytes([frame_bin[6], frame_bin[7]]) as usize;
        let mut pxl_data : Vec<Vec<[u8;4]>> = vec![vec![[0; 4]; w]; h];
        
        let chunk_cnt = u16::from_be_bytes([frame_bin[8], frame_bin[9]]);
        let mut palette :Vec<u8> = Vec::new();

        let mut offset = file_offset + 0x14;
        let mut chunks : Vec<SpriteChunk> = Vec::new();
        let mut chk_hdrs : Vec<Vec<u8>> = Vec::new();

        match format {
            ImgFmt::CI4 => {
                //align with file
                offset = (offset + (8 - 1)) & !(8 - 1) ; //align to 0x8
                palette  = bin[offset.. offset + 0x20].to_vec();
                offset += 0x20;
                
                let mut i = 0;
                while i < chunk_cnt{
                    chk_hdrs.push(bin[offset.. offset + 8].to_vec());
                    chunks.push(SpriteChunk::new(bin, &mut offset, format));
                    i += 1;
                }                
            }
            ImgFmt::CI8 => {
                //align with file
                offset = (offset + (8 - 1)) & !(8 - 1) ; //align to 0x8
                palette  = bin[offset.. offset + 0x200].to_vec();
                offset += 0x200;
                let mut i = 0;
                while i < chunk_cnt{
                    chk_hdrs.push(bin[offset.. offset + 8].to_vec());
                    chunks.push(SpriteChunk::new(bin, &mut offset, format));
                    i += 1;
                }
            }
            ImgFmt::I4 => {
                offset = offset;
                let mut i = 0;
                while i < chunk_cnt{
                    chk_hdrs.push(bin[offset.. offset + 8].to_vec());
                    chunks.push(SpriteChunk::new(bin, &mut offset, format));
                    i += 1;
                }
            }
            ImgFmt::I8 => {
                offset = offset;
                let mut i = 0;
                while i < chunk_cnt{
                    chk_hdrs.push(bin[offset.. offset + 8].to_vec());
                    chunks.push(SpriteChunk::new(bin, &mut offset, format));
                    i += 1;
                }
            }
            ImgFmt::RGBA32 => {
                offset = offset;
                let mut i = 0;
                while i < chunk_cnt{
                    chk_hdrs.push(bin[offset.. offset + 8].to_vec());
                    chunks.push(SpriteChunk::new(bin, &mut offset, format));
                    i += 1;
                }
            }
            ImgFmt::RGBA16 => {
                offset = offset;
                let mut i = 0;
                while i < chunk_cnt{
                    chk_hdrs.push(bin[offset.. offset + 8].to_vec());
                    chunks.push(SpriteChunk::new(bin, &mut offset, format));
                    i += 1;
                }
            }
            _ => {}
        }

        for chnk in chunks{
            let raw_data = match format {
                ImgFmt::CI4    => Texture::ci4_to_rgba32(&chnk.pixel_data, &palette),
                ImgFmt::CI8    => Texture::ci8_to_rgba32(&chnk.pixel_data, &palette),
                ImgFmt::I4     => Texture::i4_to_rgba32(&chnk.pixel_data),
                ImgFmt::I8     => Texture::i4_to_rgba32(&chnk.pixel_data),
                ImgFmt::RGBA16 => Texture::rgba16_to_rgba32(&chnk.pixel_data),
                ImgFmt::RGBA32 => chnk.pixel_data,
                ImgFmt::IA4    => Texture::ia4_to_rgba32(&chnk.pixel_data),
                ImgFmt::IA8    => Texture::ia4_to_rgba32(&chnk.pixel_data),
                _=> Vec::new(),
            };

            if(chunk_cnt) == 1{
                let row_data : Vec<&[u8]> = raw_data.chunks_exact(4*chnk.w).collect();

                for (j,row) in row_data.iter().enumerate(){
                    for (i, pxl) in row.chunks_exact(4).enumerate(){
                        let fx :isize = i as isize;
                        let fy :isize = j as isize;
                        if (0 <= fx) && (fx < (w as isize)) && (0 <= fy) && (fy < (h as isize)){
                            pxl_data[fy as usize][fx as usize] = pxl.try_into().unwrap();
                        }
                    }
                }
            }
            else{
                let row_data : Vec<&[u8]> = raw_data.chunks_exact(4*chnk.w).collect();
                for (j,row) in row_data.iter().enumerate(){
                    for (i, pxl) in row.chunks_exact(4).enumerate(){
                        let fx :isize = (chnk.x + i as isize) as isize;
                        let fy :isize = (chnk.y + j as isize) as isize;
                        if (0 <= fx) && (fx < (w as isize)) && (0 <= fy) && (fy < (h as isize)){
                            pxl_data[fy as usize][fx as usize] = pxl.try_into().unwrap();
                        }
                    }
                }
            }
        }

        let pal = match format{
            ImgFmt::CI4 | ImgFmt::CI8 => Some(palette),
            _ => None,
        };

        SpriteFrame{w: w as usize,h: h as usize, header: header, chk_hdrs:chk_hdrs, palette : pal, pixel_data: pxl_data.into_iter().flatten().flatten().collect()}
    }
}

pub struct Sprite{
    format: ImgFmt,
    pub frame: Vec<SpriteFrame>,
    bytes: Vec<u8>,
}

impl Sprite{
    pub fn from_bytes(in_bytes: &[u8])->Sprite{
        let frame_cnt = u16::from_be_bytes([in_bytes[0], in_bytes[1]]);
        let format = u16::from_be_bytes([in_bytes[2], in_bytes[3]]);
        let frmt = match format{
            0x0001 => ImgFmt::CI4,
            0x0004 => ImgFmt::CI8,
            0x0020 => ImgFmt::I4,
            0x0040 => ImgFmt::I8,
            0x0400 => ImgFmt::RGBA16,
            0x0800 => ImgFmt::RGBA32,
            _ => ImgFmt::Unknown(format),
        };
        match frmt {
            ImgFmt::Unknown(_) => {return Sprite{format: frmt, frame: Vec::new(), bytes: in_bytes.to_vec()}},
            _=> {}
        }

        if frame_cnt > 0x100{
            let mut offset = 8 as usize;
            let chunk = SpriteChunk::new(in_bytes, &mut offset, &ImgFmt::RGBA16);
            let frame = SpriteFrame{w:chunk.w, h:chunk.h, header: Vec::new(), chk_hdrs: vec![in_bytes[8..16].to_vec()], palette: None, pixel_data: Texture::rgba16_to_rgba32(&chunk.pixel_data)};
            return Sprite{format: frmt, frame: vec![frame], bytes: in_bytes.to_vec()};
        }
        // println!("{:02X?}", &in_bytes[..0x10]);
        let frames : Vec<SpriteFrame>= in_bytes[0x10..]
                .chunks_exact(0x4)
                .take(frame_cnt as usize)
                .map(|a|{
                    let offset = u32::from_be_bytes(a.try_into().unwrap());
                    SpriteFrame::new(in_bytes, 0x10 + offset as usize + 4*frame_cnt as usize, &frmt)
                })
                .collect(); 
        return Sprite{format: frmt, frame: frames, bytes: in_bytes.to_vec()};
    }

    pub fn read(path: &Path) -> Sprite{
        Sprite{format: ImgFmt::Unknown(0), frame: Vec::new(), bytes: fs::read(path).unwrap()}
    }
}

/// Sprite TODO !!!!!!!!!
///     - struct members
///     - read
///     - to_bytes

impl Asset for Sprite{
    fn to_bytes(&self)->Vec<u8>{
        return self.bytes.clone();
    }

    fn get_type(&self)->AssetType{
        return AssetType::Sprite(self.format);
    }

    fn write(&self, path: &Path){
        //write bin. TODO remove once one to 1 conversion
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();

        //write descriptor yaml and folder containing frame pngs
        let base_name = Path::new(path.file_stem().unwrap());
        let fmt_str = base_name.extension().unwrap();
        let new_base = Path::new(base_name.file_stem().unwrap());
        let base_name = Path::new(new_base.file_stem().unwrap());
        let base_path = path.parent().unwrap().join(base_name);
        let mut desc_path = base_path.clone();
        desc_path.set_extension("sprite.yaml");
        let mut desc_f = File::create(desc_path).unwrap();
        writeln!(desc_f, "type: Sprite").unwrap();
        writeln!(desc_f, "format: {:?}", self.format).unwrap();
        writeln!(desc_f, "frames:").unwrap();
        
        DirBuilder::new().recursive(true).create(&base_path.clone()).unwrap();
        for(i, frame) in self.frame.iter().enumerate(){
            let mut i_path = base_path.join(format!("{:02X}.", i));
            i_path.set_extension(format!("{}.png",fmt_str.to_str().unwrap()));
            writeln!(desc_f, "  - {:?}", i_path).unwrap();
            let texture_f = File::create(i_path).unwrap();
            let ref mut w = BufWriter::new(texture_f);

            let mut encoder = png::Encoder::new(w, frame.w as u32, frame.h as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();

            let data = &frame.pixel_data;
            // let mirrored : Vec<u8> = data.rchunks_exact(4*frame.w).map(|a|{a.to_vec()}).flatten().collect();

            writer.write_image_data(&data).unwrap(); // Save
        }
    }
}
