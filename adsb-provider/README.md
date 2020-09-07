# ADS-B Capability Provider

This capability provider captures real-time messages from a telnet server that exposes AVR messages. These AVR messages are then parsed into a meaningful data structure and delivered via actor binding to a processor actor.

## Pre-Requisites

The following must be in place in order to successfully run this provider in a waSCC host:

* TCP connectivity to the telnet server exposed by the [dump1090](https://github.com/MalcolmRobb/dump1090) application.
* A machine connected to an [RTL-SDR](https://www.rtl-sdr.com/) dongle and antenna combination with the appropriate device drivers. Consult the RTL SDR website for advice on purchasing devices, antennas, physical configuration, and device drivers and software installation for your equipment.

Simply run `dump1090 --net` or `dump1090 --interactive --net` to start the telnet server required by this capability provider.
