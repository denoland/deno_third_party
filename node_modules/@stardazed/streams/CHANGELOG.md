# @stardazed/streams changelog

## 3.0.0
_2019-01-16_
* BREAKING: now uses the built-in TypeScript types and no longer exports own types. Now requires TS 3.2 or newer or your
  own set of types, but using new TS is highly recommended.
* Supports the AbortSignal `signal` field in the PipeOptions for ReadableStream's pipeTo and pipeThrough methods to manually
  abort pipe operations.
* Incorporate changes to streams spec up to 2019-01-16

## 2.0.0
_2018-10-15_
* BREAKING: the TypeScript interfaces to the streams and associated types are now parameterized by the incoming/outgoing chunk types.
  This is both a breaking change from earlier versions and also from the TS built-in types.
* No functional changes, the code is equal to that of 1.0.7.

## 1.0.7
_2018-10-01_
* Expose internal stream tee method for linked implementations to use ([#2](https://github.com/stardazed/sd-streams/issues/2))

## 1.0.6
_2018-09-18_
* Fix broken URLs

## 1.0.5
_2018-09-10_
* Incorporate changes to streams spec up to 2018-09-10

## 1.0.4
_2018-07-23_
* Fix bug where public read request objects were marked as internal

## 1.0.3
_2018-07-22_
* Fix potential perf issue in Chrome with enormous queues. ([#1](https://github.com/stardazed/sd-streams/issues/1))
* Incorporate changes to streams spec up to 2018-07-22
* Merge adapter and polyfill into streams repo
* Switch to pnpm for package mgmt and builds

## 1.0.2
_2018-07-01_
* Add UMD output, mapped to main, browser and module entry points remain ESM
* Node is now a supported runtime (>= 7)

## 1.0.1
_2018-06-28_
* Now passes all current web platform tests for streams.
* Fully compliant save for the detached buffers bit, because reality is cold like Irithyll.

## 1.0.0
_2018-06-28_
* Initial release
