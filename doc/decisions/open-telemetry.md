# Open Telemetry

## Status

Accepted

## Context

The [Open Telemetry](https://opentelemetry.io/) project is an effort to put
together standards around how libraries and applications define, gather, and
export telemetry data. This helps encourage compatibility between monitoring
tools, and my hope is that it  will help reduce vendor lock-in and make it
easier to use the right tool for the job.

I would like to support this effort, and think it is a good thing for the
industry.

That said, the purpose of this project is to experiment with different approaches
to monitoring and observability tooling. This is sometimes at odds with the
goals of the Open Telemetry project, which is to agree upon standard ways of
doing things; novel approaches can introduce incompatibility.

## Decision

I decided to try and stick as close as possible to Open Telemetry
specifications, but not attempt to be 100% API or SDK compatible. When talking
about monitoring concepts, the project will try to use the same terminology as
the Open Telemetry specifications. (Measurements, instruments, meters, etc.) 

I will attempt to build APIs and use a data model that is as close as possible, 
deviating only where needed to help explore and iterate quickly on ideas.
It will be viewed as okay to do things that are "quick and dirty" if it helps
test an idea in a more reasonable time frame, but with the intent to
address/reduce incompatibilities over time.

Longer term if any of the ideas explored here seem worthwhile, I will attempt to
work with the Open Telemetry project to incorporate the ideas through their
standardization processes.

## Consequences

What becomes easier or more difficult to do because of this change?

 - It will be difficult to balance compatibility with development velocity.
 - The Metrics API & SDK is quite complicated and will take a significant
   development effort to get anywhere close to the full spec.
 - The APIs and collection infrastructure may be more complicated than they
   need to be early on in the life of the project.
 - Should the ideas prove useful, it may be easier to contribute changes back to
   the Open Telemetry project.
