"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * Symbol display part.
 */
var SymbolDisplayPart = /** @class */ (function () {
    /** @internal */
    function SymbolDisplayPart(compilerObject) {
        this._compilerObject = compilerObject;
    }
    Object.defineProperty(SymbolDisplayPart.prototype, "compilerObject", {
        /** Gets the compiler symbol display part. */
        get: function () {
            return this._compilerObject;
        },
        enumerable: true,
        configurable: true
    });
    /**
     * Gets the text.
     */
    SymbolDisplayPart.prototype.getText = function () {
        return this.compilerObject.text;
    };
    /**
     * Gets the kind.
     *
     * Examples: "text", "lineBreak"
     */
    SymbolDisplayPart.prototype.getKind = function () {
        return this.compilerObject.kind;
    };
    return SymbolDisplayPart;
}());
exports.SymbolDisplayPart = SymbolDisplayPart;
