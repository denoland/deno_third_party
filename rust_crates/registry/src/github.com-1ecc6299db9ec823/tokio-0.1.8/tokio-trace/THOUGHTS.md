# Thoughts

> if you think about, different contexts are associated with different parts of
> a span tree, basically — i.e. while there’s a request context that spans a
> bunch of execution, there’s also thread and thread-pool contexts that are
> orthogonal, but you may want to trace across either dimension

A single span might not run entirely on a single thread...
