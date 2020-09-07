# RESTful Flight Data Service

This actor exposes flight data for individual aircraft as well as the list of all configuration receiving stations (capability providers). It requires a binding with an HTTP server provider and a key-value store provider to access raw data.

When running, it will expose the following URLs on a port given by the `PORT` actor binding configuration value:

* `/aircraft` - Last known status of all discovered aircraft in the system
* `/stations` - List of all stations from which data has arrived
