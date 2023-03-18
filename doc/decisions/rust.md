# Rust

## Status

Retroactive

## Context

What programming language should I use to explore monitoring ideas?

## Decision

I decided to use Rust, for selfish and illogical reasons.

The backstory is that I have been wanting to learn Rust, and I am the type of
person that learns by doing. In order to learn a language I need to actively use
it to try and solve a non-trivial problem. I worked briefly for a startup that I
had hoped would give me the opportunity to do this as part of my regular work,
but that did not work out that way. When I started really working on this
project I figured this was a good opportunity.

My thinking is that this is okay (for now?) because I am currently the only
developer on the project. The only time & productivity I'm burning is my own.
If/when other contributors join the project it might be prudent to revisit this
decision.

## Consequences

What becomes easier or more difficult to do because of this change?

* I will spend more time fighting with a language I don't know that well. This
  is somewhat by design, as it is an explicit goal for me to learn the language
  concepts better.
* I expect lots of friction around ownership and lifetimes. Monitoring is a
  cross-cutting concern, and metrics are often global/static singletons. I've
  heard from   others that Rust makes it difficult to write code in this style,
  by design, as this leads to problems around thread safety and correctness. 
* It might be marginally easier to get others interested in contributing to the
  project, because lots of people are looking for opportunities to try Rust.
