# Message Queue Module

## Intro
This module offers mega the ability to send and handle specific events.

After sending the event you created into a global message queue, it will be received in a handler thread and run the callback function defined in trait `EventBase`.

The events would also be asynchronously flush into database for further investigation.

## New Customized Event

If you want to make a new event type and use it in other modules, you should do as follows.

- Create a event struct which implements trait `EventBase`.
- Give your new event a way to enqueue itself into the message queue.
- Add a enum variation in `EventType` which is defined in `src/event/mod.rs`.
- Fill the missing match arms in `src/event/mod.rs` and `src/queue.rs`.
- Import and use it.

> Tips:
> Check `src/event/api_request.rs` for a brief example.
