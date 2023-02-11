# Protohackers Problem 6 — Speed

For this Protohackers problem, I tried to write my rust solution as I would
generally approach a production system that I wanted to change over time. In my
earlier solutions, I was more interested in correctly solving the problem and
keeping the code understandable. Lacking rust experience, it hadn't been clear
to me how best to decompose my code into functions and modules, but I can't
put that problem off forever.

I adopted a couple of constraints for my experiment:

* no locks, mutexes, or other guards around state — I wanted to see how far you
  can get with message passing
* no tests, other than the Protohackers test suite — I waned to see what bugs I
  would write despite the type and borrow checkers

It took me an embarrassingly long time to achieve a solution, but I finally did.

The things that went right:

* socket handling and message serialization were very easy to get right. The
  spec was clear, the parsing and rendering code very straightforward. Were I
  writing clojure, I'd've looked to express the wire format in data and operate
  over that, and am curious how that might work in rust.

After satisfying the compiler and thinking I had satisfied the requirements, I
ran through, or, perhaps, into the test suite and my many bugs. The first
category, as one might expect:

* arithmetic, logical, and rounding errors in converting between various
  integers and floats. Probably no worse than any other language, really, and
  with experience, the last would probably not be common. For the first two, my
  general predilection to writing pure and total functions as the core of my
  business logic, augmented by unit tests thereof, would have been a shorter
  path to correctness.

I initially wrote my `Region` struct, the core of my business logic, independent
of the concept of channels. This led to my dispatchers having to poll for
tickets, which timed out the test suite due to latency or overwork. The
compiler can't really help you with performance guarantees.

I rewrote the `Region` struct as a sequence of tasks connected by channels,
taken to the extreme — recording observations, computing violations, issuing
tickets, and dispatching tickets. Each tasks holds only its own state, and the
root struct contains only the external channels, which are written to by the
two public functions.

I'm not sure how natural this architecture is for rust, but there's a lot to
like about it. The work is as concurrent as it can be, doesn't contend for locks
within the application, and could execute on multiple threads if the runtime
permits. On the other hand:

* The state existing only in the task's local bindings means it's not readily
  visible for debugging. Perhaps a more robust implementation would have some
  facility to push a diagnostics message through a channel system, or for tasks
  to emit telemetry on intervals or certain events. Delegating business logic to
  pure, unit tested functions would have helped.
* Rust or tokio's automatic closing of channels when the last instance of the
  other end goes out of scope is probably good and useful, but was the cause of
  most of my pernicious errors. Introducing better logging earlier would have
  helped.

On that last, I started with just `eprintln!`, tried `env-logger` finally, then
discovered and fell rapturously in love with the `tracing` crate, which nails
every problem I've ever had with introspecting code with logging and metrics.
Truly a fantastic design, though in fairness, they had decades of bad attempts
at solving this problem to consider.

I did have more duplicated code than I would have liked, particularly in the
independent handlers for unknown, camera, and dispatcher connections, where,
notably, the single heartbeat request may come before or after the type
registration. Sure, that's just bad protocol design, but it is what it is. I
tried a single handler which had an enum for the type state, but the checker was
very concerned about local variables that would necessarily be set for certain
types and not for others — it couldn't see that. Conversely, I couldn't figure
out how to centralize the heartbeat logic from separate handler loops. Maybe it
would have been smart to move that logic into the underlying connection.

I'm pretty happy with the result. It could benefit from a few cleanup rounds to
remove unnecessary cloning, as I'm probably not using references where I should,
and I'd love to introduce a signal for cleanly shutting down the system and see
how that complicates, or clarifies, the design. Having written a few structs
that spawn tasks and expose their behavior via channels or their wrapper
functions now, I can see those being very repetitive and would love to see if
there is a way to express the common bits more declaratively.