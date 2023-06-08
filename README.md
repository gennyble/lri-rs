The Light L16 is a weird little camera with sixteen lens.
It's cheapish now on the 2nd hand market as it's been discontinued.

I would like to be able to read the raw LRI files it puts out. They are, however,
proprietary and not described anywhere. This is my effort.

[helloavo/Light-L16-Archive](https://github.com/helloavo/Light-L16-Archive):  
helloavo archived a lot of files and data related to the camera here and I am
very, very grateful for that. I'm using the `.class` files they ripped from the
APK. I then used quiltflower to decompile the entire directory. I used this
command: `java -jar quiltflower.jar -dgs=1 Light-L16-Archive/APKs/light_camera_decompiled`

And it's proved useful! In `light/co/camera/proto/LightHeader.java` we can get an idea
of the file header?

Can we parse the message in the header with the protobuf as described in: [dllu/lri-rs](https://github.com/dllu/lri-rs/blob/main/proto/lightheader.proto)?

### File Header
The file seems to consist of a header followed be a proto buf message
that then gets appended to it.

The header is **little endian**

#### File Header Structure
The header is 32 bytes long. and goes as follows:  
| bytes | meaning |
| ----- | ------- |
| 4     | Magic Number: "LELR" |
| 8     | header length (32) + protobuf message length |
| 8     | header length (32) |
| 4     | message length |
| 1     | type (?) |
| 7     | reserved |

and then follows the message which already has a known length

## Image Sensors

| Sensor | Resolutions | Output |
| - | - | - |
| AR0835HS | 8 Mp: 3264 × 2448, 6 Mp: 3264 × 1836 | 10−bit Raw, 10−to−8 bit A−Law, 8/6−bit DPCM |
| AR1335 | 4208 × 3120 | DPCM: 10-8-10, 10-6-10 |
| IMX386 | 4032 x 3024 | ? |