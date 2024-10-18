use std::io::Write;
use std::{fmt, path::Path};
use std::fs::{self, File};
use std::collections::HashMap;

use super::asset::{Asset, AssetType};

// const DEBUG_MAP: &str = "SM_BANJOS_HOUSE";
const DEBUG_MAP: &str = "SM_SPIRAL_MOUNTAIN";

/// LevelSetup TODO !!!!!!!!!
///     - struct members
///     - from_bytes
///     - read
///     - to_bytes
///     - write
///
#[derive(Clone, Debug)]
pub struct LevelSetup {
    bytes: Vec<u8>,
    cubes: Vec<LevelCubes>,
    camera_nodes: Vec<CameraNode>,
    lighting_nodes: Vec<LightingNode>,
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

    pub fn u8s_to_string(in_bytes: &[u8]) -> String {
        in_bytes
            .iter()
            .map(|x| format!("{:02X}", x))
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl fmt::Display for LevelSetupReader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.in_bytes[self.offset..]
                .iter()
                .map(|x| format!("{:02X}", x))
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}

#[derive(Clone, Debug)]
struct LevelCubes {
    start_position: [i32; 3],
    end_position: [i32; 3],
    cubes: Vec<LevelCube>,
}

#[derive(Clone, Debug)]
// Objects that are given scripts to follow, typically involving multiple states.
struct ActorNode {
    x: i16,
    y: i16,
    z: i16,
    script_id: u16,
    object_id: u16,
    unknown_1: u8,
    unknown_2: u8,
    rotation: u8, // Rotation » 1 (Y Axis)
    unknown_3: u8,
    size: u16,
    current_nodes: u16,
    next_nodes: u16,
}

#[derive(Clone, Debug)]
// Objects that perform follow a script, but are limited on how long their use is.
struct TimedNode {
    x: i16,
    y: i16,
    z: i16,
    script_id: u16,
    object_id: u16,
    unknown_1: u8,
    unknown_2: u8,
    timer: u8,
    unknown_3: u8,
    size: u16,
    current_nodes: u16,
    next_nodes: u16,
}

#[derive(Clone, Debug)]
// Objects that cause another object or set of objects to perform actions.
struct ScriptNode {
    x: i16,
    y: i16,
    z: i16,
    script_id: u16,
    object_id: u16,
    unknown_1: u8,
    unknown_2: u8,
    unknown_3: u8,
    unknown_4: u8,
    unknown_5: u8,
    unknown_6: u8,
    current_nodes: u16,
    next_nodes: u16,
}

#[derive(Clone, Debug)]
// Objects that trigger when Banjo-Kazooie enter their radius.
struct RadiusNode {
    x: i16,
    y: i16,
    z: i16,
    radius: u16, // Radius » 1
    bit6: u8, // probably unused
    bit0: bool, // some sort of flag, probably unused for the radius node
    associated_id: u16, // for stuff like flags?
    unknown_1: u8,
    unknown_2: u8,
    unknown_3: u8,
    unknown_4: u8,
    unknown_5: u8,
    unknown_6: u8,
    current_nodes: u16,
    next_nodes: u16,
}

impl RadiusNode {
    fn from_bytes(bytes: &[u8]) -> RadiusNode {
        // grab first 9 bits
        let radius = (u16::from_be_bytes([bytes[6], bytes[7]]) & 0xFF80) >> 7;
        // grab 2nd to 7th bit (inclusive)
        let bit6 = (bytes[7] & 0x7F) >> 1;
        // grab last bit
        let bit0 = (bytes[7] & 0x1) == 1;
        RadiusNode {
            x: i16::from_be_bytes([bytes[0], bytes[1]]),
            y: i16::from_be_bytes([bytes[2], bytes[3]]),
            z: i16::from_be_bytes([bytes[4], bytes[5]]),
            radius,
            bit6,
            bit0,
            associated_id: u16::from_be_bytes([bytes[8], bytes[9]]), // for stuff like flags?
            unknown_1: bytes[10],
            unknown_2: bytes[11],
            unknown_3: bytes[12],
            unknown_4: bytes[13],
            unknown_5: bytes[14],
            unknown_6: bytes[15],
            current_nodes: u16::from_be_bytes([bytes[16], bytes[17]]),
            next_nodes: u16::from_be_bytes([bytes[17], bytes[18]]),
    }
    }
}

#[derive(Clone, Debug)]
// 2D Objects that will face the camera at all times (aka Billboarding).
struct SpriteNode {
    object_id: u16,
    size: u16,
    x: i16,
    y: i16,
    z: i16,
    unknown_1: u8,
    unknown_2: u8,
}

#[derive(Clone, Debug)]
// 2D Objects that will face the camera at all times (aka Billboarding).
struct StructureNode {
    object_id: u16,
    rotatation_y: u8,
    rotation_xz: u8,
    x: i16,
    y: i16,
    z: i16,
    size: u8,
    unknown_1: u8,
}

#[derive(Clone, Debug)]
enum LevelVoxelType {
    Actor(ActorNode),
    Timed(TimedNode),
    Script(ScriptNode),
    Radius(RadiusNode),
    Sprite(SpriteNode),
    Structure(StructureNode),
}

impl LevelVoxelType {
    pub fn new(bytes: &[u8]) -> LevelVoxelType {
        if bytes.len() == 20 {
            // TODO: Determine type (actor, timed, script, radius) somehow
            // for now always return actor

            let x = i16::from_be_bytes([bytes[0], bytes[1]]);
            let y = i16::from_be_bytes([bytes[2], bytes[3]]);
            let z = i16::from_be_bytes([bytes[4], bytes[5]]);

            let maybe_script_id = u16::from_be_bytes([bytes[6], bytes[7]]);
            let maybe_object_id = u16::from_be_bytes([bytes[8], bytes[9]]); // unk8
            // grab first 9 bits
            let radius = maybe_script_id >> 7;
            // grab 2nd to 7th bit (inclusive)
            let bit6 = (bytes[7] & 0x7F) >> 1;
            // grab last bit
            let bit0 = bytes[7] & 0x1 == 1;
            let unk_a = bytes[10];                  
            let unk_10 = u32::from_be_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]);
            let _unk_10_0 = unk_10 & 0x3;
            // let pad_10_5 = (unk_10 >> 2) & 0x8;
            let _unk_10_6 = (unk_10 >> 6) & 0x1 == 1;
            let _unk_10_7 = (unk_10 >> 7) & 0x1 == 1;
            let _unk_10_19 = ((unk_10 >> 8) & 0xFFF) as u16;
            let _unk_10_31 = (unk_10 >> 20) as u16;

