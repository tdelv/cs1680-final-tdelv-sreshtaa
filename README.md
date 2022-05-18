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

## Future Work