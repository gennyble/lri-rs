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
