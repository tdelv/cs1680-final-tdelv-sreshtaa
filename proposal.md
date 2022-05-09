# Final Project Proposal

<!-- Write up a brief description of the project you want to
implement and submit it on Gradescope. Your proposal writeup need not be more than a couple
of paragraphs: just describe what you’re thinking of doing, what you hope to achieve as a final
deliverable, and how you intend to get started. -->

Team: Sreshtaa Rajesh and Thomas Del Vecchio

Repository: https://github.com/tdelv/cs1680-final-tdelv-sreshtaaa

We plan to reimplement the Snowcast assignment using gRPC in Rust. We will use [tonic](https://github.com/hyperium/tonic), a Rust native implementation of gRPC. We will start by implementing the basic Snowcast handshake, involving a `Hello` command and a `Welcome` command. After we get an understanding of the basics of gRPC, we will then try to implement as much of the rest of the commands, making use of our code from the original project for server logic.

Goals:
- learn how gRPC works
- understand the benefits and costs of using gRPC as opposed to custom-made protocol implementation
- compare the process of developing gRPC code, as well as the process of updating existing gRPC code, against custom-made protocol implementation

Deliverable: We hope to at least be able to support a single client being able to switch between multiple stations, but will if we reach that faster than expected, then we will try to support mutiple clients at once, and possibly additional requests (e.g. the extra credit portion).