            let unk_c = u32::from_be_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]);
            let _unk_c_22 = unk_c & 0x1FF;
            let _unk_c_31 = unk_c >> 23;

            let known_actor_objects = LevelVoxelType::get_known_actor_objects();
            let known_timed_objects = LevelVoxelType::get_known_timed_objects();
            let known_script_objects = LevelVoxelType::get_known_script_objects();

            if known_actor_objects.contains(&(maybe_object_id, maybe_script_id)) {
                // no-op
            } else if known_timed_objects.contains(&(maybe_object_id, maybe_script_id)) {
                // no-op
            } else if known_script_objects.contains(&(maybe_object_id, maybe_script_id)) {
                // no-op
            } else {
                if bit6 == 3 {
                    // there's a total of 311 warp points, however some of them are (func_8033443C) which is an empty function so probably not actually warping
                    // some others are calls to func_802C169C which calls func_802C16CC
                    // therefore: if 0 < maybe_object_id < 311 then this voxel is very likely a warp
                    if maybe_object_id < 311 {
                        return LevelVoxelType::Radius(RadiusNode::from_bytes(bytes))
                    }
                }
                if bit6 == 4 {
                    // 4 is a radius trigger for camera it seems
                    // there's a total of 251 trigger points, however some of them are (func_80334430) which is an empty function
                    return LevelVoxelType::Radius(RadiusNode::from_bytes(bytes))
                }
                if bit6 == 7 {
                    // calls func_803065E4(maybe_object_id, [x, y, z], radius, unk10_31, unk10_7) in func_803303B8
                }
                if bit6 == 9 {
                    // calls func_8030688C(maybe_object_id, [x, y, z], radius, unk10_0) in func_803303B8
                }
                if bit6 == 0xA {
                    // calls func_80306AA8(maybe_object_id, [x, y, z], radius) in func_803303B8
                }
            }

            LevelVoxelType::Actor(ActorNode {
                x,
                y,
                z,
                script_id: maybe_script_id,
                object_id: maybe_object_id,
                unknown_1: bytes[10],
                unknown_2: bytes[11],
                rotation: bytes[12],
                unknown_3: bytes[13],
                size: u16::from_be_bytes([bytes[14], bytes[15]]),
                current_nodes: _unk_10_19,
                next_nodes: _unk_10_31,
            })
        } else if bytes.len() == 12 {
            // TODO: Determine type (sprite, structure) somehow
            // for now always return sprite
            LevelVoxelType::Sprite(SpriteNode {
                object_id: u16::from_be_bytes([bytes[0], bytes[1]]),
                size: u16::from_be_bytes([bytes[2], bytes[3]]),
                x: i16::from_be_bytes([bytes[4], bytes[5]]),
                y: i16::from_be_bytes([bytes[6], bytes[7]]),
                z: i16::from_be_bytes([bytes[8], bytes[9]]),
                unknown_1: bytes[10],
                unknown_2: bytes[11],
            })
        } else {
            panic!("Can not create level cube bytes from given bytes: {bytes:?}");
        }
    }

    pub fn get_known_script_objects() -> Vec<(u16, u16)> {
        vec![
            (0x0000, 0x1910), // Path
            (0x0001, 0x190C), // Entry Point 1
            (0x0002, 0x190C), // Entry Point 2
            (0x0002, 0x1910), // Path
            (0x0012, 0x1910), // Path
            (0x0015, 0x190C), // Entry Point 3
            (0x0016, 0x190C), // Entry Point 18
            (0x0017, 0x1910), // Path
            (0x0013, 0x190C), // Crate On Water Script
            (0x0037, 0x190C), // Crate On Water Script
            (0x0071, 0x190C), // Entry Point 14
            (0x0072, 0x190C), // Entry Point 17
            (0x0075, 0x190C), // Entry Point 3
            (0x0076, 0x190C), // Entry Point 4
            (0x0076, 0x0F0C), // Entry Point 4
            (0x0077, 0x190C), // Entry Point 5
            (0x0078, 0x190C), // Entry Point 6
            (0x0079, 0x190C), // Entry Point 7
            (0x007A, 0x190C), // Entry Point 8
            (0x007B, 0x190C), // Entry Point 9
            (0x007C, 0x190C), // Entry Point 10
            (0x007D, 0x190C), // Entry Point 11
            (0x007E, 0x190C), // Entry Point 12
            (0x007F, 0x190C), // Entry Point 13
            (0x0103, 0x190C), // Entry Point 18
            (0x0104, 0x190C), // Entry Point 19
            (0x0105, 0x190C), // Entry Point
            (0x0106, 0x190C), // Entry Point
            (0x0149, 0x190C), // Where Banjo Throws Blubbers Gold Point 1
            (0x014A, 0x190C), // Where Banjo Throws Blubbers Gold Point 2
            (0x016E, 0x190C), // SM Beak Barge Tutorial Quarrie Counter Script
            (0x01B0, 0x190C), // SM To Lair 01B0
            (0x01CF, 0x190C), // SM Bridge Script
            (0x0349, 0xC30C), // SM Bridge Bottles 0349
            (0x0373, 0x008C), // Bottles Leaving MM Entrance Without All Moves Script
            (0x0373, 0x010C), // Bottles Leaving TTC Entrance Without All Moves Script
            (0x0373, 0x018C), // Bottles Leaving CC Entrance Without All Moves Script
            (0x0373, 0x020C), // Bottles Leaving BGS Entrance Without All Moves Script
            (0x0373, 0x028C), // Bottles Leaving FP Entrance Without All Moves Script
            (0x0373, 0x030C), // Bottles Leaving GV Entrance Without All Moves Script
            (0x0376, 0x190C), // Mini Game Script
            (0x0379, 0x030C), // SM Camera Tutorial Bottles 0379
            (0x0379, 0x040C), // SM Attack Tutorial Bottles 0379
            (0x0379, 0x070C), // TTC Water 0379
            (0x0379, 0x190C), // Triggered Dialog
            (0x03B9, 0x190C), // SM Jump Tutorial Bottles Script
            (0x03BA, 0x190C), // SM Bridge Bottles Invisible Wall Script
            (0x03BD, 0x820C), // SM Bridge Bottles 03BD
            (0x03BE, 0x190C), // SM Bridge Bottles 03BE
            (0x03C3, 0x190C), // Cartoony Falling Noise Script
        ]
    }

    pub fn get_known_timed_objects() -> Vec<(u16, u16)> {
        vec![
            (0x002C, 0x008C), // Running Shoes (Used for Bottle's Tutorial)
            (0x002C, 0x190C), // Running Shoes
            (0x0065, 0x008C), // Wading Boots (Used for Bottle's Tutorial)
            (0x0065, 0x190C), // Wading Boots
        ]
    }

    pub fn get_known_actor_objects() -> Vec<(u16, u16)> {
        vec![
            (0x0004, 0x190C), // Bigbutt
            (0x0005, 0x190C), // Ticker
            (0x0006, 0x190C), // Grublin
            (0x0007, 0x190C), // Mumbo
            (0x0008, 0x190C), // Conga
            (0x0009, 0x190C), // MM Hut
            (0x000A, 0x190C), // Chump
            (0x000B, 0x190C), // Shock Jump Pad
            (0x000C, 0x190C), // BGS Hut
            (0x000F, 0x190C), // Chimpy
            (0x0011, 0x190C), // Ju-Ju
            (0x0012, 0x190C), // Beehive
            (0x001E, 0x190C), // Leaky
            (0x0020, 0x190C), // Pumpkin Transformation
            (0x0021, 0x190C), // Crocodile Transformation
            (0x0022, 0x190C), // Walrus Transformation
            (0x0023, 0x190C), // Bee Transformation
            (0x0025, 0x190C), // MMM Flower Pot (Thaaaaank Youuuu)
            (0x0026, 0x168C), // Start Climb
            (0x0026, 0x190C), // Start Climb
            (0x0026, 0x1E0C), // Start Climb
            (0x0026, 0x280C), // Start Climb (Conga Tree)
            (0x0027, 0x190C), // End Climb (Can't Go Higher)
            (0x0027, 0x1E0C), // End Climb (Can't Go Higher)
            (0x0027, 0x280C), // End Climb (Conga Tree)
            (0x0028, 0x168C), // End Climb (Jump At Peak)
            (0x0028, 0x190C), // End Climb (Jump At Peak)/CC Green Drain
            (0x0028, 0x1E0C), // End Climb (Jump At Peak)
            (0x0029, 0x190C), // Orange
            (0x002A, 0x190C), // Blubber's Gold
            (0x002D, 0x0C8C), // Mumbo Token
            (0x002D, 0x190C), // Mumbo Token
            (0x002F, 0x190C), // Waterfall Smoke
            (0x0030, 0x190C), // Waterfall Smoke
            (0x0031, 0x190C), // School Of Fish
            (0x003A, 0x190C), // Motzand
            (0x003C, 0x190C), // Clanker's Key
            (0x003D, 0x190C), // CC Sawblades (Witch Switch Room)
            (0x003E, 0x190C), // Clanker's Sawblade
            (0x0041, 0x190C), // Clanker's Sawblade
            (0x0043, 0x190C), // Clanker's Bolt
            (0x0046, 0x190C), // Jiggy
            (0x0047, 0x140C), // Empty Honeycomb
            (0x0047, 0x190C), // Empty Honeycomb
            (0x0049, 0x190C), // Extra Life
            (0x0050, 0x190C), // Honeycomb
            (0x0052, 0x190C), // Blue Egg
            (0x0055, 0x190C), // Red Cross
            (0x0056, 0x190C), // Shrapnel
            (0x0057, 0x190C), // Orange Pad
            (0x005B, 0x190C), // BGS Egg 1
            (0x005E, 0x190C), // Yellow Jinjo
            (0x005F, 0x190C), // Orange Jinjo
            (0x0060, 0x190C), // Blue Jinjo
            (0x0061, 0x190C), // Pink Jinjo
            (0x0062, 0x190C), // Green Jinjo
            (0x0067, 0x008C), // Snippet
            (0x0067, 0x190C), // Snippet
            (0x0069, 0x190C), // Yum-Yum
            (0x0070, 0x190C), // Camera Entry 15
            (0x0081, 0x190C), // Camera Entry 1
            (0x0082, 0x190C), // Camera Entry 2
            (0x0083, 0x190C), // Camera Entry 3
            (0x0084, 0x190C), // Camera Entry 4
            (0x0085, 0x190C), // Camera Entry 5
            (0x0088, 0x190C), // Camera Entry 8
            (0x008C, 0x190C), // Camera Entry 12
            (0x00BC, 0x170C), // Banjo's Curtains
            (0x00BC, 0x190C), // Banjo's Curtains
            (0x00C5, 0x170C), // Chimpy's Tree Stump
            (0x00C6, 0x190C), // Where Banjo Throws Chimpys Orange
            (0x00C7, 0x008C), // Ripper
            (0x00C7, 0x190C), // Ripper
            (0x00CA, 0x190C), // Tee-Hee
            (0x00CB, 0x190C), // 1881 Barrel Top
            (0x00CC, 0x190C), // Camera Controller 1
            (0x00CD, 0x190C), // Camera Controller 2
            (0x00CE, 0x190C), // Camera Controller 3
            (0x00D0, 0x190C), // Camera Controller 5
            (0x00D1, 0x190C), // Camera Controller 6
            (0x00D5, 0x190C), // Camera Controller 10
            (0x00D7, 0x190C), // Camera Controller 12
            (0x00E4, 0x190C), // Flight Pad
            (0x00E6, 0x190C), // Gloop
            (0x00E8, 0x190C), // Tanktup
            (0x00EE, 0x190C), // BGS Egg 2
            (0x00EF, 0x190C), // BGS Egg 3
            (0x00F0, 0x190C), // BGS Egg 4
            (0x00F1, 0x190C), // BGS Leaf
            (0x00F2, 0x190C), // Black Snippet
            (0x00F5, 0x008C), // Mutie Snippet
            (0x00F5, 0x190C), // Mutie Snippet
            (0x00F6, 0x190C), // BGS Big Alligator Head
            (0x00F7, 0x190C), // Honeycomb Spawn
            (0x00F9, 0x190C), // Clanker Ring 1
            (0x00FA, 0x190C), // Clanker Ring 2
            (0x00FB, 0x190C), // Clanker Ring 3
            (0x00FC, 0x190C), // Clanker Ring 4
            (0x00FD, 0x190C), // Clanker Ring 5
            (0x00FE, 0x190C), // Clanker Ring 6
            (0x00FF, 0x190C), // Clanker Ring 7
            (0x0100, 0x190C), // Clanker Ring 8
            (0x0101, 0x190C), // Clanker's Tooth 1
            (0x0102, 0x190C), // Clanker's Tooth 2
            (0x0109, 0x190C), // MMM Tumblar Shack Door
            (0x010A, 0x190C), // MMM Mansion Door
            (0x010C, 0x190C), // MMM Locked Gate (Lock Left)
            (0x010D, 0x190C), // MMM Locked Gate (Lock Right 2)
            (0x010E, 0x0F0C), // Salty Hippo Upper Entrance Plank
            (0x0110, 0x190C), // Camera Entry 19
            (0x0111, 0x190C), // Camera Entry 20
            (0x0114, 0x190C), // MMM Church Door
            (0x0115, 0x190C), // Blubber
            (0x0116, 0x004C), // FP Button 1
            (0x0116, 0xAA8C), // FP Button 2
            (0x0116, 0xAB0C), // FP Button 3
            (0x0117, 0x190C), // Nipper
            (0x0119, 0x190C), // GV Jinxy Magic Carpet
            (0x011A, 0x190C), // GV Sarcophagus
            (0x011B, 0x190C), // Rubee
            (0x011C, 0x190C), // Histup
            (0x011D, 0x190C), // Rubee's Egg Pot
            (0x011E, 0x004C), // Grabba
            (0x01F9, 0x190C), // Cactus
            (0x0120, 0x190C), // Slappa
            (0x0121, 0x190C), // GV Jinxy Head (Inside Jinxy)
            (0x0123, 0x190C), // GV Main Magic Carpet
            (0x0124, 0x190C), // Sir Slush
            (0x0124, 0x198C), // Sir Slush
            (0x0129, 0x190C), // Red Feather
            (0x012B, 0x008C), // Bottles Mound (Start)
            (0x012B, 0x010C), // Bottles Mound (Camera)
            (0x012B, 0x018C), // Bottles Mound (Swim)
            (0x012B, 0x020C), // Bottles Mound (Attack)
            (0x012B, 0x028C), // Bottles Mound (Beak Barge)
            (0x012B, 0x030C), // Bottles Mound (Jump)
            (0x012B, 0x038C), // Bottles Mound (Climb)
            (0x012B, 0x040C), // Bottles Mound (Top Of SM)
            (0x012E, 0x190C), // Gobi 1
            (0x0130, 0x190C), // Gobi's Rock
            (0x0131, 0x190C), // Gobi 2
            (0x0132, 0x190C), // Trunker
            (0x0133, 0x190C), // Red Flibbit
            (0x0134, 0x190C), // Buzzbomb
            (0x0135, 0x190C), // Gobi 3
            (0x0137, 0x190C), // Yellow Flibbit
            (0x0139, 0x190C), // Yumblie/Grumblie
            (0x013A, 0x190C), // Mr Vile
            (0x013B, 0x190C), // Flotsam
            (0x013E, 0x190C), // TTC Lighthouse Door
            (0x013F, 0x190C), // GV Sun Switch
            (0x0142, 0x190C), // GV Star Hatch
            (0x0143, 0x190C), // GV Kazooie Door
            (0x0144, 0x190C), // GV Star Switch
            (0x0145, 0x190C), // GV Empty Honeycomb Switch
            (0x0146, 0x190C), // GV Kazooie Target
            (0x0147, 0x008C), // Ancient Ones 1
            (0x0147, 0x010C), // Ancient Ones 2
            (0x0147, 0x018C), // Ancient Ones 3
            (0x0147, 0x020C), // Ancient Ones 4
            (0x0147, 0x028C), // Ancient Ones 5
            (0x014D, 0x190C), // Jiggy Spawn (Green Jiggy Switch - Central)
            (0x014E, 0x190C), // Green Jiggy Switch (Central)
            (0x0150, 0x640C), // Conga Hard Throw Location
            (0x0151, 0x190C), // Lockup
            (0x0152, 0x190C), // Lockup
            (0x0153, 0x050C), // Lockup
            (0x0153, 0x190C), // Lockup
            (0x015F, 0x190C), // Christmas Tree
            (0x0160, 0x190C), // Boggy 1 (Dying?)
            (0x0161, 0x008C), // Boggy Race Checkpoint 1 (Right)
            (0x0161, 0x010C), // Boggy Race Checkpoint 2 (Right)
            (0x0161, 0x018C), // Boggy Race Checkpoint 3 (Right)
            (0x0161, 0x020C), // Boggy Race Checkpoint 4 (Right)
            (0x0161, 0x028C), // Boggy Race Checkpoint 5 (Right)
            (0x0161, 0x030C), // Boggy Race Checkpoint 6 (Right)
            (0x0161, 0x038C), // Boggy Race Checkpoint 7 (Right)
            (0x0161, 0x040C), // Boggy Race Checkpoint 8 (Right)
            (0x0161, 0x048C), // Boggy Race Checkpoint 9 (Right)
            (0x0161, 0x050C), // Boggy Race Checkpoint 10 (Right)
            (0x0161, 0x058C), // Boggy Race Checkpoint 11 (Right)
            (0x0161, 0x060C), // Boggy Race Checkpoint 12 (Right)
            (0x0161, 0x068C), // Boggy Race Checkpoint 13 (Right)
            (0x0161, 0x070C), // Boggy Race Checkpoint 14 (Right)
            (0x0161, 0x078C), // Boggy Race Checkpoint 15 (Right)
            (0x0161, 0x080C), // Boggy Race Checkpoint 16 (Right)
            (0x0161, 0x088C), // Boggy Race Checkpoint 17 (Right)
            (0x0161, 0x090C), // Boggy Race Checkpoint 18 (Right)
            (0x0161, 0x098C), // Boggy Race Checkpoint 19 (Right)
            (0x0161, 0x0A0C), // Boggy Race Checkpoint 20 (Right)
            (0x0161, 0x0A8C), // Boggy Race Checkpoint 21 (Right)
            (0x0161, 0x0B0C), // Boggy Race Checkpoint 22 (Right)
            (0x0161, 0x0B8C), // Boggy Race Checkpoint 23 (Right)
            (0x0161, 0x0C0C), // Boggy Race Checkpoint 24 (Right)
            (0x0161, 0x0C8C), // Boggy Race Checkpoint 25 (Right)
            (0x0161, 0x0D0C), // Boggy Race Checkpoint 26 (Right)
            (0x0161, 0x0D8C), // Boggy Race Checkpoint 27 (Right)
            (0x0161, 0x0E0C), // Boggy Race Checkpoint 28 (Right)
            (0x0161, 0x0E8C), // Boggy Race Checkpoint 29 (Right)
            (0x0161, 0x0F0C), // Boggy Race Checkpoint 30 (Right)
            (0x0161, 0x0F8C), // Boggy Race Checkpoint 31 (Right)
            (0x0161, 0x100C), // Boggy Race Checkpoint 32 (Right)
            (0x0161, 0x108C), // Boggy Race Checkpoint 33 (Right)
            (0x0161, 0x110C), // Boggy Race Checkpoint 34 (Right)
            (0x0161, 0x118C), // Boggy Race Checkpoint 35 (Right)
            (0x0161, 0x120C), // Boggy Race Checkpoint 36 (Right)
            (0x0161, 0x128C), // Boggy Race Checkpoint 37 (Right)
            (0x0161, 0x130C), // Boggy Race Checkpoint 38 (Right)
            (0x0161, 0x138C), // Boggy Race Checkpoint 39 (Right)
            (0x0162, 0x008C), // Boggy Race Checkpoint 1 (Left)
            (0x0162, 0x010C), // Boggy Race Checkpoint 2 (Left)
            (0x0162, 0x018C), // Boggy Race Checkpoint 3 (Left)
            (0x0162, 0x020C), // Boggy Race Checkpoint 4 (Left)
            (0x0162, 0x028C), // Boggy Race Checkpoint 5 (Left)
            (0x0162, 0x030C), // Boggy Race Checkpoint 6 (Left)
            (0x0162, 0x038C), // Boggy Race Checkpoint 7 (Left)
            (0x0162, 0x040C), // Boggy Race Checkpoint 8 (Left)
            (0x0162, 0x048C), // Boggy Race Checkpoint 9 (Left)
            (0x0162, 0x050C), // Boggy Race Checkpoint 10 (Left)
            (0x0162, 0x058C), // Boggy Race Checkpoint 11 (Left)
            (0x0162, 0x060C), // Boggy Race Checkpoint 12 (Left)
            (0x0162, 0x068C), // Boggy Race Checkpoint 13 (Left)
            (0x0162, 0x070C), // Boggy Race Checkpoint 14 (Left)
            (0x0162, 0x078C), // Boggy Race Checkpoint 15 (Left)
            (0x0162, 0x080C), // Boggy Race Checkpoint 16 (Left)
            (0x0162, 0x088C), // Boggy Race Checkpoint 17 (Left)
            (0x0162, 0x090C), // Boggy Race Checkpoint 18 (Left)
            (0x0162, 0x098C), // Boggy Race Checkpoint 19 (Left)
            (0x0162, 0x0A0C), // Boggy Race Checkpoint 20 (Left)
            (0x0162, 0x0A8C), // Boggy Race Checkpoint 21 (Left)
            (0x0162, 0x0B0C), // Boggy Race Checkpoint 22 (Left)
            (0x0162, 0x0B8C), // Boggy Race Checkpoint 23 (Left)
            (0x0162, 0x0C0C), // Boggy Race Checkpoint 24 (Left)
            (0x0162, 0x0C8C), // Boggy Race Checkpoint 25 (Left)
            (0x0162, 0x0D0C), // Boggy Race Checkpoint 26 (Left)
            (0x0162, 0x0D8C), // Boggy Race Checkpoint 27 (Left)
            (0x0162, 0x0E0C), // Boggy Race Checkpoint 28 (Left)
            (0x0162, 0x0E8C), // Boggy Race Checkpoint 29 (Left)
            (0x0162, 0x0F0C), // Boggy Race Checkpoint 30 (Left)
            (0x0162, 0x0F8C), // Boggy Race Checkpoint 31 (Left)
            (0x0162, 0x100C), // Boggy Race Checkpoint 32 (Left)
            (0x0162, 0x108C), // Boggy Race Checkpoint 33 (Left)
            (0x0162, 0x110C), // Boggy Race Checkpoint 34 (Left)
            (0x0162, 0x118C), // Boggy Race Checkpoint 35 (Left)
            (0x0162, 0x120C), // Boggy Race Checkpoint 36 (Left)
            (0x0162, 0x128C), // Boggy Race Checkpoint 37 (Left)
            (0x0162, 0x130C), // Boggy Race Checkpoint 38 (Left)
            (0x0162, 0x138C), // Boggy Race Checkpoint 39 (Left)
            (0x0163, 0x190C), // Nibbly
            (0x0167, 0x190C), // Colliwobble (With Empty Honeycomb)
            (0x0168, 0x190C), // CCW Winter Button
            (0x0169, 0x190C), // CCW Winter Door
            (0x016A, 0x190C), // CCW Winter Button
            (0x016B, 0x190C), // CCW Autumn Door
            (0x016C, 0x190C), // CCW Summer Button
            (0x016D, 0x190C), // CCW Summer Door
            (0x016F, 0x190C), // Quarrie
            (0x0181, 0x190C), // FP Sled 1
            (0x0182, 0x190C), // FP Sled 2
            (0x0185, 0x190C), // Banjo Walk
            (0x018B, 0x190C), // Death Plane
            (0x018F, 0x190C), // RBB Empty Honeycomb Switch
            (0x0191, 0x190C), // Secret X Barrel Top
            (0x0192, 0x190C), // Reverb
            (0x0194, 0x190C), // Activated Shock Jump Pad
            (0x019E, 0x190C), // Banjo Walk
            (0x01A3, 0x190C), // CC Tooth 1
            (0x01A4, 0x190C), // CC Tooth 2
            (0x01BF, 0x190C), // RBB Button 1
            (0x01C0, 0x190C), // RBB Button 2
            (0x01C1, 0x190C), // RBB Button 3
            (0x01C2, 0x190C), // RBB Whistle 1
            (0x01C3, 0x190C), // RBB Whistle 2
            (0x01C4, 0x190C), // RBB Whistle 3
            (0x01C6, 0x008C), // Grimlet
            (0x01C6, 0x190C), // Grimlet
            (0x01C8, 0x190C), // Snorkel
            (0x01C9, 0x190C), // RBB Anchor & Chain
            (0x01CA, 0x190C), // Rareware Flag Pole
            (0x01CC, 0x190C), // Grille Chompa
            (0x01CD, 0x190C), // MM Demo Start
            (0x01D8, 0x190C), // Blue Egg Refill
            (0x01D9, 0x190C), // Red Feather Refill
            (0x01DA, 0x190C), // Gold Feather Refill
            (0x01E2, 0x190C), // CCW Spring Button
            (0x01E9, 0x190C), // Snarebear
            (0x01E3, 0x190C), // CCW Spring Door
            (0x01E4, 0x190C), // Toots
            (0x01EA, 0x190C), // Moggy
            (0x01EB, 0x190C), // Soggy
            (0x01EC, 0x190C), // Groggy
            (0x01ED, 0x190C), // FP Blue Present (Collectable)
            (0x01EE, 0x190C), // FP Blue Present Deliver Location
            (0x01EF, 0x190C), // FP Green Present (Collectable)
            (0x01F0, 0x190C), // FP Green Present Deliver Location
            (0x01F1, 0x190C), // FP Red Present (Collectable)
            (0x01F2, 0x190C), // FP Red Present Deliver Location
            (0x01F3, 0x190C), // Wozza 1
            (0x01F6, 0x0C8C), // Empty Honeycomb Spawn (GV Cactus/RBB Boat Room)
            (0x01F7, 0x190C), // Jinxy
            (0x01FD, 0x190C), // MMM Church Door Switch
            (0x01FE, 0x190C), // MMM Locked Gate (Lock Right 2)
            (0x01FA, 0x008C), // Croctus 1
            (0x01FA, 0x010C), // Croctus 2
            (0x01FA, 0x018C), // Croctus 3
            (0x01FA, 0x020C), // Croctus 4
            (0x01FA, 0x028C), // Croctus 5
            (0x01FB, 0x190C), // Green Jiggy Switch (Maze)
            (0x01FC, 0x190C), // Jiggy Spawn (Green Jiggy Switch - Maze)
            (0x0203, 0x008C), // Note Door (50)
            (0x0203, 0x010C), // Note Door (100)
            (0x0203, 0x018C), // Note Door (260)
            (0x0203, 0x020C), // Note Door (350)
            (0x0203, 0x028C), // Note Door (450)
            (0x0203, 0x030C), // Note Door (640)
            (0x0203, 0x038C), // Note Door (765)
            (0x0203, 0x040C), // Note Door (810)
            (0x0203, 0x048C), // Note Door (828)
            (0x0203, 0x050C), // Note Door (846)
            (0x0203, 0x058C), // Note Door (864)
            (0x0203, 0x060C), // Note Door (882)
            (0x0204, 0x190C), // Witch Switch (MM)
            (0x0206, 0x190C), // Witch Switch (MMM)
            (0x0208, 0x190C), // Witch Switch (TTC)
            (0x020B, 0x190C), // Witch Switch (RBB)
            (0x020D, 0x008C), // GL Breakable Brick Wall
            (0x020D, 0x010C), // GL Breakable Brick Wall
            (0x020E, 0x190C), // MM Entrance Door
            (0x020F, 0x190C), // RBB Entrance Door
            (0x0210, 0x190C), // BGS Entrance Door
            (0x0211, 0x190C), // TTC Entrance Chest Lid
            (0x0212, 0x190C), // CC Entrance Iron Bars
            (0x0213, 0x190C), // CC Entrance BGS Puzzle Grate
            (0x0214, 0x190C), // CC Entrance BGS Puzzle Grate Button
            (0x0215, 0x190C), // CC Entrance Tall Raisable Pipe 1
            (0x0216, 0x190C), // CC Entrance Tall Raisable Pipe 2
            (0x0217, 0x190C), // CC Entrance Tall Raisable Pipe Button
            (0x0218, 0x190C), // CC Entrance Short Raisable Pipe
            (0x0219, 0x190C), // CC Entrance Short Raisable Pipe Button
            (0x021A, 0x190C), // GL Breakable Statue Grate
            (0x021B, 0x190C), // GL Breakable Statue Hat
            (0x021D, 0x190C), // RBB Door To Engine Room
            (0x0221, 0x190C), // Water Switch 1
            (0x0222, 0x190C), // Water Switch 2
            (0x0223, 0x190C), // Water Switch 3
            (0x0226, 0x190C), // GV Entrance Door
            (0x0227, 0x190C), // GL Gruntilda Head Breakable Glass Eye
            (0x0229, 0x008C), // FP House (With Chimney)
            (0x0229, 0x190C), // FP House (No Chimney)
            (0x022B, 0x190C), // FP Frozen Mumbo Hut
            (0x022C, 0x190C), // Christmas Present Stack
            (0x0230, 0x008C), // GL Floor Yellow Cobweb
            (0x0230, 0x010C), // GL Floor Yellow Cobweb
            (0x0231, 0x190C), // GL Wall Yellow Cobweb
            (0x0234, 0x190C), // CCW Entrance Door
            (0x0235, 0x190C), // FP Entrance Door (Left)
            (0x0236, 0x190C), // FP Entrance Door (Right)
            (0x0237, 0x190C), // Witch Switch (CCW)
            (0x0239, 0x190C), // Witch Switch (FP)
            (0x023B, 0x008C), // Cauldron (TTC Puzzle → FP Entrance)
            (0x023B, 0x010C), // Cauldron (FP Entrance → TTC Puzzle)
            (0x023B, 0x018C), // Cauldron (FP Entrance → RBB Entrance)
            (0x023B, 0x020C), // Cauldron (RBB Entrance → FP Entrance)
            (0x023B, 0x028C), // Cauldron (Pipe Room → CCW Entrance)
            (0x023B, 0x030C), // Cauldron (CCW Entrance → Pipe Room)
            (0x023B, 0x048C), // Cauldron (FF Start → 810 Note Door)
            (0x023B, 0x050C), // Cauldron (810 Note Door → FF Start)
            (0x023C, 0x190C), // CCW Puzzle Switch
            (0x023D, 0x028C), // Mumbo Transformation Pad (Termite)
            (0x023D, 0x050C), // Mumbo Transformation Pad (Walrus)
            (0x023D, 0x078C), // Mumbo Transformation Pad (Walrus)
            (0x023D, 0x0A0C), // Mumbo Transformation Pad (Pumpkin)
            (0x023D, 0x0C8C), // Mumbo Transformation Pad (Bee)
            (0x023F, 0x190C), // RBB Warehouse Window
            (0x0243, 0x190C), // GV Door Blocking SNS Egg
            (0x0246, 0x190C), // Flying Pad Switch
            (0x0247, 0x190C), // Temporary Flying Pad
            (0x0248, 0x190C), // Shock Jump Pad Switch
            (0x0256, 0x190C), // Witch Switch (GV)
            (0x0257, 0x190C), // Witch Switch (BGS)
            (0x025B, 0x190C), // Witch Switch (CC)
            (0x025C, 0x190C), // Sharkfood Island
            (0x025D, 0x190C), // Ice Key
            (0x025E, 0x008C), // Yellow SnS Egg
            (0x025E, 0x018C), // Green SnS Egg
            (0x025E, 0x028C), // Pink SnS Egg
            (0x025E, 0x030C), // Cyan SnS Egg
            (0x0266, 0x190C), // RBB Ship Window
            (0x0267, 0x190C), // RBB Ship Window
            (0x0268, 0x190C), // Unknown (Jombo's Favorite)
            (0x027A, 0x190C), // Tiptup
            (0x027B, 0x190C), // Tiptup Choir Member 1
            (0x027C, 0x190C), // Tiptup Choir Member 2
            (0x027D, 0x190C), // Tiptup Choir Member 3
            (0x027E, 0x190C), // Tiptup Choir Member 4
            (0x027F, 0x190C), // Tiptup Choir Member 5
            (0x0280, 0x190C), // Tiptup Choir Member 6
            (0x0285, 0x190C), // King Sandybutt Jinxy Head 1
            (0x0286, 0x190C), // King Sandybutt Jinxy Head 2
            (0x0287, 0x190C), // King Sandybutt Jinxy Head 3
            (0x0288, 0x190C), // King Sandybutt Jinxy Head Activate Area
            (0x0289, 0x190C), // Beta Vent
            (0x028A, 0x008C), // Whiplash
            (0x028A, 0x190C), // Whiplash
            (0x0292, 0x190C), // CC Sawblades (Wonderwing Room)
            (0x0296, 0x190C), // RBB Bell Buoy
            (0x0297, 0x190C), // RBB Boat Room Row Boat
            (0x0299, 0x190C), // Zubba Hive Honey Block
            (0x029C, 0x190C), // Zubba (Roaming)
            (0x029D, 0x190C), // CCW Beanstalk
            (0x029F, 0x008C), // Big Clucker
            (0x029F, 0x190C), // Big Clucker
            (0x02A1, 0x190C), // Eyrie
            (0x02A2, 0x190C), // CCW Caterpillar
            (0x02A4, 0x190C), // Boom Box (Extra Life)
            (0x02A6, 0x190C), // CCW Spring Nabnut
            (0x02A7, 0x190C), // CCW Summer Nabnut
            (0x02A8, 0x190C), // CCW Autumn Nabnut
            (0x02A9, 0x190C), // CCW Acorn
            (0x02AA, 0x190C), // CCW Spring Gnawty
            (0x02AB, 0x190C), // CCW Summer Gnawty
            (0x02AC, 0x190C), // Gnawty's Boulder
            (0x02DB, 0x038C), // Dingpot
            (0x02DE, 0x668C), // Gnawty's Den
            (0x02E2, 0x668C), // Lighthouse
            (0x02E3, 0x008C), // World Entrance Sign (MM)
            (0x02E3, 0x010C), // World Entrance Sign (TTC)
            (0x02E3, 0x018C), // World Entrance Sign (CC)
            (0x02E3, 0x020C), // World Entrance Sign (BGS)
            (0x02E3, 0x028C), // World Entrance Sign (FP)
            (0x02E3, 0x030C), // World Entrance Sign (GV)
            (0x02E3, 0x038C), // World Entrance Sign (MMM)
            (0x02E3, 0x040C), // World Entrance Sign (RBB)
            (0x02E3, 0x048C), // World Entrance Sign (CCW)
            (0x02E4, 0x008C), // MM Start Pad
            (0x02E4, 0x010C), // TTC Start Pad
            (0x02E4, 0x018C), // CC Start Pad
            (0x02E4, 0x020C), // BGS Start Pad
            (0x02E4, 0x028C), // GV Start Pad
            (0x02E4, 0x030C), // BGS Start Pad
            (0x02E4, 0x038C), // MMM Start Pad
            (0x02E4, 0x040C), // RBB Start Pad
            (0x02E4, 0x048C), // CCW Start Pad
            (0x02E5, 0x190C), // Door Of Grunty
            (0x02E7, 0x190C), // CCW Door To Whipcrack Room
            (0x02E8, 0x190C), // MMM Window (Small)
            (0x02E9, 0x190C), // MMM Window (Wide)
            (0x02EA, 0x190C), // MMM Window (Tall)
            (0x02F4, 0x190C), // Roysten
            (0x02F5, 0x190C), // Cuckoo Clock
            (0x030D, 0x190C), // Boom Box
            (0x030F, 0x008C), // Whipcrack
            (0x030F, 0x190C), // Whipcrack
            (0x0311, 0x190C), // CCW Winter Nabnut's Girlfriend
            (0x0312, 0x190C), // CCW Winter Nabnut's Sheets
            (0x0315, 0x190C), // CCW Winter Nabnut (Top Half)
            (0x031A, 0x190C), // CCW Autumn/Winter Gnawty
            (0x031D, 0x190C), // King Sandybutt's Tomb
            (0x033A, 0x190C), // Blue Present (Delivered)
            (0x033B, 0x190C), // Green Present (Delivered)
            (0x033C, 0x190C), // Red Present (Delivered)
            (0x033D, 0x190C), // Boggy 3 (Igloo?)
            (0x033F, 0x190C), // Wozza 2
            (0x0340, 0x0A0C), // FP Glass Tree Case
            (0x0348, 0x008C), // Brentilda (GL 2)
            (0x0348, 0x010C), // Brentilda (GL 3)
            (0x0348, 0x018C), // Brentilda (GL BGS)
            (0x0348, 0x020C), // Brentilda (GL 6)
            (0x0348, 0x028C), // Brentilda (GL Lava)
            (0x0348, 0x030C), // Brentilda (GL 5)
            (0x0348, 0x038C), // Brentilda (GL 4)
            (0x0348, 0x040C), // Brentilda (GL CCW)
            (0x0348, 0x048C), // Brentilda (GL MMM)
            (0x0348, 0x050C), // Brentilda (GL CC)
            (0x034D, 0x078C), // Bee Swarm
            (0x034D, 0x0A0C), // Bee Swarm
            (0x034D, 0x0C8C), // Bee Swarm
            (0x034E, 0x190C), // Limbo
            (0x034F, 0x190C), // Mum-Mum
            (0x0350, 0x190C), // Seaman Grublin
            (0x0354, 0x190C), // Water Droplet Spawner
            (0x0355, 0x190C), // FP Purple Ice Crystals
            (0x0356, 0x190C), // FP Green Ice Crystals
            (0x0357, 0x190C), // FP Blue Ice
            (0x0361, 0x190C), // Boggy Race Start (Right Pole)
            (0x0362, 0x190C), // Boggy Race Start Flag
            (0x0363, 0x190C), // Boggy Race Finish Flag
            (0x0364, 0x190C), // Boggy Race Rostrum
            (0x0367, 0x190C), // Red Gruntling
            (0x0365, 0x190C), // Boggy Race Start (Left Pole)
            (0x0366, 0x190C), // Boggy Race Finish (Left Pole)
            (0x0368, 0x190C), // Mumbo Sign 5
            (0x0369, 0x190C), // Mumbo Sign 20
            (0x036A, 0x190C), // Mumbo Sign 15
            (0x036B, 0x190C), // Mumbo Sign 10
            (0x036C, 0x190C), // Mumbo Sign 25
            (0x036D, 0x190C), // Colliwobble
            (0x036E, 0x190C), // Bawl
            (0x036F, 0x190C), // Topper
            (0x0370, 0x190C), // Gold Feather
            (0x0372, 0x190C), // Banjo Placement When Learning Move
            (0x0375, 0x190C), // Grublin Hood
            (0x037A, 0x048C), // Bottles Mound (Beak Bomb)
            (0x037A, 0x050C), // Bottles Mound (Eggs)
            (0x037A, 0x058C), // Bottles Mound (Beak Buster)
            (0x037A, 0x060C), // Bottles Mound (Talon Trot)
            (0x037A, 0x068C), // Bottles Mound (Shock Spring Jump)
            (0x037A, 0x070C), // Bottles Mound (Flight)
            (0x037A, 0x078C), // Bottles Mound (Wonderwing)
            (0x037A, 0x080C), // Bottles Mound (Stilt Stride)
            (0x037A, 0x088C), // Bottles Mound (Turbo Talon Trot)
            (0x037A, 0x090C), // Bottles Mound (Note Door)
            (0x037B, 0x190C), // Boggy Race Finish (Right Pole)
            (0x037D, 0x010C), // Chinker
            (0x037D, 0x190C), // Chinker
            (0x037E, 0x190C), // Dead Snarebear
            (0x037F, 0x190C), // Loggo
            (0x0380, 0x190C), // Scabby
            (0x0381, 0x190C), // Portrait Chompa
            (0x0381, 0x1A0C), // Portrait Chompa
            (0x0381, 0x1A8C), // Portrait Chompa
            (0x0381, 0x1B0C), // Portrait Chompa
            (0x0383, 0x020C), // Fire Pain Object
            (0x0383, 0x188C), // Fire Pain Object
            (0x0383, 0x190C), // Fire Pain Object
            (0x0387, 0x190C), // Portrait (Tree)
            (0x038B, 0x190C), // Fire Pain Object
            (0x03B7, 0x008C), // Jigsaw Podium (MM)
            (0x03B7, 0x018C), // Jigsaw Podium (CC)
            (0x03B7, 0x020C), // Jigsaw Podium (BGS)
            (0x03B7, 0x028C), // Jigsaw Podium (FP)
            (0x03B7, 0x030C), // Jigsaw Podium (GV)
            (0x03B7, 0x038C), // Jigsaw Podium (MMM)
            (0x03B7, 0x040C), // Jigsaw Podium (RBB)
            (0x03B7, 0x048C), // Jigsaw Podium (CCW)
            (0x03B7, 0x050C), // Jigsaw Podium (Door Of Grunty)
            (0x03BC, 0x010C), // Jigsaw Podium (TTC)
            (0x03BF, 0x190C), // Blue Gruntling
            (0x03C0, 0x190C), // Black Gruntling
            (0x03C1, 0x190C), // Purple Tee-Hee
            (0x03C2, 0x190C), // Ripper
        ]
    }
}

