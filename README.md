Still early days. Some things work, some things don't.

I've detailed what I know about the LRI format in [LRI.md](LRI.md). Details about a weird format they use called Bayer JPEG described in [bayer_jpeg.md](bayer_jpeg.md).

A friend archived a lot of Light L16 stuff at [helloavo/Light-L16-Archive](https://github.com/helloavo/Light-L16-Archive). Notably it contains firmaware images, instructions for upgrading your firmware, root instructions, and a lot more! It was incredibly helpful.

### lri-rs
A Rust crate for parsing LRI files. This library isn't perfect, but it works enough to be able to grab image data from the files. 

### prism
Breaks an LRI into the individual images it contains  
`prism <lri> <output_directory>`

TODO: I'd like to, one day, be able to write DNG files from prism, but currently it just spits out PNG.

### lri-proto
This is a gently modified version of the [dllu/lri.rs](https://github.com/dllu/lri-rs) repository. Without the work from Daniel pulling the protobuf definitions from the Lumen software I truly don't know if I could've got as far as I did.

MIT Copyright Daniel Lawrence Lu

### lri-study
Run with the arguments `gather <path>` to print information about the LRI files in the directory to stdout.

This was very useful to me while developing lri-rs to be able to see if patterns repeated across many different images so I could make some assumptions.

#### Licensing?
`lri-proto` is MIT Copyright Daniel Lawrence Lu.

everything else is ISC copyright gennyble <gen@nyble.dev>.

Just means you have to provide attribution to the correct person if you use this code and that you're free to do with it what you like.