use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use yaml_rust::YamlLoader;

use super::asset::{Asset, AssetType};

fn u8s_to_readable_hex(in_bytes: &[u8], chunk_size: usize) -> String {
    in_bytes
        .chunks(chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .map(|x| format!("{:02X}", x))
                .collect::<Vec<String>>()
                .join(" ")
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn u8s_to_byte_array_string(bytes: &[u8]) -> String {
    format!(
        "[{}]",
        bytes
            .iter()
            .map(|x| format!("0x{:02X}", x))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

/// LevelSetup TODO !!!!!!!!!
///     - read
///
#[derive(Clone, Debug)]
pub struct LevelSetup {
    bytes: Vec<u8>,
    voxel_list: LevelVoxelList,
    camera_nodes: CameraNodeList,
    lighting_nodes: LightingNodeList,
}

type VoxelObject = Option<Vec<u8>>; // 20 bytes
type VoxelProp = Vec<u8>; // 16 bytes

#[derive(Clone, Debug)]
struct LevelVoxelList {
    start_position: [i32; 3],
    end_position: [i32; 3],
    voxels: Vec<LevelVoxel>,
}

#[derive(Clone, Debug)]
struct LevelVoxel {
    objects: Vec<VoxelObject>,
    props: Vec<VoxelProp>,
}

#[derive(Clone, Debug)]
struct LightingNodeList {
    nodes: Vec<LightingNode>,
}

#[derive(Clone, Debug)]
struct CameraNodeList {
    nodes: Vec<LevelCameraNode>,
}

#[derive(Clone, Debug)]
struct LevelCameraNode {
    index: i16,
    camera_type: u8,
    sections: Vec<PayloadSection>,
}

#[derive(Clone, Debug)]
struct PayloadSection {
    index: u8,
    bytes: Vec<u8>,
}

struct LevelSetupReader<'a> {
    in_bytes: &'a [u8],
    offset: usize,
}

impl LevelSetupReader<'_> {
    pub fn new(in_bytes: &[u8]) -> LevelSetupReader {
        LevelSetupReader {
            in_bytes,
            offset: 0,
        }
    }

    pub fn read_word(&mut self) -> i32 {
        self.read_i32()
    }

    // the BK code uses s32 instead of i32
    pub fn read_i32(&mut self) -> i32 {
        let out = i32::from_be_bytes([
            self.in_bytes[self.offset],
            self.in_bytes[self.offset + 1],
            self.in_bytes[self.offset + 2],
            self.in_bytes[self.offset + 3],
        ]);

        self.offset += 4;

        out
    }

    // the BK code uses s16 instead of i16
    pub fn read_i16(&mut self) -> i16 {
        let out = i16::from_be_bytes([self.in_bytes[self.offset], self.in_bytes[self.offset + 1]]);
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
            self.in_bytes[self.offset + 1],
            self.in_bytes[self.offset + 2],
            self.in_bytes[self.offset + 3],
        ]);

        self.offset += 4;

        out
    }

    pub fn read_u16(&mut self) -> u16 {
        let out = u16::from_be_bytes([self.in_bytes[self.offset], self.in_bytes[self.offset + 1]]);
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
            self.in_bytes[self.offset + 1],
            self.in_bytes[self.offset + 2],
            self.in_bytes[self.offset + 3],
        ]);

        self.offset += 4;

        out
    }

    pub fn read_n<T>(
        &mut self,
        n: usize,
        reader_fn: impl Fn(&mut LevelSetupReader) -> T,
    ) -> Vec<T> {
        let mut out = vec![];
        for _ in 0..n {
            out.push(reader_fn(self));
        }

        out
    }

    pub fn read_u8_n(&mut self, n: usize) -> Vec<u8> {
        return self.read_n(n, |r| r.read_u8());
        let out = self.in_bytes[self.offset..(self.offset + n)].into();
        self.offset += n;

        out
    }

    pub fn read_if_expected<T>(
        &mut self,
        expected_value: u8,
        reader_fn: impl Fn(&mut LevelSetupReader) -> T,
    ) -> Option<T> {
        if self.in_bytes[self.offset] == expected_value {
            self.offset += 1;
            Some(reader_fn(self))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct LightingNode {
    position: [f32; 3],
    // some unknown factors which are used to calculate an RGB modifier to modify the vertex RGB
    // goes up in steps of 0.125
    // it looks like the second flag also determines the minimum distance between lighting position and vertex position as to where the rgb modification should take place
    unknown_flags: [f32; 2],
    rgb: [u8; 3],
}

impl LevelSetup {
    pub fn from_bytes(in_bytes: &[u8]) -> LevelSetup {
        let mut voxels_list: Option<LevelVoxelList> = None;
        let mut camera_nodes: Vec<LevelCameraNode> = vec![];
        let mut lighting_nodes: Vec<LightingNode> = vec![];
        let mut reader = LevelSetupReader::new(in_bytes);
        loop {
            let cmd = reader.read_u8();
            match cmd {
                0 => {
                    break;
                }
                1 => {
                    assert!(
                        voxels_list.is_none(),
                        "Only one voxel definition per level setup expected"
                    );

                    // cubeList_fromFile
                    // file_getNWords_ifExpected(file_ptr, 1, sp50, 3)
                    let voxel_negative_position = reader
                        .read_if_expected(1, |r| [r.read_word(), r.read_word(), r.read_word()])
                        .expect("Level setup should have negative position");

                    // file_getNWords(file_ptr, sp44, 3)
                    let voxel_positive_position =
                        [reader.read_word(), reader.read_word(), reader.read_word()];

                    let mut voxels = vec![];
                    for _x in voxel_negative_position[0]..=voxel_positive_position[0] {
                        for _y in voxel_negative_position[1]..=voxel_positive_position[1] {
                            for _z in voxel_negative_position[2]..=voxel_positive_position[2] {
                                voxels.push(LevelSetup::get_level_voxel_from_reader(&mut reader));
                            }
                        }
                    }

                    // in the c code after the for loops there is:
                    // file_isNextByteExpected(file_ptr, 0);
                    // which, in essence, advances the file_ptr by 1 if the current value is 0
                    reader.read_if_expected(0, |_| 0);

                    voxels_list = Some(LevelVoxelList {
                        start_position: voxel_negative_position,
                        end_position: voxel_positive_position,
                        voxels,
                    });
                }
                3 => {
                    // ncCameraNodeList_fromFile
                    loop {
                        let cmd = reader.read_u8();
                        if cmd == 0 {
                            break;
                        }

                        if cmd != 1 {
                            panic!("Unexpected cmd {cmd}");
                        }

                        let camera_node_index = reader.read_i16();
                        let camera_node_type =
                            reader.read_if_expected(2, |r| r.read_u8()).unwrap_or(0);

                        let mut sections = vec![];

                        match camera_node_type {
                            0 => {
                                // no-op
                            }
                            1 => {
                                // cameraNodeType1_fromFile
                                loop {
                                    let section_index = reader.read_u8();
                                    let section_bytes = match section_index {
                                        0 => break,
                                        1 | 4 => [
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                        ]
                                        .as_flattened()
                                        .to_vec(),
                                        2 | 3 => [
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                        ]
                                        .as_flattened()
                                        .to_vec(),
                                        5 => reader.read_word().to_be_bytes().to_vec(),
                                        _ => panic!("Unknown section = {section_index}"),
                                    };

                                    sections.push(PayloadSection {
                                        index: section_index,
                                        bytes: section_bytes,
                                    });
                                }
                            }
                            2 => {
                                // cameraNodeType2_fromFile
                                loop {
                                    let section_index = reader.read_u8();
                                    let section_bytes = match section_index {
                                        0 => break,
                                        1 | 2 => [
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                        ]
                                        .as_flattened()
                                        .to_vec(),
                                        _ => panic!("Unknown section = {section_index}"),
                                    };

                                    sections.push(PayloadSection {
                                        index: section_index,
                                        bytes: section_bytes,
                                    });
                                }
                            }
                            3 => {
                                // cameraNodeType3_fromFile
                                loop {
                                    let section_index = reader.read_u8();
                                    let section_bytes = match section_index {
                                        0 => break,
                                        1 | 4 => [
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                        ]
                                        .as_flattened()
                                        .to_vec(),
                                        2 | 3 | 6 => [
                                            reader.read_f32().to_be_bytes(),
                                            reader.read_f32().to_be_bytes(),
                                        ]
                                        .as_flattened()
                                        .to_vec(),
                                        5 => reader.read_word().to_be_bytes().to_vec(),
                                        _ => panic!("Unknown section = {section_index}"),
                                    };
                                    sections.push(PayloadSection {
                                        index: section_index,
                                        bytes: section_bytes,
                                    });
                                }
                            }
                            4 => {
                                // cameraNodeType4_fromFile
                                loop {
                                    let section_index = reader.read_u8();
                                    let section_bytes = match section_index {
                                        0 => break,
                                        1 => reader.read_i32().to_be_bytes().to_vec(),
                                        _ => panic!("Unknown section = {section_index}"),
                                    };
                                    sections.push(PayloadSection {
                                        index: section_index,
                                        bytes: section_bytes,
                                    });
                                }
                            }
                            _ => {
                                panic!("Unknown camera_node_type {camera_node_type}");
                            }
                        }

                        camera_nodes.push(LevelCameraNode {
                            index: camera_node_index,
                            camera_type: camera_node_type,
                            sections,
                        });
                    }
                }
                4 => {
                    // lightingVectorList_fromFile
                    loop {
                        let cmd = reader.read_u8();
                        if cmd == 0 {
                            break;
                        }

                        if cmd != 1 {
                            panic!("Unexpected cmd = {cmd}");
                        }

                        // file_getNFloats_ifExpected(file_ptr, 2, position, 3)
                        let (position, unknown_flags, rgb) = reader
                            .read_if_expected(2, |r| {
                                let position = [r.read_f32(), r.read_f32(), r.read_f32()];

                                // file_getNFloats_ifExpected(file_ptr, 3, unknown_flags, 2)
                                let (unknown_flags, rgb) = r
                                    .read_if_expected(3, |r| {
                                        let unknown_flags = [r.read_f32(), r.read_f32()];

                                        // file_getNWords_ifExpected(file_ptr, 4, rgb, 3)
                                        let rgb = r
                                            .read_if_expected(4, |r| {
                                                // the C code reads words, however only the last byte of the 4 read bytes contain the RGB hex value (0-255)
                                                let rgb = r.read_n(12, |r| r.read_u8());

                                                [rgb[3], rgb[7], rgb[11]]
                                            })
                                            .expect("Unable to read RGB of lighting node");

                                        (unknown_flags, rgb)
                                    })
                                    .expect("Unable to read unknown flags of lighting node");

                                (position, unknown_flags, rgb)
                            })
                            .expect("Unable to read position of lighting node");

                        lighting_nodes.push(LightingNode {
                            position,
                            unknown_flags,
                            rgb,
                        });
                    }
                }
                _ => {
                    todo!("Implement cmd {cmd}?");
                }
            }
        }

        LevelSetup {
            bytes: in_bytes.to_vec(),
            voxel_list: voxels_list.expect("Voxels list to exist in level setup file"),
            camera_nodes: CameraNodeList {
                nodes: camera_nodes,
            },
            lighting_nodes: LightingNodeList {
                nodes: lighting_nodes,
            },
        }
    }

    fn get_level_voxel_from_reader(reader: &mut LevelSetupReader) -> LevelVoxel {
        let mut voxel_objects: Vec<VoxelObject> = vec![];
        let mut voxel_props: Vec<VoxelProp> = vec![];

        loop {
            let cmd = reader.read_u8();

            match cmd {
                0 => {
                    /*
                    if (file_getNWords_ifExpected(file_ptr, 0, sp2C, 3)) {
                        file_getNWords(file_ptr, sp2C, 3);
                    */
                    panic!("Unexpected cmd 0");
                }
                1 => {
                    return LevelVoxel {
                        objects: voxel_objects,
                        props: voxel_props,
                    };
                }
                2 => {
                    /*
                    !file_getNWords_ifExpected(file_ptr, 2, &sp2C, 3)
                    */
                    panic!("Unexpected cmd 2");
                }
                3 => {
                    // ->code7AF80_initCubeFromFile
                    let voxel_type = reader.read_u8();
                    let count: usize = reader.read_u8().into();
                    assert!(
                        voxel_type == 0xA || voxel_type == 0x6,
                        "Unknown voxel type {voxel_type}"
                    );

                    if count == 0 {
                        voxel_objects.push(None);
                        continue;
                    }

                    let next_expected = voxel_type + 1;
                    let voxel_byte_size = 20;
                    let voxel_bytes = reader
                        .read_if_expected(next_expected, |r| r.read_u8_n(count * voxel_byte_size));

                    if let Some(voxel_bytes) = voxel_bytes {
                        voxel_bytes
                            .chunks(voxel_byte_size)
                            .for_each(|voxel| voxel_objects.push(Some(voxel.to_vec())));
                    } else {
                        panic!(
                            "Did not read {voxel_byte_size}, count = {count}, found {}",
                            reader.read_u8()
                        );
                    }
                }
                8 => {
                    let count: usize = reader.read_u8().into();

                    if count == 0 {
                        continue;
                    }

                    let voxel_byte_size = 12; // sizeof(Prop)
                    let voxel_bytes =
                        reader.read_if_expected(9, |r| r.read_u8_n(count * voxel_byte_size));

                    if let Some(voxel_bytes) = voxel_bytes {
                        voxel_bytes.chunks(voxel_byte_size).for_each(|voxel| {
                            voxel_props.push(voxel.to_vec());
                        });
                    } else {
                        panic!("Did not read props?")
                    }
                }
                _ => {
                    panic!("Unexpected cmd {cmd}");
                }
            }
        }
    }

    pub fn read(path: &Path) -> LevelSetup {
        let doc =
            &YamlLoader::load_from_str(&fs::read_to_string(path).expect("could not open yaml"))
                .unwrap()[0];
        let doc_type = doc["type"].as_str().unwrap();
        assert_eq!(doc_type, "LevelSetup");

        todo!("Implement read");
    }
}

impl Asset for LevelSetup {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }

    fn get_type(&self) -> AssetType {
        AssetType::LevelSetup
    }

    fn write(&self, path: &Path) {
        let mut bin_file = File::create(path).unwrap();
        let mut out_bytes: Vec<u8> = vec![];

        self.voxel_list.as_bytes(&mut out_bytes);
        self.camera_nodes.as_bytes(&mut out_bytes);
        self.lighting_nodes.as_bytes(&mut out_bytes);

        // EOF
        out_bytes.push(0);

        let mut hex_file = File::create(path.with_extension("hex")).unwrap();

        hex_file
            .write_all(u8s_to_readable_hex(out_bytes.as_slice(), 16).as_bytes())
            .unwrap();

        bin_file.write_all(&out_bytes).unwrap();

        if out_bytes.ne(&self.bytes) {
            panic!("Byte mismatch for {path:?}");
        }

        let mut yaml_file = File::create(path.with_extension("yaml")).unwrap();

        writeln!(yaml_file, "type: LevelSetup").unwrap();
        writeln!(yaml_file, "voxels:").unwrap();
        writeln!(
            yaml_file,
            "  startPosition: {{ x: {}, y: {}, z: {} }}",
            self.voxel_list.start_position[0],
            self.voxel_list.start_position[1],
            self.voxel_list.start_position[2]
        )
        .unwrap();

        writeln!(
            yaml_file,
            "  endPosition: {{ x: {}, y: {}, z: {} }}",
            self.voxel_list.end_position[0],
            self.voxel_list.end_position[1],
            self.voxel_list.end_position[2]
        )
        .unwrap();
        writeln!(yaml_file, "  voxels:").unwrap();

        for voxel in &self.voxel_list.voxels {
            writeln!(yaml_file, "    - {{").unwrap();
            if voxel.objects.is_empty() {
                writeln!(yaml_file, "      objects: [],").unwrap();
            } else {
                writeln!(yaml_file, "      objects: [").unwrap();
                for object in &voxel.objects {
                    let bytes = object.clone().unwrap_or(vec![]);
                    writeln!(
                        yaml_file,
                        "        {},",
                        u8s_to_byte_array_string(bytes.as_slice())
                    )
                    .unwrap();
                }
                writeln!(yaml_file, "      ],").unwrap();
            }
            if voxel.props.is_empty() {
                writeln!(yaml_file, "      props: []").unwrap();
            } else {
                writeln!(yaml_file, "      props: [").unwrap();
                for prop in &voxel.props {
                    writeln!(yaml_file, "        {},", u8s_to_byte_array_string(prop)).unwrap();
                }
                writeln!(yaml_file, "      ]").unwrap();
            }
            writeln!(yaml_file, "    }}").unwrap();
        }

        writeln!(yaml_file, "cameras:").unwrap();
        for camera_node in &self.camera_nodes.nodes {
            writeln!(yaml_file, "  -").unwrap();
            writeln!(yaml_file, "    index: {}", camera_node.index).unwrap();
            writeln!(yaml_file, "    type: {}", camera_node.camera_type).unwrap();
            writeln!(yaml_file, "    sections:",).unwrap();
            for section in &camera_node.sections {
                writeln!(
                    yaml_file,
                    "      - {{ section: {}, bytes: {} }}",
                    section.index,
                    u8s_to_byte_array_string(&section.bytes)
                )
                .unwrap();
            }
        }

        writeln!(yaml_file, "lightings:").unwrap();
        for lighting_node in &self.lighting_nodes.nodes {
            writeln!(yaml_file, "    - {{").unwrap();
            writeln!(
                yaml_file,
                "      position: {{ x: {}, y: {}, z: {} }},",
                lighting_node.position[0], lighting_node.position[1], lighting_node.position[2]
            )
            .unwrap();
            writeln!(
                yaml_file,
                "      flags: [{}, {}],",
                u8s_to_byte_array_string(&lighting_node.unknown_flags[0].to_be_bytes()),
                u8s_to_byte_array_string(&lighting_node.unknown_flags[1].to_be_bytes())
            )
            .unwrap();
            writeln!(
                yaml_file,
                "      rgb: {:02X}{:02X}{:02X}",
                lighting_node.rgb[0], lighting_node.rgb[1], lighting_node.rgb[2]
            )
            .unwrap();
            writeln!(yaml_file, "    }}").unwrap();
        }
    }
}

impl LevelVoxelList {
    fn as_bytes(&self, out_bytes: &mut Vec<u8>) {
        out_bytes.push(1);

        out_bytes.push(1);
        out_bytes.append(
            &mut self
                .start_position
                .map(|x| x.to_be_bytes().to_vec())
                .concat(),
        );

        out_bytes.append(&mut self.end_position.map(|x| x.to_be_bytes().to_vec()).concat());

        for voxel in &self.voxels {
            if !voxel.objects.is_empty() || !voxel.props.is_empty() {
                out_bytes.push(3);

                out_bytes.push(0xA);
                let objects = voxel
                    .objects
                    .iter()
                    .filter_map(|x| x.clone())
                    .collect::<Vec<Vec<u8>>>();
                assert!((objects.len() as u8) < u8::MAX);

                out_bytes.push(objects.len() as u8);
                if !objects.is_empty() {
                    out_bytes.push(0xB);
                    for object in objects {
                        out_bytes.append(&mut object.clone());
                    }
                }

                out_bytes.push(0x8);
                let props = voxel.props.clone();
                assert!((props.len() as u8) < u8::MAX);

                out_bytes.push(props.len() as u8);
                if !props.is_empty() {
                    out_bytes.push(9);
                    for prop in props {
                        out_bytes.append(&mut prop.clone());
                    }
                }
            }

            out_bytes.push(1);
        }

        out_bytes.push(0);
    }
}

impl CameraNodeList {
    fn as_bytes(&self, out_bytes: &mut Vec<u8>) {
        out_bytes.push(3);
        for node in &self.nodes {
            out_bytes.push(1);
            out_bytes.append(&mut node.index.to_be_bytes().into());
            out_bytes.push(2);
            out_bytes.push(node.camera_type);
            if !node.sections.is_empty() {
                for section in &node.sections {
                    out_bytes.push(section.index);
                    out_bytes.append(&mut section.bytes.clone());
                }
                out_bytes.push(0);
            }
        }
        out_bytes.push(0);
    }
}

impl LightingNodeList {
    fn as_bytes(&self, out_bytes: &mut Vec<u8>) {
        out_bytes.push(4);
        for node in &self.nodes {
            out_bytes.push(1);
            out_bytes.push(2);
            for xyz in node.position {
                out_bytes.append(&mut xyz.to_be_bytes().into());
            }
            out_bytes.push(3);

            for flag in node.unknown_flags {
                out_bytes.append(&mut flag.to_be_bytes().into());
            }
            out_bytes.push(4);

            for rgb in node.rgb {
                out_bytes.append(&mut vec![0, 0, 0, rgb]);
            }
        }
        out_bytes.push(0);
    }
}
