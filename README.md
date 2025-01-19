A repository containing code I wrote to learn new languages and experiment with new things. 

##### Software architect
-   `[In Progress]` **Plugin system** – a system I originally designed in Go at work.
    It supports application lifecycle (on start, on destroy, etc., inspired by
    Unity) for each plugin and a callback driven event dispatcher.
-   `[In Progress]` **Asynchronous runtime** – A runtime inspired heavily by Tokio and
    smol which can schedule and run Rust's futures.

##### Data structures
- `[Planned]` Re-implement vec[^rust-vec]

##### Graphics
- `[In Progress]` Experiment rendering in WebGPU (via wgpu, also my first time trying WebGPU)[^wgpu]

##### Algorithms
- `[Planned]` Re-implement image blob detection[^blob-detection]

##### Coding challenges/courses
- [Fly.io Distributed Systems Challenge](https://fly.io/dist-sys/)
- [CIS 1940's Haskell](https://www.cis.upenn.edu/~cis1940/spring13/lectures.html)

[^rust-vec]: [Implementing Vec - The Rustonomicon](https://doc.rust-lang.org/nomicon/vec/vec.html)
[^wgpu]: [Learn wgpu - sotrh.github.io](https://sotrh.github.io/learn-wgpu)
[^blob-detection]: [Blob detection - Wikipedia](https://en.wikipedia.org/wiki/Blob_detection)
