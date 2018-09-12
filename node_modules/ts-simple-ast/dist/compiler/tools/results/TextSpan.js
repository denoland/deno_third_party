"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * Represents a span of text.
 */
var TextSpan = /** @class */ (function () {
    /** @internal */
    function TextSpan(compilerObject) {
        this._compilerObject = compilerObject;
    }
    Object.defineProperty(TextSpan.prototype, "compilerObject", {
        /** Gets the compiler text span. */
        get: function () {
            return this._compilerObject;
        },
        enumerable: true,
        configurable: true
    });
    /**
     * Gets the start.
     */
    TextSpan.prototype.getStart = function () {
        return this.compilerObject.start;
    };
    /**
     * Gets the start + length.
     */
    TextSpan.prototype.getEnd = function () {
        return this.compilerObject.start + this.compilerObject.length;
    };
    /**
     * Gets the length.
     */
    TextSpan.prototype.getLength = function () {
        return this.compilerObject.length;
    };
    return TextSpan;
}());
exports.TextSpan = TextSpan;
