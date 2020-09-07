# ADS-B Processor Actor

This actor is responsible for processing the messages delivered to it by an ADS-B capability provider. Remembering that all actors are supposed to be stateless, this actor cannot make any assumptions about the context surrounding the delivery of these messages.

It performs the following tasks:

* Examines the inbound raw ADS-B message
* Converts the message to an _event sourcing_ event
* Applies the event to multiple aggregates to produce new state
* Persists updated state in a key-value store
* Publishes the post-processing event on an appropriate message broker subject for use by downstream consumers (e.g. the real-time web UI).

This actor requires the following capabilities:

* Message Broker
* ADS-B
* Key-Value Store