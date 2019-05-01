@stardazed/streams
==================
This library provides a full implementation of the web streams standard. It has
no dependencies and can be used as a streams replacement in browsers without (full)
support for the streams standard or in Node.

It also provides a full set of TypeScript types for the library as an improvement
over the incomplete typings in the TS standard library.

Installation
------------
```
npm install @stardazed/streams
pnpm install @stardazed/streams
yarn add @stardazed/streams
```

Usage
-----
In a build system or runtime with module support:

```js
// stream types
import { ReadableStream, WriteStream, TransformStream } from "@stardazed/streams";
// built-in strategies
import { ByteLengthQueuingStrategy, CountQueuingStrategy } from "@stardazed/streams";
```

In pre-modular Node:

```js
// stream types
const { ReadableStream, WriteStream, TransformStream } = require("@stardazed/streams");
// built-in strategies
const { ByteLengthQueuingStrategy, CountQueuingStrategy } = require("@stardazed/streams");
```

See the [Web Streams Standard Specification](https://streams.spec.whatwg.org) for
documentation, examples, etc.

Compliance
-----------
This implementation passes all tests (as specified by January 2019) in the
[web platform tests](https://github.com/web-platform-tests/wpt/tree/master/streams)
except for the detached buffer tests as explained below and a few internal name check tests.

This is a good thing, but a number of tests in the suite are aimed mainly at browser engine
internals or ordering of instructions strictly to the letter of the spec.
This implementation may at any point deviate from certain spec tests for legibility or
optimization purposes, but only if it's deemed worthwhile. (Actual browser implementations
already do this as well.)

Limitations
-----------
Although the full streams API is implemented, this library's code lives in the client space
and cannot directly be used with other built-in APIs. This includes calling `getReader` on
the `body` of a `fetch` call, which may either not be implemented at all or return a browser
internal `ReadableStream`. Due to implementation details of streams, you cannot mix and
match the types in this implementation with those provided by the browser.

👉 The [streams fetch adapter](https://www.npmjs.com/package/@stardazed/streams-fetch-adapter) package
can be used to create modified versions of `fetch` and `Response` to work with this or
any other `ReadableStream` implementation.

👉 The [Stardazed streams polyfill](https://www.npmjs.com/package/@stardazed/streams-polyfill)
package provides a full replacement for streams, `fetch` and `Response` as a global polyfill.
Use this if you just want a drop-in, make-it-work version of Stardazed streams.

In addition, while the BYOB variant of `ReadableStream` is implemented, buffers are copied
and not transferred as no browser has implemented detached buffers yet, let alone exposed
them to client-level code.

Copyright
---------
© 2018-Present by Arthur Langereis - [@zenmumbler](https://twitter.com/zenmumbler)

License
-------
MIT
