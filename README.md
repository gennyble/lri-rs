Still early days. Some things work, some things don't.

I've details what I know about the LRI format in [LRI.md](LRI.md).

And details about a weird format they use called Bayer JPEG described in [bayer_jpeg.md](bayer_jpeg.md)

I took some notes as I worked on this that are in [NOTES.md](NOTES.md) if you're interested in that.

## lri-rs
A Rust crate for parsing LRI files. This library isn't perfect, but it works enough to be able to grab image data from the filee.

## prism
Breaks an LRI into the individual images it contains.

TODO: I'd like to, one day, be able to write DNG files from prism, but currently it just spits out PNG.

## lri-proto
This is a gently modified version of the [dllu/lri.rs](https://github.com/dllu/lri-rs) repository. Without the work from Daniel pulling the protobuf definitions from the Lumen software I truly don't know if I could've got as far as I did.

MIT Copyright Daniel Lawrence Lu

## lri-study
Run with the arguments `gather <path>` to print information about the LRI files in the directory to stdout.

This was very useful to me while developing lri-rs to be able to see if patterns repeated across many different images so I could make some assumptions.

### Licensing?
`lri-proto` is MIT Copyright Daniel Lawrence Lu.

everything else is ISC copyright gennyble <gen@nyble.dev>.

Just means you have to provide attribution to the correct person if you use this code and that you're free to do with it what you like.