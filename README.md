# yap_client

Client for `yap` - a proof-of-concept chat program

## IMPORTANT: PROOF OF CONCEPT

*For Highlights see [the server README.](https://github.com/rmanosuthi/yap_server)*

**NOT FOR PRODUCTION USE.** Only basic features have been implemented. DO NOT USE.

| Implemented | Feature | Notes |
|-------------|---------|-------|
|Done|Login
|Done|Register
|Done|Public profile
|Done|Direct messages
|Done|Password hashing
|Done|E2E encryption (DM)
|WIP|Group messages
|Not started|E2E encryption (Group)
|WIP|Query
|WIP|Friends

# Requirements

- OpenSSL
- Nightly Rust (`1.50` as of time of writing)

# How to use

`yap_client login <cfg-path>` - Login and connect. `<cfg-path>` is path to config generated by `register`.
`yap_client register <save-to> <http-addr> <email> <password>` - Register a new account. Saves config to `<save-to>`.

# Implemented commands

`/u <user-id>` - Target `<user-id>` to send a message to.
`<text>` - Send `<text>` to the target.
