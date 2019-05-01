/**
 * @stardazed/streams - implementation of the web streams standard
 * Part of Stardazed
 * (c) 2018-Present by Arthur Langereis - @zenmumbler
 * https://github.com/stardazed/sd-streams
 */

 // extend global PipeOptions interface with signal
declare global {
	interface PipeOptions {
		signal?: AbortSignal;
	}
}

// ---- Stream Types

export declare const ReadableStream: {
	prototype: ReadableStream;
	new(underlyingSource: UnderlyingByteSource, strategy?: { highWaterMark?: number, size?: undefined }): ReadableStream<Uint8Array>;
	new<R = any>(underlyingSource?: UnderlyingSource<R>, strategy?: QueuingStrategy<R>): ReadableStream<R>;
};

export declare const WritableStream: {
	prototype: WritableStream;
	new<W = any>(underlyingSink?: UnderlyingSink<W>, strategy?: QueuingStrategy<W>): WritableStream<W>;
};

export declare const TransformStream: {
	prototype: TransformStream;
	new<I = any, O = any>(transformer?: Transformer<I, O>, writableStrategy?: QueuingStrategy<I>, readableStrategy?: QueuingStrategy<O>): TransformStream<I, O>;
};

// ---- Built-in Strategies

export declare class ByteLengthQueuingStrategy {
	constructor(options: { highWaterMark: number });
	size(chunk: ArrayBufferView): number;
	highWaterMark: number;
}

export declare class CountQueuingStrategy {
	constructor(options: { highWaterMark: number });
	size(): number;
	highWaterMark: number;
}

// ---- Internal helpers for other standards

/**
 * Internal function for use in other web standard implementations.
 * Don't use this unless you are implementing web standards.
 * @private
 */
export function internal_readableStreamTee<T>(stream: ReadableStream<T>, cloneForBranch2: boolean): ReadableStream<T>[];
