because it's easy to forget progress and also I'm excited to be figuring stuff out!
think of it like a blog but in a markdown file in this repository instead of on my
website.

# Starting to figure it out
## 2023-06-07 19:23 CST
I wanted to find the protobuf data but I don't know how to work with protobuf,
so I figured looking for known data could help? I decided looking for a timestamp
would be the best bet because the camera gives me that information in the gallery
app. Down to the minute, at least. We already know some of the protobuf definitions
because of work by [Daniel Lawrence Lu](https://github.com/dllu)! That work can be
found in [lri-rs](https://github.com/dllu/lri-rs).

[lri-rs_lightheader]: https://github.com/dllu/lri-rs/blob/main/proto/lightheader.proto#L77-L106

So I used a website to build some data! <https://www.protobufpal.com/>. I used this protobuf defintition:
```proto
message TimeStamp {
    required uint32 year = 1;
    required uint32 month = 2;
    required uint32 day = 3;
    required uint32 hour = 4;
    required uint32 minute = 5;
}
```

paired with this JSON:
```json
{
  "year": 2023,
  "month": 6,
  "day": 7,
  "hour": 19,
  "minute": 14
}
```

to get this timestamp: `08e70f100618072013280e`. *(note, of course, that this data is specific to my file)*.
I then did a sliding-window kind of thing where I stepped through the data byte-by-byte and compared it with
the timestamp. It gave me three matches which I thought was weird. I only expected one! But at 11 bytes
matched I didn't think it could be random noise that happened to match.

So I thought some. I Googled "how do I find the start of a protobuf message" and found out that fields can
be output in any order, but that you should *try* and do it numerically by field. I was about ready to write
a *really* bad protobuf reader but then I thought back to the java decompile and `LightHeader.java`. Notably,
look at it's constructor:
```java
import com.squareup.wire.Message;

public LightHeader(@NonNull Message var1, byte var2) {
	this.message = var1;
	this.type = (byte)var2;
}
```

It takes a *generic* Wire message. I was assuming that Header in LightHeader
meant a *file* header, but this is a header for the protobuf messages!

So I looked through the file for the magic number LELR. It was found fifteen times!
I assumed some would be false positives, it's only four bytes!, so I had a good idea
for once! I looked at the 7 bytes that were offset 21 from the end of the magic number.
In the header, these should be reserved: all null. It matched 10 times and failed 5!

# Found some RAW data!!!!
## 2023-06-08 00:37 CST
tired, need sleep, bullet point for now.
- no "lost data"; all data is held by a LightHeader
- it appears sometimes the length fields move around in `header_length` *(which is indeed sometimes set to the header length)* and the message length
- lak was able to use [SourceExplorer][se-dev] to figure out that, yeah, the data really is right after the LightHeader.
- **thank you**
- it appears to be 14bpp packed *(for wall.lri at least)*. the `camera_module.proto` has this as one of the raw options. we should make it a priority to find and parse this.
- yay

[se-dev]: https://github.com/LAK132/SourceExplorer/tree/dev

# lak's really good at things
## 2023-06-08 09:02 CST
I'm on a break from work, lol.

While I was asleep, lak worked for like seven hours! lak improved SourceExplorer while unpacking the data and it's very cool.
For `wall.lri` and in the 2nd bit of sensor data *(block index 3; the third block)*, the data is likely from the AR1335. lak
was able to get a debayered, but not colour correct, image reading it as RAW_10BIT_PACKED in the BGGR arrangement.

lak was able to get a usuable image with a width of 2080 in source explorer, but says "also it's croped in to 4160 4208". noteably 4160 is 2 * 2080, so I don't know? curious.

# Data Data Data Data Data Data
## 2023-06-08 22:41 CST
I now know what lak was talking about. lak was debayering by just cramming the 2x2 BGGR area into one pixel, therefore loosing half
the width. from laks experience the height was unaffected? but it works! so the width *is* 4160. Which apparently is cropped ar1135.
I was able to confirm that by finding the bloody sensor data! The `sensor_data.proto` anyway.

### What We Know So Far
- File made up of blocks of data. Each block starts with a header that, in the decompiled java code *(and mine, currently)*,
is called `LightHeader`. I will call this the **DataHeader** from here on out as suggested by helloavo. It is described below.
- If the header length of the DataHeader is not 32, we interpret this as sensor data. Following the header starts the raw bayer data. It's likely to be packed bits. Either 10, 12, or 14. *(as per `camera_module.proto`)*. The example file we've been using is 10-but packed. It's also 4160 by 3120 which corresponds to a cropped image from the ar1335 sensor.

#### DataHeader *(A.K.A. LightHeader from the Java Code)*
The header is 32 bytes long. and goes as follows:  
| bytes | meaning |
| ----- | ------- |
| 4     | Magic Number: "LELR" |
| 8     | Combined Length (total length including this header) |
| 8     | header length (32) **OR** message length |
| 4     | message length **OR** unknown |
| 1     | message type. 0 for `LightHeader` *(as described in lightheader.proto)* or 1 for `view_preferences.proto` |
| 7     | reserved |

### ***it's September 12th now and I removed this from the readme. putting it here for safekeeping***

[helloavo/Light-L16-Archive](https://github.com/helloavo/Light-L16-Archive):  
helloavo archived a lot of files and data related to the camera here and I am
very, very grateful for that. I'm using the `.class` files they ripped from the
APK. I then used quiltflower to decompile the entire directory. I used this
command: `java -jar quiltflower.jar -dgs=1 Light-L16-Archive/APKs/light_camera_decompiled`

And it's proved useful! In `light/co/camera/proto/LightHeader.java` we can get an idea
of the file header?

Can we parse the message in the header with the protobuf as described in: [dllu/lri-rs](https://github.com/dllu/lri-rs/blob/main/proto/lightheader.proto)?