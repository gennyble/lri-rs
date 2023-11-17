use std::collections::HashMap;

use lri_proto::{lightheader::LightHeader, view_preferences::ViewPreferences};

#[derive(Debug)]
pub struct Signature {
	pub lh: HashMap<&'static str, Vec<String>>,
	pub vp: HashMap<&'static str, Vec<String>>,
}

impl Signature {
	pub fn new() -> Self {
		Self {
			lh: HashMap::new(),
			vp: HashMap::new(),
		}
	}

	pub fn merge(&mut self, lh: &LightHeader) {
		let LightHeader {
			image_unique_id_low,
			image_unique_id_high,
			image_time_stamp,
			image_focal_length,
			image_reference_camera,
			device_unique_id_low,
			device_unique_id_high,
			device_model_name,
			device_fw_version,
			device_asic_fw_version,
			device_temperature,
			modules,
			module_calibration,
			device_calibration,
			gold_cc,
			sensor_data,
			tof_range,
			hw_info,
			view_preferences,
			proximity_sensors,
			flash_data,
			imu_data,
			af_info,
			gps_data,
			compatibility,
			face_data,
			special_fields,
		} = lh;

		macro_rules! hh {
			($field:ident) => {
				let i = match $field {
					Some(v) => vec![v.to_string()],
					None => vec![],
				};

				self.lh
					.entry(stringify!($field))
					.and_modify(|v| v.extend_from_slice(&i))
					.or_insert(i);
			};
		}

		macro_rules! mf {
			($field:ident) => {
				let add = if $field.is_some() { 1 } else { 0 };

				self.lh
					.entry(stringify!($field))
					.and_modify(|count| *count += add)
					.or_insert(add);
			};
		}

		macro_rules! hv {
			($field:ident) => {
				let add = $field.len();

				self.lh
					.entry(stringify!($field))
					.and_modify(|count| *count += add)
					.or_insert(add);
			};
		}

		hh!(image_unique_id_low);
		hh!(image_unique_id_high);
		mf!(image_time_stamp);
		hh!(image_focal_length);
		hh!(image_reference_camera);
		hh!(device_unique_id_low);
		hh!(device_unique_id_high);
		hh!(device_model_name);
		hh!(device_fw_version);
		hh!(device_asic_fw_version);
		mf!(device_temperature);
		hv!(modules);
		hv!(module_calibration);
		mf!(device_calibration);
		hv!(gold_cc);
		hv!(sensor_data);
		hh!(tof_range);
		mf!(hw_info);
		mf!(view_preferences);
		mf!(proximity_sensors);
		mf!(flash_data);
		hv!(imu_data);
		mf!(af_info);
		mf!(gps_data);
		mf!(compatibility);
		hv!(face_data);
	}

	pub fn vp(&mut self, vp: &ViewPreferences) {}
}

/*
optional uint64 image_unique_id_low = 1;
optional uint64 image_unique_id_high = 2;
optional TimeStamp image_time_stamp = 3;
optional int32 image_focal_length = 4;
optional CameraID image_reference_camera = 5;
optional uint64 device_unique_id_low = 6;
optional uint64 device_unique_id_high = 7;
optional string device_model_name = 8;
optional string device_fw_version = 9;
optional string device_asic_fw_version = 10;
optional DeviceTemp device_temperature = 11;
repeated CameraModule modules = 12;
repeated FactoryModuleCalibration module_calibration = 13;
optional FactoryDeviceCalibration device_calibration = 14;
repeated ColorCalibrationGold gold_cc = 15;
repeated SensorData sensor_data = 16;
optional float tof_range = 17;
optional HwInfo hw_info = 18;
optional ViewPreferences view_preferences = 19;
optional ProximitySensors proximity_sensors = 20;
optional FlashData flash_data = 22;
repeated IMUData imu_data = 23;
optional AFDebugInfo af_info = 24;
optional GPSData gps_data = 25;
optional Compatibility compatibility = 26;
repeated FaceData face_data = 27;
*/
