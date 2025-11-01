Each request has the following pattern:

1. Generate a request message
2. Register the request with the various message brokers
3. Send the request to a given socket
4. Loop through the various receivers until we get the reply we're waiting for

For flexibility, these should always return closures that can be drained like
coroutines/generators. The reason for this is that, while most requests would
be expected to return a single response, some, like execute requests, may
return many.

Furthermore, it's not guaranteed that other channels, such as stdin, will not
fire in response to a given request. For example, R may ask for stdin on
shutdown request.


