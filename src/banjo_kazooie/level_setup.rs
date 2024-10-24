use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

use super::asset::{Asset, AssetType};

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

#[derive(Clone, Debug)]
pub struct LevelSetup {
    voxel_list: VoxelList,
    camera_nodes: CameraNodeList,
    lighting_nodes: LightingNodeList,
}

#[derive(Clone, Debug)]
struct VoxelList {
    start_position: [i32; 3],
    end_position: [i32; 3],
    voxels: Vec<Voxel>,
}

type VoxelObject = Option<Vec<u8>>; // 20 bytes
type VoxelProp = Vec<u8>; // 16 bytes

#[derive(Clone, Debug)]
struct Voxel {
    objects: Vec<VoxelObject>,
    props: Vec<VoxelProp>,
}

#[derive(Clone, Debug)]
struct LightingNodeList {
    nodes: Vec<LightingNode>,
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

#[derive(Clone, Debug)]
struct CameraNodeList {
    nodes: Vec<CameraNode>,
}

#[derive(Clone, Debug)]
struct CameraNode {
    index: i16,
    camera_type: u8,
    sections: Vec<CameraNodeSection>,
}

#[derive(Clone, Debug)]
struct CameraNodeSection {
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

impl LevelSetup {
    pub fn from_bytes(in_bytes: &[u8]) -> LevelSetup {
        let mut reader = LevelSetupReader::new(in_bytes);

        let voxel_list = reader
            .read_if_expected(1, VoxelList::from_reader)
            .expect("Voxel list should be first in level setup file");

        let camera_nodes = reader
            .read_if_expected(3, CameraNodeList::from_reader)
            .expect("Camera nodes should be after voxel list in level setup file");

        let lighting_nodes = reader
            .read_if_expected(4, LightingNodeList::from_reader)
            .expect("Lighting nodes should be after camera nodes in level setup file");

        LevelSetup {
            voxel_list,
            camera_nodes,
            lighting_nodes,
        }
    }

    pub fn read(path: &Path) -> LevelSetup {
        let doc =
            &YamlLoader::load_from_str(&fs::read_to_string(path).expect("could not open yaml"))
                .unwrap()[0];
        let doc_type = doc["type"].as_str().unwrap();
        assert_eq!(doc_type, "LevelSetup");

        LevelSetup {
            voxel_list: VoxelList::from_yaml(doc),
            camera_nodes: CameraNodeList::from_yaml(doc),
            lighting_nodes: LightingNodeList::from_yaml(doc),
        }
    }
}

impl Asset for LevelSetup {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out_bytes: Vec<u8> = vec![];

        self.voxel_list.to_bytes(&mut out_bytes);
        self.camera_nodes.to_bytes(&mut out_bytes);
        self.lighting_nodes.to_bytes(&mut out_bytes);

        // EOF
        out_bytes.push(0);

        out_bytes
    }

    fn get_type(&self) -> AssetType {
        AssetType::LevelSetup
    }

