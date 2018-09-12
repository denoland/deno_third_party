"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../../utils");
var TextSpan_1 = require("./TextSpan");
/**
 * Represents a text change.
 */
var TextChange = /** @class */ (function () {
    /** @internal */
    function TextChange(compilerObject) {
        this._compilerObject = compilerObject;
    }
    Object.defineProperty(TextChange.prototype, "compilerObject", {
        /** Gets the compiler text change. */
        get: function () {
            return this._compilerObject;
        },
        enumerable: true,
        configurable: true
    });
    /**
     * Gets the text span.
     */
    TextChange.prototype.getSpan = function () {
        return new TextSpan_1.TextSpan(this.compilerObject.span);
    };
    /**
     * Gets the new text.
     */
    TextChange.prototype.getNewText = function () {
        return this.compilerObject.newText;
    };
    tslib_1.__decorate([
        utils_1.Memoize
    ], TextChange.prototype, "getSpan", null);
    return TextChange;
}());
exports.TextChange = TextChange;
