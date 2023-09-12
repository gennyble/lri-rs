Still early days. It's usable, to some degree, but nothing is stable, everything is broken, and I will scream.

### LRI File Structure
The file is made up if blocks each with a short header, a protobuf message, and possibly some associated data.

Things seem to be **little endian**

#### Block Header
The header is 32 bytes long. and goes as follows:  
| bytes | meaning |
| ----- | ------- |
| 4     | Magic Number: "LELR" |
| 8     | block length |
| 8     | protobuf message offset from the start of the block |
| 4     | protobuf message length |
| 1     | message type. 0 for `LightHeader` *(as described in lightheader.proto)*, 1 for `view_preferences.proto`, or 2 for gps data  |
| 7     | reserved |
