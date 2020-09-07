# Wasm Air

**Wasm Air** is a use case scenario exploration to answer the question, "What would it be like to try and build a system like Flight Aware with waSCC, WebAssembly, and lattice?" We started out with the idea of enabling the collection and aggregation of [Automatic Dependent Surveillance-Broadcast (ADS-B)](https://www.faa.gov/nextgen/programs/adsb/faq/) signals. The next step was to figure out where the business logic would reside and what we would use for capability providers.

## Components

The following is a list of the main logical components of Wasm Air:

* The ADS-B Capability Provider
* The ADS-B Message Processor (Actor)
* A RESTful Service Exposing Current Flight Data (Actor)
* A Website Consuming Post-Processing Events from the Processors

### ADS-B Capability Provider

The [ADS-B capability provider](./adsb-provider/) could be written so that it consumes the raw signals from an SDR and then decodes those into ADS-B messages, but there is already an application that does this with far greater skill than we could, with years of running deployments supporting it.

As a result, we wrote the capability provider to connect over TCP to the [dump1090](https://github.com/MalcolmRobb/dump1090) telnet server and decode the AVR messages from that. This is a subtle, yet powerful point. The capability provider emitting a firehose of raw ADS-B messages to bound actors _need not_ be physically running on the device connected to the SDR. It _only_ needs TCP connectivity on a port like `30002` in order to obtain the raw data. This is ideal for hub-and-spoke IoT models where you have a smaller device with the SDR attached, and a collector that might be running the capability provider.

Again, the flexibility that waSCC's lattice provides means you can deploy this in whatever configuration or scale you like.

Each named instance (currently called a "binding name") of a capability provider will be bound to a horizontally scaled group of processor actors. This binding feeds ADS-B messages _from one receiving station_ to that actor group for processing.

If you want to aggregate multiple stations, then you should deploy a capability provider _per station_, which each one connected to that station's `dump1090` process. Then, in your lattice configuration, simply creating a new binding between the processor actor and each of these per-station providers (in a manifest `yaml` file, this will look like binding the same actor ID to multiple instances of the same provider). The processing actors will then receive all traffic from all connected stations.

### ADS-B Message Processor (Actor)

The [ADS-B Message Processor](./adsb-processor) is an actor written using the waSCC SDK. It receives ADS-B messages by virtue of its binding to an ADS-B capability provider. In turn, it will pull apart that message, convert it into an internal format, and then create an internal _event_ representing some plane event (velocity changed, aircraft identification received, etc). This event is then published on a message broker subject, while it is also run through an _event sourcing_ domain model to calculate the current state of all discovered aircraft. This state is persisted in a key-value store, which can be bound to anything from Redis to Cassandra to a transient in-memory cache.

### RESTful Flight Data Service (Actor)

The [RESTful flight data service](./wasmair-rest) is also an actor written using the waSCC SDK. It is bound to an HTTP server capability provider which creates a listening endpoint, and it is bound to a key-value store configured to read the materialized data produced by the message processor actor(s). It exposes the following resources:

* `/aircraft` - The current status of all aircraft discovered by the system, in aggregate. While there's no current functionality for limiting/filtering this data, that can be easily added.
* `/stations` - Queries the list of all registered receiving stations (providers connected to a `dump1090` server).

### Realtime UI (Website)

The [Realtime UI](./wasmair-web) is a website written in Phoenix & LiveView (Elixir/OTP). This website allows users to view a list of registered stations, navigate to a live view of each station's real-time data, and view a global aggregate of all live system data. The website gets its data via a combination of querying the RESTful flight data service and receiving live post-processing events from the event stream produced by the processor actors.

The website can be deployed _anywhere_, no matter what scale your deployment is, so long as it has network connectivity to the flight data service and can subscribe to the appropriate message broker subjects for live events.

## Strategies for Scale

One of the goals of **Wasm Air** is to illustrate how waSCC and lattice enable you to dynamically scale your systems without having to recompile your actors or redesign your providers.

### Offline / Laptop / Lab

The smallest scale involves running everything in a single host on a laptop or a small device like a Raspberry Pi. This is easily done by plugging in the USB dongle for your SDR (after you've installed the appropriate RTL-SDR device drivers), running `dump1090` on a TCP port, and then starting up a waSCC host that holds implementations of the message broker, key-value, and ADS-B (`sdr:adsb`) capability providers. Scaling out from here just involves splitting up the components and scaling them independently.

### Flat Lattice Topology

In this scenario, everything is running on its own waSCC host on its own (physical or virtual) machine. A single NATS server can provide the lattice backbone for the following waSCC hosts:

* **Processor Actor** - holds processor actor and key-value and broker providers
* **Flight Data Service** - holds rest actor, key-value provider, and HTTP server provider
* **ADS-B Provider** - holds the ADS-B capability provider and has TCP connectivity to the host running `dump1090`

### Multiple Receiving Stations

In this scenario, we have multiple receiving stations, which means we are likely running multiple copies of the processor actors and we're running a named instance of the ADS-B capability provider for each of our receiving stations. Because the public key of the processor actor remains identical across all of the receiving stations, if we use a flat topology with a single NATS backbone, then the lattice will _randomly choose_ which of the many running processor actors will receive each ADS-B message. This can be horribly inefficient, especially if there's a processor actor and ADS-B provider on the same machine.

We can optimize this scenario with NATS _leaf nodes_. Each of our logical _receiving stations_ (can be a single box or further separated as desired) consists of the following:

* NATS server exposing a port on the loopback adapter connected to the system backbone NATS in leaf mode
* waSCC host process holding _a_ processor actor
* waSCC host process holding a named instance of the ADS-B capability provider (can also be the same host process as the one holding the actor)

This receiving station isolation on leaf nodes means that while the binding between the processing actor and ADS-B provider is _global_, the traffic between them will always be _local optimized_. In other words, if we have 5 receiving stations, each with an ADS-B provider and a waSCC host with 1 or more processing actors, the _local actors_ will receive messages from the provider. If the local actor's host goes down, the system will use the higher-latency backbone round trip to deliver messages.

## Topology Examples

The following are just a few among the countless options for deploying the Wasm Air sample.

### Flat, Unoptimized Receiving Station Network

```
.------------.  .------------.  .------------.
| Provider A |  | Provider B |  | Provider C |
`------------'  `------------'  `------------'
      |              |               |
.-----------------------------------------.
|             L A T T I C E               |
`-----------------------------------------'
      |                              |
.-------------.               .-------------.
| Processor A |               | Processor B |
`-------------'               `-------------'
```

In the flat network there is no affinity between processors and providers. Every provider will deliver each inbound message to a _randomly chosen_ running bound actor.

### Leaf Node, Edge-Optimized Receiving Station Network

In the leaf-isolated (edge optimized) network design, each provider will deliver its messages to the processor actor within the leaf node, not sending traffic across the backbone until the post-processing event is published.

```
.-----------------.     .-----------------.
|    LEAF NODE    |     |    LEAF NODE    |
|  .-----------.  |     |  .-----------.  |
|  |  Prov A   |  |     |  |  Prov  B  |  |
|  `-----------'  |     |  `-----------'  |
|        |        |     |       |         |
|  .-----------.  |     |  .-----------.  |
|  | Processor |  |     |  | Processor |  |
|  `-----------'  |     |  `-----------'  |
`-----------------'     `-----------------'
         |                       |
.------------------------------------------.
|               L A T T I C E              |
`------------------------------------------'
```

You are free to adopt either of these topologies, or a hybrid of both, by simply changing your runtime configuration and networking setup. You do not need to recompile any of your code to switch between these designs.
