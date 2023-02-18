# Line Reversal Control Protocol

I am approaching this as an exercise in memory management. Specifically, I want
an implementation that copies memory as few times as possible. The flow should
be something like:

* receive a udp datagram of 1000 bytes or less, reading it into a vector on the
heap, probably.
* parse the vector into a message, the data message containing a reference to the
datagram vector
* if the datagram vector contains any escape characters, either copy the decoded
data message into a new vector on the heap, or record the offsets of the escape
characters to ignore later
* implement a byte reader interface atop the collection of vectors or vector
fragments, releasing the vectors as soon as they are fully read