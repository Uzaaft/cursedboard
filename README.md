# Cursedboard

This is supercursed.

## Underlying assumptions:

1. We will always use port 34254
2. TCP
3. Zero encryption

## Architecture

1. Every platform needs a server, and a client:

- Server: Handles incoming clipboard updates
- Client: handles updates of the local clibboard instance

Questions: Do I need one socket per direction? Can I use the same socket for both writing and reading?
