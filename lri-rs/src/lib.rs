use std::{fmt, vec::IntoIter};

use lri_proto::Message as PbMessage;
use lri_proto::{
    camera_id::CameraID,
    camera_module::{camera_module::surface::FormatType, CameraModule},
    color_calibration::color_calibration::IlluminantType,
    gps_data::GPSData,
    lightheader::LightHeader,
    view_preferences::ViewPreferences,
};

pub struct LriFile {
    pub blocks: Vec<Block>,
    pub models: Vec<SensorModel>,
}

impl LriFile {
    /// Read
    pub fn decode(mut data: Vec<u8>) -> Self {
        let mut blocks = vec![];

        loop {
            let header = Header::ingest(&data[..]);
            let end = header.combined_length as usize;

            if end == data.len() {
                blocks.push(Block { header, data });
                break;
            } else {
                let remain = data.split_off(end);
                blocks.push(Block { header, data });
                data = remain;
            }
        }

        let models = Self::grab_sensor_models(&blocks);

        Self { blocks, models }
    }

    fn grab_sensor_models(blocks: &[Block]) -> Vec<SensorModel> {
        let mut models = vec![];

        for blk in blocks {
            match blk.message() {
                Message::LightHeader(LightHeader {
                    module_calibration, ..
                }) => {
                    for mcal in module_calibration {
                        let id = mcal.camera_id().into();
                        let color = match mcal.color.first() {
                            None => continue,
                            Some(c) => c,
                        };
                        let whitepoint = color.type_().into();
                        let forward = color.forward_matrix.clone().unwrap();
                        let our_forward = [
                            forward.x00(),
                            forward.x01(),
                            forward.x02(),
                            forward.x10(),
                            forward.x11(),
                            forward.x12(),
                            forward.x20(),
                            forward.x21(),
                            forward.x22(),
                        ];

                        let forward = color.color_matrix.clone().unwrap();
                        let our_color = [
                            forward.x00(),
                            forward.x01(),
                            forward.x02(),
                            forward.x10(),
                            forward.x11(),
                            forward.x12(),
                            forward.x20(),
                            forward.x21(),
                            forward.x22(),
                        ];

                        let rg = color.rg_ratio();
                        let bg = color.bg_ratio();

                        let model = SensorModel {
                            id,
                            whitepoint,
                            forward_matrix: our_forward,
                            color_matrix: our_color,
                            rg,
                            bg,
                        };
                        models.push(model);
                    }
                }
                _ => (),
            }
        }

        models
    }

    pub fn image_count(&self) -> usize {
        let mut count = 0;

        for block in &self.blocks {
            match block.message() {
                Message::LightHeader(LightHeader { modules, .. }) => {
                    for cam in modules {
                        count += 1;
                    }
                }
                _ => (),
            }
        }

        count
    }

    pub fn images(&self) -> ImageIterator {
        ImageIterator {
            blocks: &self.blocks,
            modules: None,
        }
    }

    pub fn color_models(&self, cameraid: SensorId) -> Vec<&SensorModel> {
        self.models.iter().filter(|sm| sm.id == cameraid).collect()
    }
}

pub struct ImageIterator<'lri> {
    blocks: &'lri [Block],
    modules: Option<(&'lri Block, IntoIter<CameraModule>)>,
}

impl<'lri> Iterator for ImageIterator<'lri> {
    type Item = RawImage<'lri>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.modules.as_mut() {
                None => match self.blocks.first() {
                    None => return None,
                    Some(block) => {
                        self.blocks = &self.blocks[1..];

                        if let Message::LightHeader(lh) = block.message() {
                            let mod_iter = lh.modules.into_iter();
                            self.modules = Some((block, mod_iter));
                        } else {
                            continue;
                        }
                    }
                },
                Some((block, mods)) => {
                    for module in mods {
                        let sensor_id = module.id().into();
                        let mut surface = module.sensor_data_surface.unwrap();
                        let size = surface.size.take().unwrap();
                        let offset = surface.data_offset() as usize;
                        let data_length = surface.row_stride() as usize * size.y() as usize;

                        let data = &block.data[offset..offset + data_length];

                        return Some(RawImage {
                            sensor_id,
                            width: size.x() as usize,
                            height: size.y() as usize,
                            format: surface.format().into(),
                            data,
                        });
                    }

                    self.modules = None;
                }
            }
        }
    }
}

pub struct Block {
    pub header: Header,
    /// This includes the 32 bytes that make up the header.
    pub data: Vec<u8>,
}

impl Block {
    pub fn body(&self) -> &[u8] {
        &self.data[32..]
    }

    pub fn message_data(&self) -> &[u8] {
        let end = self.header.message_offset + self.header.message_length;
        &self.data[self.header.message_offset..end]
    }

    pub fn message(&self) -> Message {
        match self.header.kind {
            BlockType::LightHeader => {
                Message::LightHeader(LightHeader::parse_from_bytes(self.message_data()).unwrap())
            }
            BlockType::ViewPreferences => Message::ViewPreferences(
                ViewPreferences::parse_from_bytes(self.message_data()).unwrap(),
            ),
            BlockType::GPSData => {
                Message::Gps(GPSData::parse_from_bytes(self.message_data()).unwrap())
            }
        }
    }
}