    fn write(&self, path: &Path) {
        let mut yaml_file = File::create(path).unwrap();

        writeln!(yaml_file, "type: LevelSetup").unwrap();
        writeln!(yaml_file, "{}", self.voxel_list.to_yaml()).unwrap();
        writeln!(yaml_file, "{}", self.camera_nodes.to_yaml()).unwrap();
        writeln!(yaml_file, "{}", self.lighting_nodes.to_yaml()).unwrap();
    }
}

impl VoxelList {
    fn to_bytes(&self, out_bytes: &mut Vec<u8>) {
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

    fn from_reader(reader: &mut LevelSetupReader) -> VoxelList {
        // cubeList_fromFile
        // file_getNWords_ifExpected(file_ptr, 1, sp50, 3)
        let start_position = reader
            .read_if_expected(1, |r| [r.read_word(), r.read_word(), r.read_word()])
            .expect("Level setup should have negative position");

        // file_getNWords(file_ptr, sp44, 3)
        let end_position = [reader.read_word(), reader.read_word(), reader.read_word()];

        let mut voxels = vec![];
        for _x in start_position[0]..=end_position[0] {
            for _y in start_position[1]..=end_position[1] {
                for _z in start_position[2]..=end_position[2] {
                    voxels.push(Self::get_level_voxel_from_reader(reader));
                }
            }
        }

        // in the c code after the for loops there is:
        // file_isNextByteExpected(file_ptr, 0);
        // which, in essence, advances the file_ptr by 1 if the current value is 0
        reader.read_if_expected(0, |_| 0);

        VoxelList {
            start_position,
            end_position,
            voxels,
        }
    }

    fn get_level_voxel_from_reader(reader: &mut LevelSetupReader) -> Voxel {
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
                    return Voxel {
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

    fn to_yaml(&self) -> String {
        let mut out_string = String::new();

        out_string.push_str("voxels:\n");
        out_string.push_str(
            format!(
                "  startPosition: {{ x: {:?}, y: {:?}, z: {:?} }}\n",
                self.start_position[0], self.start_position[1], self.start_position[2]
            )
            .as_str(),
        );

        out_string.push_str(
            format!(
                "  endPosition: {{ x: {:?}, y: {:?}, z: {:?} }}\n",
                self.end_position[0], self.end_position[1], self.end_position[2]
            )
            .as_str(),
        );
        out_string.push_str("  voxels:\n");

        for voxel in &self.voxels {
            out_string.push_str("    - {\n");
            if voxel.objects.is_empty() {
                out_string.push_str("      objects: [],\n");
            } else {
                out_string.push_str("      objects: [\n");
                for object in &voxel.objects {
                    let bytes = object.clone().unwrap_or(vec![]);
                    out_string.push_str(
                        format!("        {},\n", u8s_to_byte_array_string(bytes.as_slice()))
                            .as_str(),
                    );
                }
                out_string.push_str("      ],\n");
            }
            if voxel.props.is_empty() {
                out_string.push_str("      props: []\n");
            } else {
                out_string.push_str("      props: [\n");
                for prop in &voxel.props {
                    out_string.push_str(
                        format!("        {},\n", u8s_to_byte_array_string(prop)).as_str(),
                    );
                }
                out_string.push_str("      ]\n");
            }
            out_string.push_str("    }\n");
        }

        out_string
    }

    fn from_yaml(doc: &Yaml) -> VoxelList {
        let start_position_vec = &doc["voxels"]["startPosition"];
        let voxels_start_position = [
            start_position_vec["x"].as_i64().unwrap() as i32,
            start_position_vec["y"].as_i64().unwrap() as i32,
            start_position_vec["z"].as_i64().unwrap() as i32,
        ];

        let end_position_vec = &doc["voxels"]["endPosition"];
        let voxels_end_position = [
            end_position_vec["x"].as_i64().unwrap() as i32,
            end_position_vec["y"].as_i64().unwrap() as i32,
            end_position_vec["z"].as_i64().unwrap() as i32,
        ];

        let yaml_voxels = &doc["voxels"]["voxels"].as_vec().unwrap();
        let mut voxels = vec![];
        for yaml_voxel in *yaml_voxels {
            let mut objects = vec![];
            let yaml_objects = &yaml_voxel["objects"].as_vec().unwrap();
            for yaml_object in yaml_objects.iter() {
                let object_bytes = yaml_object
                    .as_vec()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_i64().unwrap() as u8)
                    .collect::<Vec<u8>>();
                if object_bytes.is_empty() {
                    objects.push(None);
                } else {
                    objects.push(Some(object_bytes));
                }
            }

            let mut props = vec![];
            let yaml_props = &yaml_voxel["props"].as_vec().unwrap();
            for yaml_prop in yaml_props.iter() {
                let prop_bytes = yaml_prop
                    .as_vec()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_i64().unwrap() as u8)
                    .collect::<Vec<u8>>();
                props.push(prop_bytes);
            }
            voxels.push(Voxel { objects, props });
        }

        VoxelList {
            voxels,
            start_position: voxels_start_position,
            end_position: voxels_end_position,
        }
    }
}

impl CameraNodeList {
    fn to_bytes(&self, out_bytes: &mut Vec<u8>) {
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

    fn from_reader(reader: &mut LevelSetupReader) -> CameraNodeList {
        // ncCameraNodeList_fromFile
        let mut nodes = vec![];
        loop {
            let cmd = reader.read_u8();
            if cmd == 0 {
                break;
            }

            if cmd != 1 {
                panic!("Unexpected cmd {cmd}");
            }

            let camera_node_index = reader.read_i16();
            let camera_node_type = reader.read_if_expected(2, |r| r.read_u8()).unwrap_or(0);

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

                        sections.push(CameraNodeSection {
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

                        sections.push(CameraNodeSection {
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
                        sections.push(CameraNodeSection {
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
                        sections.push(CameraNodeSection {
                            index: section_index,
                            bytes: section_bytes,
                        });
                    }
                }
                _ => {
                    panic!("Unknown camera_node_type {camera_node_type}");
                }
            }

            nodes.push(CameraNode {
                index: camera_node_index,
                camera_type: camera_node_type,
                sections,
            });
        }

        CameraNodeList { nodes }
    }

    fn to_yaml(&self) -> String {
        if self.nodes.is_empty() {
            return String::from("cameras: []\n");
        }

        let mut out_string = String::new();
        out_string.push_str("cameras:\n");
        for camera_node in &self.nodes {
            out_string.push_str("  -\n");
            out_string.push_str(format!("    index: {}\n", camera_node.index).as_str());
            out_string.push_str(format!("    type: {}\n", camera_node.camera_type).as_str());
            if camera_node.sections.is_empty() {
                out_string.push_str("    sections: []\n");
            } else {
                out_string.push_str("    sections:\n");
                for section in &camera_node.sections {
                    out_string.push_str(
                        format!(
                            "      - {{ section: {}, bytes: {} }}\n",
                            section.index,
                            u8s_to_byte_array_string(&section.bytes),
                        )
                        .as_str(),
                    );
                }
            }
        }

        out_string
    }

    fn from_yaml(doc: &Yaml) -> CameraNodeList {
        let yaml_camera_nodes = &doc["cameras"].as_vec().unwrap();
        let mut camera_nodes = vec![];
        for yaml_camera_node in yaml_camera_nodes.iter() {
            let index = yaml_camera_node["index"].as_i64().unwrap() as i16;
            let camera_type = yaml_camera_node["type"].as_i64().unwrap() as u8;

            let mut sections = vec![];
            for section in yaml_camera_node["sections"].as_vec().unwrap() {
                sections.push(CameraNodeSection {
                    index: section["section"].as_i64().unwrap() as u8,
                    bytes: section["bytes"]
                        .as_vec()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_i64().unwrap() as u8)
                        .collect::<Vec<u8>>(),
                });
            }
            camera_nodes.push(CameraNode {
                index,
                camera_type,
                sections,
            });
        }

        CameraNodeList {
            nodes: camera_nodes,
        }
    }
}

impl LightingNodeList {
    fn to_bytes(&self, out_bytes: &mut Vec<u8>) {
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

    fn from_reader(reader: &mut LevelSetupReader) -> LightingNodeList {
        // lightingVectorList_fromFile
        let mut nodes = vec![];

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

            nodes.push(LightingNode {
                position,
                unknown_flags,
                rgb,
            });
        }

        LightingNodeList { nodes }
    }

    fn to_yaml(&self) -> String {
        if self.nodes.is_empty() {
            return String::from("lightings: []\n");
        }

        let mut out_string = String::new();
        out_string.push_str("lightings:\n");
        for lighting_node in &self.nodes {
            out_string.push_str("    - {\n");
            out_string.push_str(
                format!(
                    "      position: {{ x: {:?}, y: {:?}, z: {:?} }},\n",
                    lighting_node.position[0], lighting_node.position[1], lighting_node.position[2]
                )
                .as_str(),
            );

            out_string.push_str(
                format!(
                    "      flags: [{}, {}],\n",
                    u8s_to_byte_array_string(&lighting_node.unknown_flags[0].to_be_bytes()),
                    u8s_to_byte_array_string(&lighting_node.unknown_flags[1].to_be_bytes()),
                )
                .as_str(),
            );

            out_string.push_str(
                format!(
                    "      rgb: \"{:02X}{:02X}{:02X}\"\n",
                    lighting_node.rgb[0], lighting_node.rgb[1], lighting_node.rgb[2],
                )
                .as_str(),
            );

            out_string.push_str("    }\n");
        }

        out_string
    }

    fn from_yaml(doc: &Yaml) -> LightingNodeList {
        let yaml_lighting_nodes = &doc["lightings"];
        let mut lighting_nodes = vec![];
        for yaml_lighting_node in yaml_lighting_nodes.as_vec().unwrap() {
            let position = [
                yaml_lighting_node["position"]["x"].as_f64().unwrap() as f32,
                yaml_lighting_node["position"]["y"].as_f64().unwrap() as f32,
                yaml_lighting_node["position"]["z"].as_f64().unwrap() as f32,
            ];

            let yaml_unknown_flags = yaml_lighting_node["flags"].as_vec().unwrap();
            let flag_0 = yaml_unknown_flags[0]
                .as_vec()
                .unwrap()
                .iter()
                .map(|x| x.as_i64().unwrap() as u8)
                .collect::<Vec<u8>>();
            let flag_1 = yaml_unknown_flags[1]
                .as_vec()
                .unwrap()
                .iter()
                .map(|x| x.as_i64().unwrap() as u8)
                .collect::<Vec<u8>>();
            let unknown_flags = [
                f32::from_be_bytes([flag_0[0], flag_0[1], flag_0[2], flag_0[3]]),
                f32::from_be_bytes([flag_1[0], flag_1[1], flag_1[2], flag_1[3]]),
            ];

            let yaml_rgb = yaml_lighting_node["rgb"].as_str().unwrap();
            let rgb = [
                u8::from_str_radix(&yaml_rgb[0..2], 16).unwrap(),
                u8::from_str_radix(&yaml_rgb[2..4], 16).unwrap(),
                u8::from_str_radix(&yaml_rgb[4..6], 16).unwrap(),
            ];

            lighting_nodes.push(LightingNode {
                position,
                unknown_flags,
                rgb,
            });
        }

        LightingNodeList {
            nodes: lighting_nodes,
        }
    }
}
