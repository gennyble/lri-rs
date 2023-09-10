use std::fs;

use protobuf_codegen::Customize;
use std::path::Path;

fn main() {
    let proto_dir = "src/proto";

    if Path::new(&proto_dir).exists() {
        fs::remove_dir_all(&proto_dir).unwrap();
    }
    fs::create_dir(&proto_dir).unwrap();

    protobuf_codegen::Codegen::new()
        .pure()
        .customize(Customize::default().gen_mod_rs(true).lite_runtime(true))
        .out_dir(proto_dir)
        .input("proto/camera_id.proto")
        .input("proto/camera_module.proto")
        .input("proto/color_calibration.proto")
        .input("proto/dead_pixel_map.proto")
        .input("proto/device_temp.proto")
        .input("proto/distortion.proto")
        .input("proto/face_data.proto")
        .input("proto/flash_calibration.proto")
        .input("proto/geometric_calibration.proto")
        .input("proto/gps_data.proto")
        .input("proto/hot_pixel_map.proto")
        .input("proto/hw_info.proto")
        .input("proto/imu_data.proto")
        .input("proto/lightheader.proto")
        .input("proto/matrix3x3f.proto")
        .input("proto/matrix4x4f.proto")
        .input("proto/mirror_system.proto")
        .input("proto/point2f.proto")
        .input("proto/point2i.proto")
        .input("proto/point3f.proto")
        .input("proto/proximity_sensors.proto")
        .input("proto/range2f.proto")
        .input("proto/rectanglei.proto")
        .input("proto/sensor_characterization.proto")
        .input("proto/sensor_type.proto")
        .input("proto/time_stamp.proto")
        .input("proto/tof_calibration.proto")
        .input("proto/view_preferences.proto")
        .input("proto/vignetting_characterization.proto")
        .include("proto")
        .run()
        .unwrap();
}
