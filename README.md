# cs1680 Final Project

(Note: Proposal found in `proposal.md`)

## Introduction

For our project, we chose to explore gRPC and attempt to implement Snowcast using RPCs. Our goal was to, at minimum, implement the client-server handshake (via `Hello` and `Welcome`) and allow the client to choose a station (via `SetStation` and `Announce`). 

Our deliverable is a reimplementation of Snowcast that allows a clients to connect to the server, choose a station, and receive the bytes being streamed. 

## Design & Implementation

### Protobuf

We used Rust's `tonic` crate, which contains a Rust-native implementation of gRPC in order to set up and integrate our RPC commands into Snowcast. This crate provides a nice interface for definining server-side logic for handling RPC service requests through the use of a trait that has functions for all of the services specified in the protobuf. 

Our protobuf was set up with three RPC services that represent the three client-server communications that we wanted to implement: the hello handshake, the correspondence when clients set their station, and the graceful exit when either the client or server quits. These are shown below.

```protobuf
service Snowcast {
    rpc SayHello (HelloRequest) returns (WelcomeReply);
    rpc SetStation (SetStationRequest) returns (AnnounceReply);
    rpc SayGoodbye (QuitRequest) returns (GoodbyeReply);
}
```

We implemented our protobuf messages to match the Snowcast specification. Namely, the client's `HelloRequest` should contain the UDP port of the listener client, and the server's `WelcomeReply` should contain the number of stations. 

```protobuf
message HelloRequest {
    uint32 udpPort = 1;
}

message WelcomeReply {
    uint32 numStations = 1;
}
```

Similarly, for the switching-stations protocol, the `SetStationRequest` command needs to contain the station number that the client wants to switch to, and the server's `AnnounceReply` should contain the name of the song playing. We represented these commands as follows: 

```protobuf
message SetStationRequest {
    uint32 stationNumber = 1;
}

message AnnounceReply {
    string songName = 1;
}
```

Lastly, both the `QuitRequest` and `GoodbyeReply` were empty messages. 

In the beginning of the semester, we had to manually serialize these, but using protobuf messages provided a really easy way to specify what our custom protocols should look like without having to worry about the details of serialization. 

### Server-side Implementation 

As mentioned previously, the `tonic` crate takes a protobuf, and provides a trait for the server to implement that details how it will handle the logic of each type of requested RPC service. The internals of our server were contained in a struct called `MyServer`. Using `tonic`, we implemented a `Snowcast` trait for this server that mirrored the RPC services detailed above: 

```rust
#[tonic::async_trait]
impl Snowcast for MyServer {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> std::result::Result<Response<WelcomeReply>, Status>;

    async fn set_station(
        &self,
        request: Request<SetStationRequest>,
    ) -> std::result::Result<Response<AnnounceReply>, Status>;

    async fn say_goodbye(
        &self,
        request: Request<QuitRequest>,
    ) -> std::result::Result<Response<GoodbyeReply>, Status>;
}
```

Each function returns a `Response` struct provided by `tonic` that contains a custom `Reply` as defined by our protobuf. This enables the client-side to handle the server's response, and thus completes the client-server communication. 

## Discussion & Results

<!-- How far did you get toward your goal? In this section, describe any results you have, what you have learned, and any challenges you faced along the way -->

We were able to implement a basic working implementation of Snowcast where a server can run a number of stations, and clients can connect and switch between stations. Our server was able to stream bytes to the listener client upon station selection, as well, and the REPL was also functional. 

One challenging aspect of this project was implementing the server feature which resends an `Announce` command when the song repeats. Revisiting the services in our protobuf, we see that requesting the `SetStation` service requires a `SetStationRequest` from the client and it solicits an `AnnounceReply` from the server. Sending an `AnnounceReply` when the song repeats is a form of unsolicited messaging from the server which was not representable using the RPC.

We considered a few different options for work-arounds. One was to have the client occasionally send a request to check for a song update, so then it has a solicited response, but we wanted to make song announcements non-delayed. Another possibility was to just have the `WelcomeReply` also include a stream of song announcements that would just persist over a very long time, but we felt this was a misuse of the RPC. Thus, our main approach was to actually have each client host their own "server" for handling broadcast messages. This didn't seem like an ideal option, but it was one we wanted to try. Unfortunately, we ran into some scary Rust async/await roadblocks, so we ultimately decided not to implement this feature.

## Conclusions & Future Work

<!-- Overall, what have you learned? How did you feel about this project overall? If you could keep working on this project, what would you do next? Are there any other directions of this work you find interesting? If you have any thoughts or feedback on this project model, please let us know! -->

This project was a fun experience, and it was a nice way to tie up the course by revisiting the first project with a new tool and exploring how it changed our design. We specifically think that all the manual serializing work we did with the past three projects increased our appreciation for gRPC and the way we could send messages between client and server without needing to actually implement the functionality that serializes and sends messages over the network. Abstracting away the logic regarding the internals of the network allowed us to also trim down our code to mostly just the implementation of the internal logic of our client and server. 

If we could keep working on our project, we would finish implementing the features of Snowcast that we didn't yet get to. Specifically, one piece of client-server communication that we did not yet implement was the server sending the client unprompted `Announce` messages when songs repeat (or when the song changes) on a station. 

As for future work, it might be interesting to actually try and implement a basic version of an RPC framework (which would require looking more at what the internals of what libraries like `tonic` contain). 

As for feedback on the project model, we like the open-ended nature of this project! One thing that we would have liked was a little bit more time to work on the project (if we were to do it again, we might have chosen something more exploratory than reimplementing Snowcast, but time pressure was a contributing factor to choosing a less exploratory project idea). 