#[derive(Clone, Debug)]
struct LevelCube {
    bytes: LevelVoxelType,
}

#[derive(Clone, Debug)]
struct CameraNode {
    index: i16,
    node_type: u8, // 1-4
    node_data: Vec<NodeDataTypes>,
}

#[derive(Clone, Debug)]
enum NodeDataTypes {
    NodeData1(NodeData1),
    NodeData2(NodeData2),
    NodeData3(NodeData3),
    NodeData4(NodeData4),
}

#[derive(Clone, Debug)]
struct NodeData1 {
    position: Option<[f32; 3]>,
    horizontal_speed: Option<f32>,
    vertical_speed: Option<f32>,
    rotation: Option<f32>,
    accelaration: Option<f32>,
    pitch_yaw_and_roll: Option<[f32; 3]>,
    unknown: Option<i32>, // word
}

#[derive(Clone, Debug)]
struct NodeData2 {
    position: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
}

#[derive(Clone, Debug)]
struct NodeData3 {
    position: Option<[f32; 3]>,
    horizontal_speedd: Option<f32>,
    vertical_speed: Option<f32>,
    rotation: Option<f32>,
    accelaration: Option<f32>,
    close_distance: Option<f32>,
    far_distance: Option<f32>,
    pitch_yaw_roll: Option<[f32; 3]>,
    unknown: Option<i32>, // word
}