pub enum Message {
    LightHeader(LightHeader),
    ViewPreferences(ViewPreferences),
    Gps(GPSData),
}

pub struct Header {
    /// The length of this header and it's associated block
    pub combined_length: usize,
    /// An offset from the start of the header to the block's protobuf message
    pub message_offset: usize,
    /// block's protobuf message length
    pub message_length: usize,
    /// The kind of protobuf message in the block
    pub kind: BlockType,
}

impl Header {
    pub fn ingest(data: &[u8]) -> Self {
        let magic = b"LELR";

        if &data[0..4] != magic {
            panic!("Magic nubmer is wrong");
        }

        let combined_length = u64::from_le_bytes(data[4..12].try_into().unwrap()) as usize;
        let message_offset = u64::from_le_bytes(data[12..20].try_into().unwrap()) as usize;
        let message_length = u32::from_le_bytes(data[20..24].try_into().unwrap()) as usize;

        let kind = match data[24] {
            0 => BlockType::LightHeader,
            1 => BlockType::ViewPreferences,
            2 => BlockType::GPSData,
            t => panic!("block type {t} is unknown"),
        };

        Header {
            combined_length,
            message_offset,
            message_length,
            kind,
        }
    }
}

pub enum BlockType {
    LightHeader,
    ViewPreferences,
    GPSData,
}

pub struct RawImage<'img> {
    pub sensor_id: SensorId,
    pub width: usize,
    pub height: usize,
    pub format: ImageFormat,
    pub data: &'img [u8],
}

pub struct SensorModel {
    pub id: SensorId,
    pub whitepoint: Whitepoint,
    /// From the camera debayered data to XYZ in the given whitepoint
    pub forward_matrix: [f32; 9],
    /// ??? is this cam -> sRGB?
    pub color_matrix: [f32; 9],
    pub rg: f32,
    pub bg: f32,
}

pub enum ImageFormat {
    // I'm not sure what this is?? Do we ever see it???
    BayerJpeg,
    Packed10bpp,
    Packed12bpp,
    Packed14bpp,
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::BayerJpeg => "BayerJpeg",
            Self::Packed10bpp => "Packed10bpp",
            Self::Packed12bpp => "Packed12bpp",
            Self::Packed14bpp => "Packed14bpp",
        };

        write!(f, "{str}")
    }
}

impl From<FormatType> for ImageFormat {
    fn from(proto: FormatType) -> Self {
        match proto {
            FormatType::RAW_BAYER_JPEG => Self::BayerJpeg,
            FormatType::RAW_PACKED_10BPP => Self::Packed10bpp,
            FormatType::RAW_PACKED_12BPP => Self::Packed12bpp,
            FormatType::RAW_PACKED_14BPP => Self::Packed14bpp,
            FormatType::RAW_RESERVED_0
            | FormatType::RAW_RESERVED_1
            | FormatType::RAW_RESERVED_2
            | FormatType::RAW_RESERVED_3
            | FormatType::RAW_RESERVED_4
            | FormatType::RAW_RESERVED_5 => unimplemented!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SensorId {
    A1,
    A2,
    A3,
    A4,
    A5,
    B1,
    B2,
    B3,
    B4,
    B5,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
}

impl From<CameraID> for SensorId {
    fn from(pbid: CameraID) -> Self {
        match pbid {
            CameraID::A1 => Self::A1,
            CameraID::A2 => Self::A2,
            CameraID::A3 => Self::A3,
            CameraID::A4 => Self::A4,
            CameraID::A5 => Self::A5,
            CameraID::B1 => Self::B1,
            CameraID::B2 => Self::B2,
            CameraID::B3 => Self::B3,
            CameraID::B4 => Self::B4,
            CameraID::B5 => Self::B5,
            CameraID::C1 => Self::C1,
            CameraID::C2 => Self::C2,
            CameraID::C3 => Self::C3,
            CameraID::C4 => Self::C4,
            CameraID::C5 => Self::C5,
            CameraID::C6 => Self::C6,
        }
    }
}

impl fmt::Display for SensorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // this is good; i write good code
        write!(f, "{self:?}")
    }
}

#[derive(Debug)]
pub enum Whitepoint {
    A,
    D50,
    D65,
    D75,
    F2,
    F7,
    F11,
    TL84,
}

impl From<IlluminantType> for Whitepoint {
    fn from(it: IlluminantType) -> Self {
        match it {
            IlluminantType::A => Self::A,
            IlluminantType::D50 => Self::D50,
            IlluminantType::D65 => Self::D65,
            IlluminantType::D75 => Self::D75,
            IlluminantType::F2 => Self::F2,
            IlluminantType::F7 => Self::F7,
            IlluminantType::F11 => Self::F11,
            IlluminantType::TL84 => Self::TL84,
            IlluminantType::UNKNOWN => unimplemented!(),
        }
    }
}
