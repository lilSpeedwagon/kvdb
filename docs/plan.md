# Implementation Plan

This document describes the initial requirements and preliminary implementation plan.

The basic ersion supports the following features:

- Client-server connections
- Custom binary network protocol over TCP
- Storing key-value pairs
- Fast lookups values by keys
- Keys are strings
- Basic value data types:
  - strings
  - binary blobs
  - booleans, byte arrays
  - integers, floating point numbers
  - arrays
  - hash maps
  - hash sets
- Single node
- Guaranteed data persistence (via WAL)
- Failover recovery
- Server CLI
- A very basic client in Python

Follow up features:

- Secondary indexes (for complex structures)
- Queue data type
- Value expiration
- Read or hot-standby replicas
  - sync/async replication
- Sharding
  - Consensus configuration
- Persistence configuration (always, on timer)
- Capacity limits and data eviction policies (FIFO, LIFO, etc)

## MVP Plan

- [ ] Key-value storage
- [ ] Strings only
- [ ] No data persistence (in-memory storage)
- [ ] Simple network protocol
- [ ] Sync server and DB engine
- [ ] Basic CLI
