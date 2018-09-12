"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../errors");
var AdvancedIterator = /** @class */ (function () {
    function AdvancedIterator(iterator) {
        this.iterator = iterator;
        this.buffer = [undefined, undefined, undefined]; // previous, current, next
        this.bufferIndex = 0;
        this.isDone = false;
        this.nextCount = 0;
        this.advance();
    }
    Object.defineProperty(AdvancedIterator.prototype, "done", {
        get: function () {
            return this.isDone;
        },
        enumerable: true,
        configurable: true
    });
    Object.defineProperty(AdvancedIterator.prototype, "current", {
        get: function () {
            if (this.nextCount === 0)
                throw new errors.InvalidOperationError("Cannot get the current when the iterator has not been advanced.");
            return this.buffer[this.bufferIndex];
        },
        enumerable: true,
        configurable: true
    });
    Object.defineProperty(AdvancedIterator.prototype, "previous", {
        get: function () {
            if (this.nextCount <= 1)
                throw new errors.InvalidOperationError("Cannot get the previous when the iterator has not advanced enough.");
            return this.buffer[(this.bufferIndex + this.buffer.length - 1) % this.buffer.length];
        },
        enumerable: true,
        configurable: true
    });
    Object.defineProperty(AdvancedIterator.prototype, "peek", {
        get: function () {
            if (this.isDone)
                throw new errors.InvalidOperationError("Cannot peek at the end of the iterator.");
            return this.buffer[(this.bufferIndex + 1) % this.buffer.length];
        },
        enumerable: true,
        configurable: true
    });
    AdvancedIterator.prototype.next = function () {
        if (this.done)
            throw new errors.InvalidOperationError("Cannot get the next when at the end of the iterator.");
        var next = this.buffer[this.getNextBufferIndex()];
        this.advance();
        this.nextCount++;
        return next;
    };
    AdvancedIterator.prototype.rest = function () {
        return tslib_1.__generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    if (!!this.done) return [3 /*break*/, 2];
                    return [4 /*yield*/, this.next()];
                case 1:
                    _a.sent();
                    return [3 /*break*/, 0];
                case 2: return [2 /*return*/];
            }
        });
    };
    AdvancedIterator.prototype.advance = function () {
        var next = this.iterator.next();
        this.bufferIndex = this.getNextBufferIndex();
        if (next.done) {
            this.isDone = true;
            return;
        }
        this.buffer[this.getNextBufferIndex()] = next.value;
    };
    AdvancedIterator.prototype.getNextBufferIndex = function () {
        return (this.bufferIndex + 1) % this.buffer.length;
    };
    return AdvancedIterator;
}());
exports.AdvancedIterator = AdvancedIterator;
