# BayerJPEG
Th BayerJPEG is a strange format used by the Light L16... *sometimes*. We don't yet know when it switches from it's normal packed 10-bit raw format.

| size    | type   | meaning |
| ------- | ------ | ------- |
| 4 bytes | String | Magic Number "BJPG" |
| 4 bytes | u32    | *Format type* <br/> 0: colour <br/> 1: for monochrome |
| 4 bytes | u32    | Length of Jpeg 0 |
| 4 bytes | u32    | Length of Jpeg 1 |
| 4 bytes | u32    | Length of Jpeg 2 |
| 4 bytes | u32    | Length of Jpeg 3 |
| 1552 bytes | | unknown |

***Monochrome***  
Jpeg0 contains a full resolution grayscale image

***Colour***  
The bayered image is split across the four Jpeg, one
for each colour location.

I.E. an image from the ar1335 sensor, color filter bggr, you'd get
- 1 jpeg for the blue channel
- 2 jpeg for each green location
- 1 jpeg for the red channel

It's not currently known if these are in the order you'd expect.

***Considerations***
When the L16 decides to use BayerJPEG, it has to save four copies of each frame. A JPEG is limited to a bit depth of eight, but the sensors output 10-bit data. In order to not loose 75% of the precision, they seemingly divide the image into fours and expect you to sum them later.