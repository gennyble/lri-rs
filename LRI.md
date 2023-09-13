# Anatomy of an LRI
The file is made up of many blocks, usually 10 or 11 but cases of 40 have occurred.

Blocks start with a header and contain some data. There is always a protobuf message within that data, and sometimes stuff like the images themselves.

## Block Header
The header is 32 bytes long. and goes as follows:  
| bytes | type | meaning |
| ----- | -----| ------- |
| 4     | -    | Signature: "LELR" |
| 8     | u64  | block length |
| 8     | u64  | message offset from block start |
| 4     | u32  | message length |
| 1     | u8   | message type, see below |
| 7     | -    | reserved |

*message* to mean the protobuf message

*Message Type*  
0: LightHeader ([proto][lh-proto])  
1: ViewPreferences ([proto][vp-proto])  
2: GPSData ([proto][gps-proto])  

[lh-proto]: /lri-proto/proto/lightheader.proto
[vp-proto]: /lri-proto/proto/view_preferences.proto
[gps-proto]: /lri-proto/proto/gps_data.proto

## highlighting key parts
blocks have messages and that's pretty much all you need to know. you can look at the protobuf definitions and pretty readily build a parser, but there are some things i'd like to mention, too.

### LightHeader
The most important header and frustratingly fractured between multiple blocks.

#### RAW Images
What we're all here for, maybe.

zero or more CameraModule ([proto][module-proto]) are collected in the `modules`. In a CameraModule we see a `sensor_data_surface` of type Surface (see line 33 of the CameraModule proto).

[module-proto]: /lri-proto/proto/camera_module.proto

- `start` might indicate a crop, but has always been (0,0) in my experience.
- `size` gives the width/height of the image.
- `data_offset` is the start of the image from the beginning of the block (meaning: it includes the length of the header).
- `format` indicates how we're meant to interpret the image data. It can be a few different things, but i've only seen RAW_BAYER_JPEG and RAW_PACKED_10BPP.
- `row_stride` gives you the number of bytes per row the image takes up. Multiply this by the width to get the size of the image (except Bayer JPEG; see below)

##### Let's talk about Bayer JPEG.  
We don't currently understand *why* the L16 makes these, just that it does. If it's from a colour sensor, you'll get four half-res JPEG (one for each bayer position). If it's monochrome, you'll get one full-res JPEG. For more information go here: [bayer_jpeg.md](/bayer_jpeg.md).

In either colour-case, the `row_stride` in the `sensor_data_surface` will be 0. You'll have to parse the Bayer JPEG header to get the length of the sensor's image data.

##### that's enough BayerJPEG

Going back to CameraModule, there's some more important data for image interpretation. You'll want the `id` which indicates which camera took the exposure. We can map this to a sensor model later! Grab `sensor_bayer_red_override` while you're at it. It'll help with figuring out what CFA we need to use for debayering.
