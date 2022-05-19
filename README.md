# cs1680 Final Project

(Note: Proposal found in `proposal.md`)

## Introduction

For our project, we chose to explore gRPC and attempt to implement Snowcast using RPCs. Our goal was to, at minimum, implement the client-server handshake (via `Hello` and `Welcome`) and allow the client to choose a station (via `SetStation` and `Announce`). 

Our deliverable is a (very pared-down) implementation of Snowcast that allows a single client to connect to the server and choose a station. Something we didn't get around to doing is actually streaming music to the UDP client, but the server does announce what song the client's selected station is playing, which gives us confidence that the RPC commands are working. 

## Design & Implementation

We used Rust's `tonic` crate, which contains a Rust-native implementation of gRPC in order to set up and integrate our RPC commands into Snowcast. This crate provides a nice interface for **[insert later]**. 

Our protobuf was set up with two RPC services that represent the two client-server communications that we wanted to implement: the hello handshake, and the correspondence when clients set their station. These are shown below.

```protobuf
service Snowcast {
    rpc SayHello (HelloRequest) returns (WelcomeReply);
    rpc SetStation (SetStationRequest) returns (AnnounceReply);
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

While originally, we had to manually serialize these, protobuf messages provided a really easy way to specify what our custom protocols should look like without having to worry about the details of serialization. 

## Discussion & Results

<!-- How far did you get toward your goal? In this section, describe any results you have, what you have learned, and any challenges you faced along the way -->


## Conclusions & Future Work

<!-- Overall, what have you learned? How did you feel about this project overall? If you could keep working on this project, what would you do next? Are there any other directions of this work you find interesting? If you have any thoughts or feedback on this project model, please let us know! -->

This project was a fun experience, and it was a nice way to tie up the course by revisiting the first project with a new tool and exploring how it changed our design. We specifically think that all the manual serializing work we did with the past three projects increased our appreciation for gRPC and the way we could send messages between client and server without needing to actually implement the functionality that serializes and sends messages over the network. Abstracting away the logic regarding the internals of the network allowed us to also trim down our code to mostly just the implementation of the internal logic of our client and server. 

If we could keep working on our project, we would finish implementing the features of Snowcast that we didn't yet get to. Specifically, one piece of client-server communication that we did not yet implement was the server sending the client unprompted `Announce` messages when songs repeat (or when the song changes) on a station. 

As for future work, it might be interesting to actually try and implement a basic version of an RPC framework (which would require looking more at what the internals of what libraries like `tonic` contain). 

As for feedback on the project model, we like the open-ended nature of this project! One thing that we would have liked was a little bit more time to work on the project (if we were to do it again, we might have chosen something more exploratory than reimplementing Snowcast, but time pressure was a contributing factor to choosing a less exploratory project idea). 