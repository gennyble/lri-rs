use lri_proto::{
	gps_data::GPSData, lightheader::LightHeader, matrix3x3f::Matrix3x3F,
	view_preferences::ViewPreferences, Message as PbMessage,
};

use crate::{CameraId, CameraInfo, ColorInfo, RawImage, SensorModel};

pub(crate) struct Block<'lri> {
	pub header: Header,
	/// This includes the 32 bytes that make up the header.
	pub data: &'lri [u8],
}

impl<'lri> Block<'lri> {
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

	pub fn extract_meaningful_data(
		&self,
		images: &mut Vec<RawImage<'lri>>,
		colors: &mut Vec<ColorInfo>,
		infos: &mut Vec<CameraInfo>,
	) -> ExtractedData {
		let mut ext = ExtractedData {
			reference_camera: None,
		};

		let LightHeader {
			mut hw_info,
			mut module_calibration,
			mut modules,
			mut image_reference_camera,
			..
		} = if let Message::LightHeader(lh) = self.message() {
			lh
		} else {
			return ext;
		};

		// Form the CameraInfo struct for mapping CameraId to SensorType
		if let Some(hw_info) = hw_info.take() {
			for info in hw_info.camera {
				let info = CameraInfo {
					camera: info.id().into(),
					sensor: info.sensor().into(),
				};

				infos.push(info);
			}
		}

		// Color information for the Camera moduels.
		for mcal in module_calibration {
			let camera = mcal.camera_id().into();

			for mut color in mcal.color {
				let whitepoint = color.type_().into();
				let forward_matrix = match color.forward_matrix.take() {
					Some(fw) => Self::deconstruct_matrix3x3(fw),
					// The forward matrix is like, what we want! If we don't get it, don't bother
					// with the struct
					None => continue,
				};
				let color_matrix = match color.color_matrix.take() {
					None => [0.0; 9],
					Some(cm) => Self::deconstruct_matrix3x3(cm),
				};

				let rg = color.rg_ratio();
				let bg = color.bg_ratio();

				colors.push(ColorInfo {
					camera,
					whitepoint,
					forward_matrix,
					color_matrix,
					rg,
					bg,
				})
			}
		}

		// The images themselves
		for mut module in modules {
			let camera = module.id().into();
			let mut surface = match module.sensor_data_surface.take() {
				Some(sur) => sur,
				// The surface is what we're after here. Don't bother with anything lacking it
				None => continue,
			};

			let size = surface.size.take().unwrap();
			let width = size.x() as usize;
			let height = size.y() as usize;

			let offset = surface.data_offset() as usize;
			let data_length = surface.row_stride() as usize * height;

			let format = surface.format().into();
			let image_data = &self.data[offset..offset + data_length];

			let sbro = module.sensor_bayer_red_override.clone().unwrap();

			images.push(RawImage {
				camera,
				// Populated after all the blocks are processed
				sensor: SensorModel::Unknown,
				width,
				height,
				format,
				data: image_data,
				sbro: (sbro.x(), sbro.y()),
				// Populated after all the blocks are processed
				color: vec![],
			});
		}

		if let Some(Ok(irc)) = image_reference_camera.map(|ev| ev.enum_value()) {
			ext.reference_camera = Some(irc.into());
		}

		ext
	}

	// It kept making my neat little array very, very tall
	#[rustfmt::skip]
	fn deconstruct_matrix3x3(mat: Matrix3x3F) -> [f32; 9] {
		[
			mat.x00(), mat.x01(), mat.x02(),
			mat.x10(), mat.x11(), mat.x12(),
			mat.x20(), mat.x21(), mat.x22(),
		]
	}
}

pub(crate) struct ExtractedData {
	pub reference_camera: Option<CameraId>,
}

pub enum Message {
	LightHeader(LightHeader),
	ViewPreferences(ViewPreferences),
	Gps(GPSData),
}

pub struct Header {
	/// The length of this header plus the data after it.
	pub block_length: usize,
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
			block_length: combined_length,
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
