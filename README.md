# Cursedboard

This is supercursed.

## Goal:

Sync the clipboard actions on the mac side with the content getting copied in the linux side, and the other way.

## Underlying assumptions:

1. We will always use port 34254
2. TCP
3. Zero encryption
4. We will prefix the length of the incoming message with a 8-byte unsigned integer.

## Architecture

1. Every platform needs a server, and a client:

- Server: Handles incoming clipboard updates
- Client: handles updates of the local clibboard instance

Questions: Do I need one socket per direction? Can I use the same socket for both writing and reading?