#[derive(Clone, Debug)]
struct NodeData4 {
    unknown: Option<i32>, // word
}

#[derive(Clone, Debug)]
struct LightingNode {
    position: Option<[f32; 3]>,
    // some unknown factor which is used to calculate an RGB modifier to modify the vertex RGB
    // goes up in steps of 0.125
    unknown_1: Option<f32>,
    // some unknown factor which is used to calculate an RGB modifier to modify the vertex RGB
    // goes up in steps of 0.125
    // it looks like this also determines the minimum distance between lighting position and vertex position as to where the rgb modification should take place
    unknown_2: Option<f32>,
    rgb: Option<[u8; 3]>,
}

impl LevelSetup {
    pub fn from_bytes(in_bytes: &[u8], i: usize) -> LevelSetup {
        let map_id_offset = 1820;
        let map_idx = i - map_id_offset;

        let maps = LevelSetup::build_map_hash_set();
        let _map_name = maps
            .get(&map_idx)
            .unwrap_or_else(|| panic!("Expected {map_idx} to exist in maps"));

        // Skip this file as it currently fails to parse
        if map_idx == 113 || _map_name.as_str().ne(DEBUG_MAP) {
            return LevelSetup {
                bytes: in_bytes.to_vec(),
                cubes: vec![],
                camera_nodes: vec![],
                lighting_nodes: vec![],
            };
        }

        let mut level_cubes = vec![];
        let mut camera_nodes = vec![];
        let mut lighting_nodes = vec![];
        let mut reader = LevelSetupReader::new(in_bytes);
        loop {
            let cmd = reader.read_u8();
            match cmd {
                0 => {
                    break;
                }
                1 => {
                    // cubeList_fromFile
                    // file_getNWords_ifExpected(file_ptr, 1, sp50, 3)
                    let mut cubes_from: [i32; 3] = [0, 0, 0];
                    let should_read = reader.read_u8();
                    if should_read == 1 {
                        cubes_from = [reader.read_i32(), reader.read_i32(), reader.read_i32()];
                    } else {
                        println!("Not reading cubes-from");
                    }

                    // file_getNWords(file_ptr, sp44, 3)
                    let cubes_to = [reader.read_i32(), reader.read_i32(), reader.read_i32()];

                    for _x in cubes_from[0]..=cubes_to[0] {
                        for _y in cubes_from[1]..=cubes_to[1] {
                            for _z in cubes_from[2]..=cubes_to[2] {
                                let cubes = LevelSetup::get_cubes_from_reader(&mut reader);

                                if !cubes.is_empty() {
                                    level_cubes.push(LevelCubes {
                                        start_position: cubes_from,
                                        end_position: cubes_to,
                                        cubes,
                                    });
                                }
                            }
                        }
                    }

                    // in the c code after the for loops there is:
                    // file_isNextByteExpected(file_ptr, 0);
                    // which, in essence, advances the file_ptr by 1 if the current value is 0
                    reader.read_if_expected(0, |_| 0);
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
                        let mut node_data = vec![];

                        match camera_node_type {
                            0 => break,
                            1 => {
                                // cameraNodeType1_fromFile
                                let mut node_data_type_1 = NodeData1 {
                                    position: None,
                                    horizontal_speed: None,
                                    vertical_speed: None,
                                    rotation: None,
                                    accelaration: None,
                                    pitch_yaw_and_roll: None,
                                    unknown: None,
                                };

                                loop {
                                    match reader.read_u8() {
                                        0 => break,
                                        1 => {
                                            node_data_type_1.position = Some([
                                                reader.read_f32(),
                                                reader.read_f32(),
                                                reader.read_f32(),
                                            ]);
                                        }
                                        2 => {
                                            node_data_type_1.horizontal_speed =
                                                Some(reader.read_f32());
                                            node_data_type_1.vertical_speed =
                                                Some(reader.read_f32());
                                        }
                                        3 => {
                                            node_data_type_1.rotation = Some(reader.read_f32());
                                            node_data_type_1.accelaration = Some(reader.read_f32());
                                        }
                                        4 => {
                                            node_data_type_1.pitch_yaw_and_roll = Some([
                                                reader.read_f32(),
                                                reader.read_f32(),
                                                reader.read_f32(),
                                            ]);
                                        }
                                        5 => {
                                            node_data_type_1.unknown = Some(reader.read_word());
                                        }
                                        _ => panic!("Unknown section = {cmd}"),
                                    }
                                }

                                node_data.push(NodeDataTypes::NodeData1(node_data_type_1));
                            }
                            2 => {
                                // cameraNodeType2_fromFile
                                let mut node_data_type_2 = NodeData2 {
                                    position: None,
                                    rotation: None,
                                };

                                loop {
                                    match reader.read_u8() {
                                        0 => break,
                                        1 => {
                                            node_data_type_2.position = Some([
                                                reader.read_f32(),
                                                reader.read_f32(),
                                                reader.read_f32(),
                                            ]);
                                        }
                                        2 => {
                                            node_data_type_2.rotation = Some([
                                                reader.read_f32(),
                                                reader.read_f32(),
                                                reader.read_f32(),
                                            ]);
                                        }
                                        _ => panic!("Unknown section = {cmd}"),
                                    }
                                }

                                node_data.push(NodeDataTypes::NodeData2(node_data_type_2));
                            }
                            3 => {
                                // cameraNodeType3_fromFile
                                let mut node_data_type_3 = NodeData3 {
                                    position: None,
                                    horizontal_speedd: None,
                                    vertical_speed: None,
                                    rotation: None,
                                    accelaration: None,
                                    close_distance: None,
                                    far_distance: None,
                                    pitch_yaw_roll: None,
                                    unknown: None,
                                };

                                loop {
                                    match reader.read_u8() {
                                        0 => break,
                                        1 => {
                                            node_data_type_3.position = Some([
                                                reader.read_f32(),
                                                reader.read_f32(),
                                                reader.read_f32(),
                                            ]);
                                        }
                                        2 => {
                                            node_data_type_3.horizontal_speedd =
                                                Some(reader.read_f32());
                                            node_data_type_3.vertical_speed =
                                                Some(reader.read_f32());
                                        }
                                        3 => {
                                            node_data_type_3.rotation = Some(reader.read_f32());
                                            node_data_type_3.accelaration = Some(reader.read_f32());
                                        }
                                        4 => {
                                            node_data_type_3.pitch_yaw_roll = Some([
                                                reader.read_f32(),
                                                reader.read_f32(),
                                                reader.read_f32(),
                                            ]);
                                        }
                                        5 => {
                                            node_data_type_3.unknown = Some(reader.read_word());
                                        }
                                        6 => {
                                            node_data_type_3.close_distance =
                                                Some(reader.read_f32());
                                            node_data_type_3.far_distance = Some(reader.read_f32());
                                        }
                                        _ => panic!("Unknown section = {cmd}"),
                                    }
                                }

                                node_data.push(NodeDataTypes::NodeData3(node_data_type_3));
                            }
                            4 => {
                                // cameraNodeType4_fromFile
                                let mut node_data_type_4 = NodeData4 { unknown: None };

                                loop {
                                    match reader.read_u8() {
                                        0 => break,
                                        1 => {
                                            node_data_type_4.unknown = Some(reader.read_i32());
                                        }
                                        _ => panic!("Unknown Cmd = {cmd}"),
                                    }
                                }

                                node_data.push(NodeDataTypes::NodeData4(node_data_type_4));
                            }
                            _ => {
                                panic!("Unknown camera_node_type {camera_node_type}");
                            }
                        }

                        camera_nodes.push(CameraNode {
                            index: camera_node_index,
                            node_type: camera_node_type,
                            node_data,
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
                        let read_data = reader.read_if_expected(2, |r| {
                            let position = [r.read_f32(), r.read_f32(), r.read_f32()];

                            // file_getNFloats_ifExpected(file_ptr, 3, unknown_flags, 2)
                            if let Some((unknown_flags, rgb)) = r.read_if_expected(3, |r| {
                                let unknown_flags = [r.read_f32(), r.read_f32()];

                                // file_getNWords_ifExpected(file_ptr, 4, rgb, 3)
                                if let Some(rgb) = r.read_if_expected(4, |r| {
                                    // the C code reads words, however only the last byte of the 4 read bytes contain the RGB hex value (0-255)
                                    let rgb = r.read_n(12, |r| r.read_u8());

                                    [rgb[3], rgb[7], rgb[11]]
                                }) {
                                    (unknown_flags, rgb)
                                } else {
                                    (unknown_flags, [0_u8, 0_u8, 0_u8])
                                }
                            }) {
                                (position, unknown_flags, rgb)
                            } else {
                                (position, [0_f32, 0_f32], [0_u8, 0_u8, 0_u8])
                            }
                        });

                        if let Some((position, unknown_flags, rgb)) = read_data {
                            lighting_nodes.push(LightingNode {
                                position: Some(position),
                                unknown_1: Some(unknown_flags[0]),
                                unknown_2: Some(unknown_flags[1]),
                                rgb: Some(rgb),
                            });
                        }
                    }
                }
                _ => {
                    todo!("Implement cmd {cmd}?");
                }
            }
        }

        LevelSetup {
            bytes: in_bytes.to_vec(),
            cubes: level_cubes,
            camera_nodes,
            lighting_nodes,
        }
    }

    fn get_cubes_from_reader(reader: &mut LevelSetupReader) -> Vec<LevelCube> {
        let mut out_cubes = vec![];

        loop {
            let cmd = reader.read_u8();

            match cmd {
                0 => {
                    /*
                    if (file_getNWords_ifExpected(file_ptr, 0, sp2C, 3)) {
                        file_getNWords(file_ptr, sp2C, 3);
                    */
                    reader.read_n(6, |r| r.read_word());
                }
                1 => {
                    return out_cubes;
                }
                2 => {
                    /*
                    !file_getNWords_ifExpected(file_ptr, 2, &sp2C, 3)
                    */
                    todo!("Cmd = 2");
                }
                3 => {
                    // ->code7AF80_initCubeFromFile
                    let cube_type = reader.read_u8();
                    let count: usize = reader.read_u8().into();

                    let next_expected = match cube_type {
                        0xA => 0xB,
                        0x6 => 0x7,
                        _ => panic!("Unsupported cude type ? cube_type = {cube_type}"),
                    };

                    let cube_byte_size = 20; // sizeof(NodeProp)
                    let cubes = reader
                        .read_if_expected(next_expected, |r| r.read_u8_n(count * cube_byte_size));

                    if let Some(cubes) = cubes {
                        cubes.chunks(cube_byte_size).for_each(|cube| {
                            out_cubes.push(LevelCube {
                                bytes: LevelVoxelType::new(cube),
                            });
                        });
                    }
                }
                8 => {
                    let count: usize = reader.read_u8().into();

                    let cube_byte_size = 12; // sizeof(OtherNode)
                    let cubes = reader.read_if_expected(9, |r| r.read_u8_n(count * cube_byte_size));

                    if let Some(cubes) = cubes {
                        cubes.chunks(cube_byte_size).for_each(|cube| {
                            out_cubes.push(LevelCube {
                                bytes: LevelVoxelType::new(cube),
                            });
                        });
                    }
                }
                _ => {
                    todo!(
                        "Unknown cmd {cmd} {}",
                        LevelSetupReader::u8s_to_string(&reader.read_u8_n(50))
                    );
                }
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
        hs.insert(0x10, String::from("BGS_MR_VILE"));
        hs.insert(0x11, String::from("BGS_TIPTUP"));
        hs.insert(0x12, String::from("GV_GOBIS_VALLEY"));
        hs.insert(0x13, String::from("GV_MEMORY_GAME"));
        hs.insert(0x14, String::from("GV_SANDYBUTTS_MAZE"));
        hs.insert(0x15, String::from("GV_WATER_PYRAMID"));
        hs.insert(0x16, String::from("GV_RUBEES_CHAMBER"));
        hs.insert(0x1A, String::from("GV_INSIDE_JINXY"));
        hs.insert(0x1B, String::from("MMM_MAD_MONSTER_MANSION"));
        hs.insert(0x1C, String::from("MMM_CHURCH"));
        hs.insert(0x1D, String::from("MMM_CELLAR"));
        hs.insert(0x1E, String::from("CS_START_NINTENDO"));
        hs.insert(0x1F, String::from("CS_START_RAREWARE"));
        hs.insert(0x20, String::from("CS_END_NOT_100"));
        hs.insert(0x21, String::from("CC_WITCH_SWITCH_ROOM"));
        hs.insert(0x22, String::from("CC_INSIDE_CLANKER"));
        hs.insert(0x23, String::from("CC_GOLDFEATHER_ROOM"));
        hs.insert(0x24, String::from("MMM_TUMBLARS_SHED"));
        hs.insert(0x25, String::from("MMM_WELL"));
        hs.insert(0x26, String::from("MMM_NAPPERS_ROOM"));
        hs.insert(0x27, String::from("FP_FREEZEEZY_PEAK"));
        hs.insert(0x28, String::from("MMM_EGG_ROOM"));
        hs.insert(0x29, String::from("MMM_NOTE_ROOM"));
        hs.insert(0x2A, String::from("MMM_FEATHER_ROOM"));
        hs.insert(0x2B, String::from("MMM_SECRET_CHURCH_ROOM"));
        hs.insert(0x2C, String::from("MMM_BATHROOM"));
        hs.insert(0x2D, String::from("MMM_BEDROOM"));
        hs.insert(0x2E, String::from("MMM_HONEYCOMB_ROOM"));
        hs.insert(0x2F, String::from("MMM_WATERDRAIN_BARREL"));
        hs.insert(0x30, String::from("MMM_MUMBOS_SKULL"));
        hs.insert(0x31, String::from("RBB_RUSTY_BUCKET_BAY"));
        hs.insert(0x34, String::from("RBB_ENGINE_ROOM"));
        hs.insert(0x35, String::from("RBB_WAREHOUSE"));
        hs.insert(0x36, String::from("RBB_BOATHOUSE"));
        hs.insert(0x37, String::from("RBB_CONTAINER_1"));
        hs.insert(0x38, String::from("RBB_CONTAINER_3"));
        hs.insert(0x39, String::from("RBB_CREW_CABIN"));
        hs.insert(0x3A, String::from("RBB_BOSS_BOOM_BOX"));
        hs.insert(0x3B, String::from("RBB_STORAGE_ROOM"));
        hs.insert(0x3C, String::from("RBB_KITCHEN"));
        hs.insert(0x3D, String::from("RBB_NAVIGATION_ROOM"));
        hs.insert(0x3E, String::from("RBB_CONTAINER_2"));
        hs.insert(0x3F, String::from("RBB_CAPTAINS_CABIN"));
        hs.insert(0x40, String::from("CCW_HUB"));
        hs.insert(0x41, String::from("FP_BOGGYS_IGLOO"));
        hs.insert(0x43, String::from("CCW_SPRING"));
        hs.insert(0x44, String::from("CCW_SUMMER"));
        hs.insert(0x45, String::from("CCW_AUTUMN"));
        hs.insert(0x46, String::from("CCW_WINTER"));
        hs.insert(0x47, String::from("BGS_MUMBOS_SKULL"));
        hs.insert(0x48, String::from("FP_MUMBOS_SKULL"));
        hs.insert(0x4A, String::from("CCW_SPRING_MUMBOS_SKULL"));
        hs.insert(0x4B, String::from("CCW_SUMMER_MUMBOS_SKULL"));
        hs.insert(0x4C, String::from("CCW_AUTUMN_MUMBOS_SKULL"));
        hs.insert(0x4D, String::from("CCW_WINTER_MUMBOS_SKULL"));
        hs.insert(0x53, String::from("FP_CHRISTMAS_TREE"));
        hs.insert(0x5A, String::from("CCW_SUMMER_ZUBBA_HIVE"));
        hs.insert(0x5B, String::from("CCW_SPRING_ZUBBA_HIVE"));
        hs.insert(0x5C, String::from("CCW_AUTUMN_ZUBBA_HIVE"));
        hs.insert(0x5E, String::from("CCW_SPRING_NABNUTS_HOUSE"));
        hs.insert(0x5F, String::from("CCW_SUMMER_NABNUTS_HOUSE"));
        hs.insert(0x60, String::from("CCW_AUTUMN_NABNUTS_HOUSE"));
        hs.insert(0x61, String::from("CCW_WINTER_NABNUTS_HOUSE"));
        hs.insert(0x62, String::from("CCW_WINTER_HONEYCOMB_ROOM"));
        hs.insert(0x63, String::from("CCW_AUTUMN_NABNUTS_WATER_SUPPLY"));
        hs.insert(0x64, String::from("CCW_WINTER_NABNUTS_WATER_SUPPLY"));
        hs.insert(0x65, String::from("CCW_SPRING_WHIPCRACK_ROOM"));
        hs.insert(0x66, String::from("CCW_SUMMER_WHIPCRACK_ROOM"));
        hs.insert(0x67, String::from("CCW_AUTUMN_WHIPCRACK_ROOM"));
        hs.insert(0x68, String::from("CCW_WINTER_WHIPCRACK_ROOM"));
        hs.insert(0x69, String::from("GL_MM_LOBBY"));
        hs.insert(0x6A, String::from("GL_TTC_AND_CC_PUZZLE"));
        hs.insert(0x6B, String::from("GL_180_NOTE_DOOR"));
        hs.insert(0x6C, String::from("GL_RED_CAULDRON_ROOM"));
        hs.insert(0x6D, String::from("GL_TTC_LOBBY"));
        hs.insert(0x6E, String::from("GL_GV_LOBBY"));
        hs.insert(0x6F, String::from("GL_FP_LOBBY"));
        hs.insert(0x70, String::from("GL_CC_LOBBY"));
        hs.insert(0x71, String::from("GL_STATUE_ROOM"));
        hs.insert(0x72, String::from("GL_BGS_LOBBY"));
        hs.insert(0x73, String::from("UNKNOWN"));
        hs.insert(0x74, String::from("GL_GV_PUZZLE"));
        hs.insert(0x75, String::from("GL_MMM_LOBBY"));
        hs.insert(0x76, String::from("GL_640_NOTE_DOOR"));
        hs.insert(0x77, String::from("GL_RBB_LOBBY"));
        hs.insert(0x78, String::from("GL_RBB_AND_MMM_PUZZLE"));
        hs.insert(0x79, String::from("GL_CCW_LOBBY"));
        hs.insert(0x7A, String::from("GL_CRYPT"));
        hs.insert(0x7B, String::from("CS_INTRO_GL_DINGPOT_1"));
        hs.insert(0x7C, String::from("CS_INTRO_BANJOS_HOUSE_1"));
        hs.insert(0x7D, String::from("CS_SPIRAL_MOUNTAIN_1"));
        hs.insert(0x7E, String::from("CS_SPIRAL_MOUNTAIN_2"));
        hs.insert(0x7F, String::from("FP_WOZZAS_CAVE"));
        hs.insert(0x80, String::from("GL_FF_ENTRANCE"));
        hs.insert(0x81, String::from("CS_INTRO_GL_DINGPOT_2"));
        hs.insert(0x82, String::from("CS_ENTERING_GL_MACHINE_ROOM"));
        hs.insert(0x83, String::from("CS_GAME_OVER_MACHINE_ROOM"));
        hs.insert(0x84, String::from("CS_UNUSED_MACHINE_ROOM"));
        hs.insert(0x85, String::from("CS_SPIRAL_MOUNTAIN_3"));
        hs.insert(0x86, String::from("CS_SPIRAL_MOUNTAIN_4"));
        hs.insert(0x87, String::from("CS_SPIRAL_MOUNTAIN_5"));
        hs.insert(0x88, String::from("CS_SPIRAL_MOUNTAIN_6"));
        hs.insert(0x89, String::from("CS_INTRO_BANJOS_HOUSE_2"));
        hs.insert(0x8A, String::from("CS_INTRO_BANJOS_HOUSE_3"));
        hs.insert(0x8B, String::from("RBB_ANCHOR_ROOM"));
        hs.insert(0x8C, String::from("SM_BANJOS_HOUSE"));
        hs.insert(0x8D, String::from("MMM_INSIDE_LOGGO"));
        hs.insert(0x8E, String::from("GL_FURNACE_FUN"));
        hs.insert(0x8F, String::from("TTC_SHARKFOOD_ISLAND"));
        hs.insert(0x90, String::from("GL_BATTLEMENTS"));
        hs.insert(0x91, String::from("FILE_SELECT"));
        hs.insert(0x92, String::from("GV_SNS_CHAMBER"));
        hs.insert(0x93, String::from("GL_DINGPOT"));
        hs.insert(0x94, String::from("CS_INTRO_SPIRAL_7"));
        hs.insert(0x95, String::from("CS_END_ALL_100"));
        hs.insert(0x96, String::from("CS_END_BEACH_1"));
        hs.insert(0x97, String::from("CS_END_BEACH_2"));
        hs.insert(0x98, String::from("CS_END_SPIRAL_MOUNTAIN_1"));
        hs.insert(0x99, String::from("CS_END_SPIRAL_MOUNTAIN_"));

        hs
    }

    pub fn read(path: &Path) -> LevelSetup {
        LevelSetup {
            bytes: fs::read(path).unwrap(),
            cubes: vec![],
            camera_nodes: vec![],
            lighting_nodes: vec![],
        }
    }
}

impl Asset for LevelSetup {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn get_type(&self) -> AssetType {
        AssetType::LevelSetup
    }

    fn write(&self, path: &Path) {
        let mut bin_file = File::create(path).unwrap();
        bin_file.write_all(&self.bytes).unwrap();
    }
}